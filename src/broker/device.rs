use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use tokio::sync::Semaphore;

use crate::ata::{
    AtaLogDirectory, AtaMedium, AtaParseError, AtaReadCommand, SmartAttribute, SmartStatus,
    parse_ata_return_descriptor, parse_identify_device, parse_log_directory,
    parse_smart_attributes, parse_smart_thresholds, smart_return_status,
};
use crate::scsi::sg_uapi::{SgIoCompletion, execute_read_only};

use super::{
    AtaBrokerOperation, AuthorizedBrokerRequest, BrokerGeneration, OpenedDeviceEvidence,
    VerifiedExecutionPlan, revalidate_opened_device,
};

const SYSTEM_DEV_ROOT: &str = "/dev";
const SYSTEM_SYS_DEV_BLOCK_ROOT: &str = "/sys/dev/block";
const ATA_SENSE_LEN: usize = 64;
const ATA_STATUS_ERROR_MASK: u8 = 0xa1;
const MAX_CONCURRENT_ATA_COMMANDS: usize = 4;
static ATA_EXECUTION_PERMITS: OnceLock<Arc<Semaphore>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtaIdentifySummary {
    pub capacity_bytes: Option<u128>,
    pub medium: AtaMedium,
    pub smart_supported: bool,
    pub general_purpose_logging_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerAtaResponse {
    Identify(AtaIdentifySummary),
    SmartData(Vec<SmartAttribute>),
    SmartThresholds(Vec<(u8, u8)>),
    SmartStatus(SmartStatus),
    GplDirectory(AtaLogDirectory),
}

#[derive(Debug)]
pub enum BrokerAtaExecutionError {
    Io(io::Error),
    InvalidPlan,
    TransportStatus { scsi: u8, host: u16, driver: u16 },
    AtaStatus { status: u8, error: u8 },
    Malformed(AtaParseError),
    WorkerClosed,
    WorkerJoin,
}

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

    /// Executes the sealed operation away from the async runtime under a
    /// process-wide concurrency bound.
    pub async fn execute_ata(self) -> Result<BrokerAtaResponse, BrokerAtaExecutionError> {
        let permits = ATA_EXECUTION_PERMITS
            .get_or_init(|| Arc::new(Semaphore::new(MAX_CONCURRENT_ATA_COMMANDS)))
            .clone();
        let permit = permits
            .acquire_owned()
            .await
            .map_err(|_| BrokerAtaExecutionError::WorkerClosed)?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            self.execute_ata_blocking()
        })
        .await
        .map_err(|_| BrokerAtaExecutionError::WorkerJoin)?
    }

    fn execute_ata_blocking(&self) -> Result<BrokerAtaResponse, BrokerAtaExecutionError> {
        let command = match self.plan.operation {
            AtaBrokerOperation::IdentifyDevice => AtaReadCommand::IdentifyDevice,
            AtaBrokerOperation::SmartReadData => AtaReadCommand::SmartReadData,
            AtaBrokerOperation::SmartReadThresholds => AtaReadCommand::SmartReadThresholds,
            AtaBrokerOperation::SmartReturnStatus => AtaReadCommand::SmartReturnStatus,
            AtaBrokerOperation::ReadGplDirectory => AtaReadCommand::ReadGplDirectory,
        };
        let data_len = command.data_len();
        if data_len != self.plan.response_limit {
            return Err(BrokerAtaExecutionError::InvalidPlan);
        }
        let timeout_ms = u32::try_from(self.plan.timeout.as_millis())
            .ok()
            .filter(|timeout| *timeout > 0)
            .ok_or(BrokerAtaExecutionError::InvalidPlan)?;
        let completion = execute_read_only(
            self.file.as_raw_fd(),
            &command.cdb(),
            data_len,
            ATA_SENSE_LEN,
            timeout_ms,
        )
        .map_err(BrokerAtaExecutionError::Io)?;
        interpret_ata_completion(command, completion)
    }
}

