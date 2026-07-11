use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::unix::AsyncFd;
use tokio::sync::{Notify, mpsc};

const UEVENT_BUFFER_BYTES: usize = 16 * 1024;
const EVENT_CHANNEL_CAPACITY: usize = 32;
const COALESCE_WINDOW: Duration = Duration::from_millis(150);

struct BlockEventMonitor {
    socket: AsyncFd<OwnedFd>,
}

impl BlockEventMonitor {
    fn open() -> io::Result<Self> {
        // SAFETY: socket() receives fixed Linux netlink constants and returns
        // a new descriptor. Ownership is transferred exactly once to OwnedFd.
        let raw_fd = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_DGRAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                libc::NETLINK_KOBJECT_UEVENT,
            )
        };
        if raw_fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // SAFETY: raw_fd was created successfully above and is not owned by
        // another Rust value. OwnedFd closes it on every following error path.
        let socket = unsafe { OwnedFd::from_raw_fd(raw_fd) };
        // SAFETY: all-zero is a valid initial sockaddr_nl; the required family
        // and multicast group fields are assigned immediately afterwards.
        let mut address: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
        address.nl_family = libc::AF_NETLINK as libc::sa_family_t;
        address.nl_pid = 0;
        address.nl_groups = 1;
        // SAFETY: address points to an initialized sockaddr_nl for the exact
        // length supplied; bind neither retains the pointer nor mutates it.
        let result = unsafe {
            libc::bind(
                socket.as_raw_fd(),
                (&address as *const libc::sockaddr_nl).cast(),
                std::mem::size_of::<libc::sockaddr_nl>() as libc::socklen_t,
            )
        };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            socket: AsyncFd::new(socket)?,
        })
    }

    async fn next_block_event(&self) -> io::Result<()> {
        loop {
            let mut ready = self.socket.readable().await?;
            match ready.try_io(|socket| receive(socket.get_ref().as_raw_fd())) {
                Ok(Ok(payload)) if is_block_event(&payload) => return Ok(()),
                Ok(Ok(_)) => continue,
                Ok(Err(error)) => return Err(error),
                Err(_would_block) => continue,
            }
        }
    }
}

fn receive(fd: RawFd) -> io::Result<Vec<u8>> {
    let mut buffer = vec![0_u8; UEVENT_BUFFER_BYTES];
    // SAFETY: buffer is writable for buffer.len() bytes and recv does not keep
    // its pointer after returning. fd is an open nonblocking netlink socket.
    let received = unsafe {
        libc::recv(
            fd,
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            libc::MSG_DONTWAIT,
        )
    };
    if received < 0 {
        return Err(io::Error::last_os_error());
    }
    buffer.truncate(received as usize);
    Ok(buffer)
}

fn is_block_event(payload: &[u8]) -> bool {
    payload
        .split(|byte| *byte == 0)
        .any(|field| field == b"SUBSYSTEM=block")
}

/// Start an unprivileged, read-only kernel event plane. Events contain no
/// trusted inventory data: they are coalesced and only wake the existing sysfs
/// reconciliation path. Failure to open/read netlink leaves periodic polling
/// as the mandatory correctness fallback.
pub fn spawn_block_event_hints(reconcile: Arc<Notify>) {
    let Ok(monitor) = BlockEventMonitor::open() else {
        return;
    };
    let (sender, receiver) = mpsc::channel(EVENT_CHANNEL_CAPACITY);

    tokio::spawn(async move {
        while monitor.next_block_event().await.is_ok() {
            if sender.send(()).await.is_err() {
                break;
            }
        }
    });

    tokio::spawn(coalesce_events(receiver, reconcile));
}

async fn coalesce_events(mut receiver: mpsc::Receiver<()>, reconcile: Arc<Notify>) {
    while receiver.recv().await.is_some() {
        loop {
            match tokio::time::timeout(COALESCE_WINDOW, receiver.recv()).await {
                Ok(Some(())) => continue,
                Ok(None) => {
                    reconcile.notify_one();
                    return;
                }
                Err(_elapsed) => {
                    reconcile.notify_one();
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::{Notify, mpsc};

    use super::{coalesce_events, is_block_event};

    #[test]
    fn accepts_only_block_subsystem_uevents() {
        assert!(is_block_event(
            b"add@/devices/pci/block/sda\0ACTION=add\0SUBSYSTEM=block\0DEVNAME=sda\0"
        ));
        assert!(!is_block_event(
            b"add@/devices/pci/usb1\0ACTION=add\0SUBSYSTEM=usb\0"
        ));
        assert!(!is_block_event(b"SUBSYSTEM=block-device\0"));
    }

    #[tokio::test]
    async fn burst_is_coalesced_into_one_reconciliation_hint() {
        let (sender, receiver) = mpsc::channel(8);
        let reconcile = Arc::new(Notify::new());
        let worker = tokio::spawn(coalesce_events(receiver, Arc::clone(&reconcile)));

        sender.send(()).await.unwrap();
        sender.send(()).await.unwrap();
        sender.send(()).await.unwrap();
        drop(sender);

        tokio::time::timeout(Duration::from_secs(1), reconcile.notified())
            .await
            .expect("coalesced hint was not emitted");
        assert!(
            tokio::time::timeout(Duration::from_millis(250), reconcile.notified())
                .await
                .is_err()
        );
        worker.await.unwrap();
    }
}
