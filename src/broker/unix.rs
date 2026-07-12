use std::io;
use std::mem::{MaybeUninit, size_of};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

use super::BrokerPeerCredentials;

/// Reads kernel-authenticated credentials for an already connected Unix
/// stream. This does not create, bind, listen on, or connect a socket.
pub fn peer_credentials(stream: &UnixStream) -> io::Result<BrokerPeerCredentials> {
    let mut credentials = MaybeUninit::<libc::ucred>::uninit();
    let mut length = libc::socklen_t::try_from(size_of::<libc::ucred>())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "ucred size overflow"))?;

    // SAFETY: `credentials` points to writable storage for exactly one
    // `libc::ucred`; `length` is initialized to that allocation's size and both
    // pointers remain live for the synchronous getsockopt call. The result is
    // assumed initialized only after success and an exact returned-size check.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            credentials.as_mut_ptr().cast(),
            &mut length,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: getsockopt succeeded and reported that it initialized the buffer;
    // exact size and PID validity are checked by `validated_credentials`.
    let credentials = unsafe { credentials.assume_init() };
    validated_credentials(credentials, length)
}

fn validated_credentials(
    credentials: libc::ucred,
    length: libc::socklen_t,
) -> io::Result<BrokerPeerCredentials> {
    if usize::try_from(length).ok() != Some(size_of::<libc::ucred>()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "kernel returned an unexpected ucred size",
        ));
    }
    let pid = u32::try_from(credentials.pid).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "kernel returned a non-positive peer pid",
        )
    })?;
    if pid == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "kernel returned a zero peer pid",
        ));
    }
    Ok(BrokerPeerCredentials {
        uid: credentials.uid,
        gid: credentials.gid,
        pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::BrokerPeerPolicy;

    #[test]
    fn socket_pair_credentials_are_kernel_derived_and_policy_compatible() {
        let (left, right) = UnixStream::pair().unwrap();
        let (left_peer, right_peer) = match (peer_credentials(&left), peer_credentials(&right)) {
            (Ok(left), Ok(right)) => (left, right),
            (Err(left), Err(right))
                if left.kind() == io::ErrorKind::PermissionDenied
                    && right.kind() == io::ErrorKind::PermissionDenied =>
            {
                return;
            }
            (left, right) => panic!("unexpected SO_PEERCRED results: {left:?}, {right:?}"),
        };
        assert_eq!(left_peer, right_peer);
        assert_eq!(left_peer.uid, unsafe { libc::geteuid() });
        assert_eq!(left_peer.gid, unsafe { libc::getegid() });
        assert_eq!(left_peer.pid, std::process::id());
        assert!(
            BrokerPeerPolicy {
                allowed_uid: left_peer.uid,
                allowed_gid: left_peer.gid,
            }
            .accepts(left_peer)
        );
    }

    #[test]
    fn synthetic_ucred_validation_rejects_bad_size_and_pid() {
        let length = libc::socklen_t::try_from(size_of::<libc::ucred>()).unwrap();
        let credentials = libc::ucred {
            pid: 123,
            uid: 1000,
            gid: 1001,
        };
        assert_eq!(
            validated_credentials(credentials, length).unwrap(),
            BrokerPeerCredentials {
                uid: 1000,
                gid: 1001,
                pid: 123,
            }
        );
        assert_eq!(
            validated_credentials(credentials, length - 1)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            validated_credentials(
                libc::ucred {
                    pid: 0,
                    ..credentials
                },
                length
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::InvalidData
        );
    }
}
