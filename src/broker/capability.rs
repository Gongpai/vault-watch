use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::Path;

use crate::ata::AtaReadCommand;
use crate::scsi::mapping::{MappingAvailability, discover_scsi_generic};
use crate::scsi::sg_uapi::execute_read_only;
use crate::scsi::{
    ReadOnlyCommand, VpdPage, parse_ata_information_vpd_prefix, parse_standard_inquiry,
    parse_supported_vpd_pages,
};
use crate::storage::{Materialization, StorageInventory, StorageKind, StorageNode};

use super::device::{interpret_ata_completion, open_system_probe_device};
use super::{
    AtaCapabilityGrant, AtaIdentifySummary, BrokerAtaExecutionError, BrokerAtaResponse,
    BrokerDeviceGrant, BrokerGeneration, GrantedBackend, broker_generation,
};

const SYSTEM_BLOCK_CLASS_ROOT: &str = "/sys/class/block";
const SCSI_SENSE_LEN: usize = 64;
const SCSI_TIMEOUT_MS: u32 = 5_000;
const STANDARD_INQUIRY_LEN: u8 = 96;
const VPD_ALLOCATION_LEN: u8 = u8::MAX;
const ATA_IDENTIFY_DEVICE_COMMAND: u8 = 0xec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerCapabilityOutcome {
    GrantedAta,
    NotCandidate,
    PartialInventory,
    MissingGeneration,
    NoScsiGeneric,
    AmbiguousScsiGeneric,
    NativeScsiOrAtapi,
    DeviceUnavailable,
    PermissionDenied,
    StaleOrInvalidDevice,
    TransportUnavailable,
    MalformedEvidence,
    WorkerJoin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerCapabilityReport {
    pub grants: Vec<BrokerDeviceGrant>,
    pub outcomes: Vec<BrokerCapabilityOutcome>,
}

/// Performs a fixed, broker-owned capability sequence. No path, CDB, timeout,
/// allocation length, or capability claim is accepted from IPC/configuration.
pub async fn discover_ata_capabilities(inventory: &StorageInventory) -> BrokerCapabilityReport {
    if inventory.partial {
        return BrokerCapabilityReport {
            grants: Vec::new(),
            outcomes: vec![BrokerCapabilityOutcome::PartialInventory],
        };
    }

    let mut grants = Vec::new();
    let mut outcomes = Vec::with_capacity(inventory.nodes.len());
    for node in &inventory.nodes {
        let Some(generation) = candidate_generation(node, &mut outcomes) else {
            continue;
        };
        let mapping = match discover_scsi_generic(Path::new(SYSTEM_BLOCK_CLASS_ROOT), &node.name) {
            Ok(mapping) => mapping,
            Err(_) => {
                outcomes.push(BrokerCapabilityOutcome::AmbiguousScsiGeneric);
                continue;
            }
        };
        match mapping.availability {
            MappingAvailability::NoScsiGenericInterface => {
                outcomes.push(BrokerCapabilityOutcome::NoScsiGeneric);
                continue;
            }
            MappingAvailability::DeviceGone | MappingAvailability::Unreadable => {
                outcomes.push(BrokerCapabilityOutcome::DeviceUnavailable);
                continue;
            }
            MappingAvailability::Complete => {}
        }
        if mapping.entries.is_empty() && mapping.rejected_entries == 0 {
            outcomes.push(BrokerCapabilityOutcome::NoScsiGeneric);
            continue;
        }
        if mapping.rejected_entries != 0 || mapping.entries.len() != 1 {
            outcomes.push(BrokerCapabilityOutcome::AmbiguousScsiGeneric);
            continue;
        }
        let file = match open_system_probe_device(&node.id, generation) {
            Ok(file) => file,
            Err(error) => {
                outcomes.push(classify_io_error(&error));
                continue;
            }
        };
        match tokio::task::spawn_blocking(move || probe_sat_capabilities(file)).await {
            Ok(Ok(capabilities)) => {
                grants.push(BrokerDeviceGrant {
                    node_id: node.id.clone(),
                    generation,
                    backend: GrantedBackend::AtaSat,
                    ata: capabilities,
                });
                outcomes.push(BrokerCapabilityOutcome::GrantedAta);
            }
            Ok(Err(outcome)) => outcomes.push(outcome),
            Err(_) => outcomes.push(BrokerCapabilityOutcome::WorkerJoin),
        }
    }
    BrokerCapabilityReport { grants, outcomes }
}

fn candidate_generation(
    node: &StorageNode,
    outcomes: &mut Vec<BrokerCapabilityOutcome>,
) -> Option<BrokerGeneration> {
    if node.materialization != Materialization::BlockDevice || node.kind != StorageKind::ScsiLike {
        outcomes.push(BrokerCapabilityOutcome::NotCandidate);
        return None;
    }
    match broker_generation(&node.generation) {
        Some(generation) => Some(generation),
        None => {
            outcomes.push(BrokerCapabilityOutcome::MissingGeneration);
            None
        }
    }
}

fn probe_sat_capabilities(file: File) -> Result<AtaCapabilityGrant, BrokerCapabilityOutcome> {
    let inquiry_data = execute_scsi(
        &file,
        ReadOnlyCommand::Inquiry {
            allocation_len: STANDARD_INQUIRY_LEN,
        },
    )?;
    let supported_data = execute_scsi(
        &file,
        ReadOnlyCommand::InquiryVpd {
            page: VpdPage::Supported,
            allocation_len: VPD_ALLOCATION_LEN,
        },
    )?;
    validate_sat_advertisement(&inquiry_data, &supported_data)?;
    let ata_information = execute_scsi(
        &file,
        ReadOnlyCommand::InquiryVpd {
            page: VpdPage::AtaInformation,
            allocation_len: VPD_ALLOCATION_LEN,
        },
    )?;
    validate_sat_evidence(&inquiry_data, &supported_data, &ata_information)?;

    let identify = match execute_ata(&file, AtaReadCommand::IdentifyDevice)? {
        BrokerAtaResponse::Identify(identify) => identify,
        _ => return Err(BrokerCapabilityOutcome::MalformedEvidence),
    };
    Ok(capabilities_from_identify(&file, identify))
}

fn validate_sat_evidence(
    inquiry_data: &[u8],
    supported_data: &[u8],
    ata_information_data: &[u8],
) -> Result<(), BrokerCapabilityOutcome> {
    validate_sat_advertisement(inquiry_data, supported_data)?;
    let ata_information = parse_ata_information_vpd_prefix(ata_information_data)
        .map_err(|_| BrokerCapabilityOutcome::MalformedEvidence)?;
    if ata_information.peripheral_device_type != 0
        || ata_information.command_code != ATA_IDENTIFY_DEVICE_COMMAND
    {
        return Err(BrokerCapabilityOutcome::NativeScsiOrAtapi);
    }
    Ok(())
}

fn validate_sat_advertisement(
    inquiry_data: &[u8],
    supported_data: &[u8],
) -> Result<(), BrokerCapabilityOutcome> {
    let inquiry = parse_standard_inquiry(inquiry_data)
        .map_err(|_| BrokerCapabilityOutcome::MalformedEvidence)?;
    if inquiry.peripheral_device_type != 0 {
        return Err(BrokerCapabilityOutcome::NativeScsiOrAtapi);
    }
    let supported = parse_supported_vpd_pages(supported_data)
        .map_err(|_| BrokerCapabilityOutcome::MalformedEvidence)?;
    if !supported.contains(&(VpdPage::AtaInformation as u8)) {
        return Err(BrokerCapabilityOutcome::NativeScsiOrAtapi);
    }
    Ok(())
}

fn capabilities_from_identify(file: &File, identify: AtaIdentifySummary) -> AtaCapabilityGrant {
    let smart_thresholds = identify.smart_supported
        && matches!(
            execute_ata(file, AtaReadCommand::SmartReadThresholds),
            Ok(BrokerAtaResponse::SmartThresholds(_))
        );
    let gpl = identify.general_purpose_logging_supported
        && matches!(
            execute_ata(file, AtaReadCommand::ReadGplDirectory),
            Ok(BrokerAtaResponse::GplDirectory(_))
        );
    AtaCapabilityGrant {
        smart: identify.smart_supported,
        smart_thresholds,
        gpl,
    }
}

fn execute_scsi(file: &File, command: ReadOnlyCommand) -> Result<Vec<u8>, BrokerCapabilityOutcome> {
    let completion = execute_read_only(
        file.as_raw_fd(),
        &command.cdb(),
        command.allocation_len(),
        SCSI_SENSE_LEN,
        SCSI_TIMEOUT_MS,
    )
    .map_err(|error| classify_io_error(&error))?;
    if completion.host_status != 0 || completion.driver_status != 0 || completion.scsi_status != 0 {
        return Err(BrokerCapabilityOutcome::TransportUnavailable);
    }
    Ok(completion.data)
}

fn execute_ata(
    file: &File,
    command: AtaReadCommand,
) -> Result<BrokerAtaResponse, BrokerCapabilityOutcome> {
    let completion = execute_read_only(
        file.as_raw_fd(),
        &command.cdb(),
        command.data_len(),
        SCSI_SENSE_LEN,
        SCSI_TIMEOUT_MS,
    )
    .map_err(|error| classify_io_error(&error))?;
    interpret_ata_completion(command, completion).map_err(|error| match error {
        BrokerAtaExecutionError::Malformed(_) => BrokerCapabilityOutcome::MalformedEvidence,
        BrokerAtaExecutionError::Io(error) => classify_io_error(&error),
        _ => BrokerCapabilityOutcome::TransportUnavailable,
    })
}

fn classify_io_error(error: &std::io::Error) -> BrokerCapabilityOutcome {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => BrokerCapabilityOutcome::PermissionDenied,
        std::io::ErrorKind::InvalidData => BrokerCapabilityOutcome::StaleOrInvalidDevice,
        std::io::ErrorKind::NotFound => BrokerCapabilityOutcome::DeviceUnavailable,
        _ => BrokerCapabilityOutcome::TransportUnavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageNode;
    use crate::storage::model::Generation;

    fn node(kind: StorageKind, materialization: Materialization) -> StorageNode {
        StorageNode {
            id: "block:sda".to_owned(),
            name: "sda".to_owned(),
            kind,
            materialization,
            removable: Some(false),
            identities: Vec::new(),
            generation: Generation {
                diskseq: Some(42),
                dev_t: Some((8, 0)),
            },
        }
    }

    #[test]
    fn only_whole_scsi_like_nodes_with_complete_generation_are_candidates() {
        let mut outcomes = Vec::new();
        assert_eq!(
            candidate_generation(
                &node(StorageKind::ScsiLike, Materialization::BlockDevice),
                &mut outcomes,
            ),
            Some(BrokerGeneration {
                diskseq: 42,
                dev_major: 8,
                dev_minor: 0,
            })
        );
        assert!(outcomes.is_empty());

        assert_eq!(
            candidate_generation(
                &node(StorageKind::Nvme, Materialization::BlockDevice),
                &mut outcomes,
            ),
            None
        );
        assert_eq!(outcomes, [BrokerCapabilityOutcome::NotCandidate]);
    }

    #[test]
    fn identify_capabilities_do_not_grant_unproven_optional_commands() {
        let file = File::open("/dev/null").unwrap();
        let grant = capabilities_from_identify(
            &file,
            AtaIdentifySummary {
                capacity_bytes: Some(1),
                medium: crate::ata::AtaMedium::SolidState,
                smart_supported: false,
                general_purpose_logging_supported: false,
            },
        );
        assert_eq!(grant, AtaCapabilityGrant::default());
    }

    #[tokio::test]
    async fn partial_inventory_never_probes_or_grants() {
        let report = discover_ata_capabilities(&StorageInventory {
            partial: true,
            ..StorageInventory::default()
        })
        .await;
        assert!(report.grants.is_empty());
        assert_eq!(report.outcomes, [BrokerCapabilityOutcome::PartialInventory]);
    }

    #[test]
    fn io_failures_preserve_permission_and_stale_device_classes() {
        assert_eq!(
            classify_io_error(&std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
            BrokerCapabilityOutcome::PermissionDenied
        );
        assert_eq!(
            classify_io_error(&std::io::Error::from(std::io::ErrorKind::InvalidData)),
            BrokerCapabilityOutcome::StaleOrInvalidDevice
        );
        assert_eq!(
            classify_io_error(&std::io::Error::from(std::io::ErrorKind::NotFound)),
            BrokerCapabilityOutcome::DeviceUnavailable
        );
    }

    #[test]
    fn sat_evidence_requires_direct_access_advertised_page_and_identify_device() {
        let mut inquiry = vec![0u8; usize::from(STANDARD_INQUIRY_LEN)];
        inquiry[4] = 31;
        let supported = [0, VpdPage::Supported as u8, 0, 1, 0x89];
        let mut ata_information = [0u8; 60];
        ata_information[1] = VpdPage::AtaInformation as u8;
        ata_information[2..4].copy_from_slice(&568u16.to_be_bytes());
        ata_information[56] = ATA_IDENTIFY_DEVICE_COMMAND;
        assert_eq!(
            validate_sat_evidence(&inquiry, &supported, &ata_information),
            Ok(())
        );

        let native_scsi_pages = [0, VpdPage::Supported as u8, 0, 1, 0x83];
        assert_eq!(
            validate_sat_evidence(&inquiry, &native_scsi_pages, &ata_information),
            Err(BrokerCapabilityOutcome::NativeScsiOrAtapi)
        );
        ata_information[56] = 0xa1;
        assert_eq!(
            validate_sat_evidence(&inquiry, &supported, &ata_information),
            Err(BrokerCapabilityOutcome::NativeScsiOrAtapi)
        );
        ata_information[3] = 1;
        assert_eq!(
            validate_sat_evidence(&inquiry, &supported, &ata_information),
            Err(BrokerCapabilityOutcome::MalformedEvidence)
        );
    }
}
