use std::io;
use std::mem::{MaybeUninit, size_of};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use super::{BrokerPeerCredentials, BrokerPeerPolicy};

const BROKER_SOCKET_MODE: u32 = 0o660;

#[derive(Debug)]
pub struct BrokerSocket {
    listener: UnixListener,
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl BrokerSocket {
    /// Binds a broker socket below an existing broker-owned directory. Existing
    /// filesystem entries are never removed or replaced.
    pub fn bind(path: &Path) -> io::Result<Self> {
        validate_socket_path(path)?;
        let listener = UnixListener::bind(path)?;
        let metadata = std::fs::symlink_metadata(path)?;
        if !metadata.file_type().is_socket() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "bound broker endpoint is not a socket",
            ));
        }
        let device = metadata.dev();
        let inode = metadata.ino();
        if let Err(error) =
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(BROKER_SOCKET_MODE))
        {
            remove_if_same_socket(path, device, inode);
            return Err(error);
        }
        let secured = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => {
                remove_if_same_socket(path, device, inode);
                return Err(error);
            }
        };
        if !secured.file_type().is_socket()
            || secured.dev() != device
            || secured.ino() != inode
            || secured.mode() & 0o777 != BROKER_SOCKET_MODE
        {
            remove_if_same_socket(path, device, inode);
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "broker socket identity or permissions changed during bind",
            ));
        }
        Ok(Self {
            listener,
            path: path.to_owned(),
            device,
            inode,
        })
    }

    pub const fn listener(&self) -> &UnixListener {
        &self.listener
    }

    /// Accepts one connection and obtains credentials exclusively from the
    /// kernel. Authorization remains the caller's explicit next step.
    pub fn accept(&self) -> io::Result<(UnixStream, BrokerPeerCredentials)> {
        let (stream, _) = self.listener.accept()?;
        let credentials = peer_credentials(&stream)?;
        Ok((stream, credentials))
    }

    pub(super) fn permits_peer_policy(&self, policy: BrokerPeerPolicy) -> io::Result<bool> {
        let metadata = std::fs::symlink_metadata(&self.path)?;
        if !metadata.file_type().is_socket()
            || metadata.dev() != self.device
            || metadata.ino() != self.inode
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "broker socket identity changed",
            ));
        }
        if policy.allowed_uid == 0 {
            return Ok(true);
        }
        let mode = metadata.mode();
        Ok(
            (metadata.uid() == policy.allowed_uid && mode & 0o600 == 0o600)
                || (metadata.gid() == policy.allowed_gid && mode & 0o060 == 0o060),
        )
    }
}

impl Drop for BrokerSocket {
    fn drop(&mut self) {
        remove_if_same_socket(&self.path, self.device, self.inode);
    }
}

fn remove_if_same_socket(path: &Path, device: u64, inode: u64) {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return;
    };
    if metadata.file_type().is_socket() && metadata.dev() == device && metadata.ino() == inode {
        let _ = std::fs::remove_file(path);
    }
}

fn validate_socket_path(path: &Path) -> io::Result<()> {
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "broker socket path must be an absolute file path",
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "broker socket has no parent")
    })?;
    let metadata = std::fs::symlink_metadata(parent)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "broker socket parent must be a real directory",
        ));
    }
    if parent.canonicalize()? != parent {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "broker socket parent must not traverse aliases or symlinks",
        ));
    }
    // SAFETY: geteuid has no preconditions and does not dereference memory.
    let effective_uid = unsafe { libc::geteuid() };
    if metadata.uid() != effective_uid || metadata.mode() & 0o022 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "broker socket parent ownership or write permissions are unsafe",
        ));
    }
    if std::fs::symlink_metadata(path).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "broker socket path already exists",
        ));
    }
    Ok(())
}

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
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn private_test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("vault-watch-broker-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path.canonicalize().unwrap()
    }

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

    #[test]
    fn lifecycle_sets_restricted_mode_refuses_replacement_and_cleans_up() {
        let directory = private_test_directory();
        let path = directory.join("broker.sock");
        {
            let socket = match BrokerSocket::bind(&path) {
                Ok(socket) => socket,
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                    fs::remove_dir(directory).unwrap();
                    return;
                }
                Err(error) => panic!("unexpected broker bind error: {error}"),
            };
            assert!(
                socket
                    .listener()
                    .local_addr()
                    .unwrap()
                    .as_pathname()
                    .is_some()
            );
            let metadata = fs::symlink_metadata(&path).unwrap();
            assert!(metadata.file_type().is_socket());
            assert_eq!(metadata.mode() & 0o777, BROKER_SOCKET_MODE);
            assert!(
                socket
                    .permits_peer_policy(BrokerPeerPolicy {
                        allowed_uid: metadata.uid(),
                        allowed_gid: metadata.gid(),
                    })
                    .unwrap()
            );
            assert!(
                !socket
                    .permits_peer_policy(BrokerPeerPolicy {
                        allowed_uid: metadata.uid().saturating_add(1),
                        allowed_gid: metadata.gid().saturating_add(1),
                    })
                    .unwrap()
            );
            assert_eq!(
                BrokerSocket::bind(&path).unwrap_err().kind(),
                io::ErrorKind::AlreadyExists
            );
        }
        assert!(!path.exists());
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn lifecycle_rejects_relative_and_unsafe_parent_paths() {
        assert_eq!(
            BrokerSocket::bind(Path::new("broker.sock"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        let directory = private_test_directory();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o722)).unwrap();
        assert_eq!(
            BrokerSocket::bind(&directory.join("broker.sock"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        fs::remove_dir(directory).unwrap();
    }
}