pub(super) fn interpret_ata_completion(
    command: AtaReadCommand,
    completion: SgIoCompletion,
) -> Result<BrokerAtaResponse, BrokerAtaExecutionError> {
    if completion.host_status != 0
        || completion.driver_status != 0
        || !matches!(completion.scsi_status, 0x00 | 0x02)
    {
        return Err(BrokerAtaExecutionError::TransportStatus {
            scsi: completion.scsi_status,
            host: completion.host_status,
            driver: completion.driver_status,
        });
    }
    let registers = parse_ata_return_descriptor(&completion.sense)
        .map_err(BrokerAtaExecutionError::Malformed)?;
    if registers.status & ATA_STATUS_ERROR_MASK != 0 {
        return Err(BrokerAtaExecutionError::AtaStatus {
            status: registers.status,
            error: registers.error,
        });
    }
    match command {
        AtaReadCommand::IdentifyDevice => {
            let identify = parse_identify_device(&completion.data)
                .map_err(BrokerAtaExecutionError::Malformed)?;
            Ok(BrokerAtaResponse::Identify(AtaIdentifySummary {
                capacity_bytes: identify.capacity_bytes,
                medium: identify.medium,
                smart_supported: identify.smart_supported,
                general_purpose_logging_supported: identify.general_purpose_logging_supported,
            }))
        }
        AtaReadCommand::SmartReadData => parse_smart_attributes(&completion.data)
            .map(BrokerAtaResponse::SmartData)
            .map_err(BrokerAtaExecutionError::Malformed),
        AtaReadCommand::SmartReadThresholds => parse_smart_thresholds(&completion.data)
            .map(BrokerAtaResponse::SmartThresholds)
            .map_err(BrokerAtaExecutionError::Malformed),
        AtaReadCommand::SmartReturnStatus => Ok(BrokerAtaResponse::SmartStatus(
            smart_return_status(registers),
        )),
        AtaReadCommand::ReadGplDirectory => parse_log_directory(&completion.data)
            .map(BrokerAtaResponse::GplDirectory)
            .map_err(BrokerAtaExecutionError::Malformed),
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
    let (file, evidence) =
        open_revalidated_file(&authorized.node_id, dev_root, sys_dev_block_root)?;
    let plan = revalidate_opened_device(authorized, evidence).map_err(|reason| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("opened device failed broker revalidation: {reason:?}"),
        )
    })?;
    Ok(BrokerOpenedDevice { file, plan })
}

pub(super) fn open_system_probe_device(
    node_id: &str,
    generation: BrokerGeneration,
) -> io::Result<File> {
    let (file, evidence) = open_revalidated_file(
        node_id,
        Path::new(SYSTEM_DEV_ROOT),
        Path::new(SYSTEM_SYS_DEV_BLOCK_ROOT),
    )?;
    if !evidence.is_block_device
        || evidence.is_partition
        || !evidence.opened_read_only
        || evidence.generation != generation
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "capability probe device failed generation or whole-device revalidation",
        ));
    }
    Ok(file)
}

fn open_revalidated_file(
    node_id: &str,
    dev_root: &Path,
    sys_dev_block_root: &Path,
) -> io::Result<(File, OpenedDeviceEvidence)> {
    let device_name = node_id.strip_prefix("block:").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "broker node is not a canonical block node",
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
    Ok((file, evidence))
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

    fn completion(command: AtaReadCommand) -> SgIoCompletion {
        let mut sense = vec![0u8; 22];
        sense[0] = 0x72;
        sense[7] = 14;
        sense[8] = 0x09;
        sense[9] = 0x0c;
        sense[17] = 0x4f;
        sense[19] = 0xc2;
        sense[21] = 0x50;
        SgIoCompletion {
            data: vec![0; command.data_len()],
            sense,
            scsi_status: 0x02,
            host_status: 0,
            driver_status: 0,
            residual: 0,
            duration_ms: 1,
            info: 1,
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
        assert_eq!(opened.plan().request_id(), 1);
    }

    #[test]
    fn typed_completion_interpretation_requires_transport_and_ata_success() {
        let response = interpret_ata_completion(
            AtaReadCommand::IdentifyDevice,
            completion(AtaReadCommand::IdentifyDevice),
        )
        .unwrap();
        assert!(matches!(response, BrokerAtaResponse::Identify(_)));

        let status = interpret_ata_completion(
            AtaReadCommand::SmartReturnStatus,
            completion(AtaReadCommand::SmartReturnStatus),
        )
        .unwrap();
        assert_eq!(status, BrokerAtaResponse::SmartStatus(SmartStatus::Passed));

        let mut transport = completion(AtaReadCommand::IdentifyDevice);
        transport.host_status = 1;
        assert!(matches!(
            interpret_ata_completion(AtaReadCommand::IdentifyDevice, transport),
            Err(BrokerAtaExecutionError::TransportStatus { .. })
        ));

        let mut ata_error = completion(AtaReadCommand::IdentifyDevice);
        ata_error.sense[21] = 0x51;
        assert!(matches!(
            interpret_ata_completion(AtaReadCommand::IdentifyDevice, ata_error),
            Err(BrokerAtaExecutionError::AtaStatus { .. })
        ));

        let mut missing = completion(AtaReadCommand::IdentifyDevice);
        missing.sense.clear();
        assert!(matches!(
            interpret_ata_completion(AtaReadCommand::IdentifyDevice, missing),
            Err(BrokerAtaExecutionError::Malformed(
                AtaParseError::TruncatedDescriptor
            ))
        ));
    }
}
