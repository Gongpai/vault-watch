use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::Path;

use super::{
    AuthorizedBrokerRequest, BrokerGeneration, OpenedDeviceEvidence, VerifiedExecutionPlan,
    revalidate_opened_device,
};

const SYSTEM_DEV_ROOT: &str = "/dev";
const SYSTEM_SYS_DEV_BLOCK_ROOT: &str = "/sys/dev/block";

/// Owns a read-only whole-device descriptor after generation revalidation.
/// The descriptor is intentionally not exposed through the public API.
#[derive(Debug)]
pub struct BrokerOpenedDevice {
    file: File,
    plan: VerifiedExecutionPlan,
}

impl BrokerOpenedDevice {
    pub fn plan(&self) -> &VerifiedExecutionPlan {
        // Keep descriptor ownership observable without exposing a raw handle to
        // frontend callers. The future in-module executor will consume it.
        let _ = self.file.as_raw_fd();
        &self.plan
    }
}

/// Opens only the broker-authorized whole-device name below `/dev`, then
/// revalidates descriptor type, access mode, dev_t, partition state, and
/// diskseq before returning an executor-owned descriptor.
pub fn open_system_authorized_device(
    authorized: &AuthorizedBrokerRequest,
) -> io::Result<BrokerOpenedDevice> {
    open_authorized_device_at(
        authorized,
        Path::new(SYSTEM_DEV_ROOT),
        Path::new(SYSTEM_SYS_DEV_BLOCK_ROOT),
    )
}

fn open_authorized_device_at(
    authorized: &AuthorizedBrokerRequest,
    dev_root: &Path,
    sys_dev_block_root: &Path,
) -> io::Result<BrokerOpenedDevice> {
    let device_name = authorized.node_id.strip_prefix("block:").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "authorized node is not a canonical block node",
        )
    })?;
    if device_name.is_empty()
        || !device_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "authorized block name is not safe for broker opening",
        ));
    }

    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(dev_root.join(device_name))?;
    let metadata = file.metadata()?;
    let flags = descriptor_flags(file.as_raw_fd())?;
    let dev_major = libc::major(metadata.rdev());
    let dev_minor = libc::minor(metadata.rdev());
    let sys_device = sys_dev_block_root.join(format!("{dev_major}:{dev_minor}"));
    let diskseq = read_diskseq(&sys_device.join("diskseq"))?;
    let evidence = OpenedDeviceEvidence {
        is_block_device: metadata.file_type().is_block_device(),
        is_partition: sys_device.join("partition").try_exists()?,
        opened_read_only: flags & libc::O_ACCMODE == libc::O_RDONLY,
        generation: BrokerGeneration {
            diskseq,
            dev_major,
            dev_minor,
        },
    };
    let plan = revalidate_opened_device(authorized, evidence).map_err(|reason| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("opened device failed broker revalidation: {reason:?}"),
        )
    })?;
    Ok(BrokerOpenedDevice { file, plan })
}

fn descriptor_flags(fd: RawFd) -> io::Result<libc::c_int> {
    // SAFETY: F_GETFL reads flags from a live descriptor and takes no pointer.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(flags)
}

fn read_diskseq(path: &Path) -> io::Result<u64> {
    let value = std::fs::read_to_string(path)?;
    let diskseq = value
        .trim()
        .parse::<u64>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid sysfs diskseq value"))?;
    if diskseq == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "zero sysfs diskseq is not a valid generation",
        ));
    }
    Ok(diskseq)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::AtaBrokerOperation;
    use std::fs;
    use std::time::Duration;

    fn authorized(node_id: &str) -> AuthorizedBrokerRequest {
        AuthorizedBrokerRequest {
            request_id: 1,
            node_id: node_id.to_owned(),
            generation: BrokerGeneration {
                diskseq: 42,
                dev_major: 8,
                dev_minor: 0,
            },
            operation: AtaBrokerOperation::IdentifyDevice,
            timeout: Duration::from_secs(5),
            response_limit: 512,
        }
    }

    #[test]
    fn unsafe_or_noncanonical_authorized_names_never_reach_open() {
        for node_id in ["sda", "block:", "block:../sda", "block:sda/child"] {
            assert_eq!(
                open_authorized_device_at(
                    &authorized(node_id),
                    Path::new("/definitely/missing"),
                    Path::new("/definitely/missing")
                )
                .unwrap_err()
                .kind(),
                io::ErrorKind::InvalidInput
            );
        }
    }

    #[test]
    fn ordinary_files_are_denied_before_becoming_execution_descriptors() {
        let root =
            std::env::temp_dir().join(format!("vault-watch-device-open-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dev = root.join("dev");
        let sys = root.join("sys");
        fs::create_dir_all(&dev).unwrap();
        fs::create_dir_all(sys.join("0:0")).unwrap();
        fs::write(dev.join("sda"), b"not a block device").unwrap();
        fs::write(sys.join("0:0/diskseq"), b"42\n").unwrap();

        assert_eq!(
            open_authorized_device_at(&authorized("block:sda"), &dev, &sys)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn diskseq_parser_rejects_zero_malformed_and_missing_values() {
        let root = std::env::temp_dir().join(format!("vault-watch-diskseq-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let path = root.join("diskseq");
        fs::write(&path, b"42\n").unwrap();
        assert_eq!(read_diskseq(&path).unwrap(), 42);
        fs::write(&path, b"0\n").unwrap();
        assert_eq!(
            read_diskseq(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::write(&path, b"not-a-number\n").unwrap();
        assert_eq!(
            read_diskseq(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn opened_descriptor_api_exposes_plan_but_not_device_contents() {
        let file = File::open("/dev/null").unwrap();
        let opened = BrokerOpenedDevice {
            file,
            plan: VerifiedExecutionPlan {
                request_id: 1,
                node_id: "block:sda".to_owned(),
                generation: authorized("block:sda").generation,
                operation: AtaBrokerOperation::IdentifyDevice,
                timeout: Duration::from_secs(5),
                response_limit: 512,
            },
        };
        assert_eq!(opened.plan().request_id, 1);
    }
}
