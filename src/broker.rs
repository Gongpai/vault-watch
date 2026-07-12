//! Pure privilege-broker request contract and authorization policy.
//!
//! This module performs no IPC, path opening, privilege changes, CDB handling,
//! or ioctls. Clients can request only fixed high-level monitoring operations.

use std::time::Duration;

use crate::storage::model::Generation;
use crate::storage::{Materialization, StorageInventory, StorageKind};

mod wire;
pub use wire::{
    BROKER_WIRE_VERSION, BrokerPeerCredentials, BrokerPeerPolicy, BrokerSession, BrokerWireError,
    decode_request_frame, encode_request_frame,
};
#[cfg(target_os = "linux")]
mod device;
#[cfg(target_os = "linux")]
pub use device::{
    AtaIdentifySummary, BrokerAtaExecutionError, BrokerAtaResponse, BrokerOpenedDevice,
    open_system_authorized_device,
};
#[cfg(target_os = "linux")]
mod unix;
#[cfg(target_os = "linux")]
pub use unix::{BrokerSocket, peer_credentials};

pub const MAX_DEVICE_ID_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtaBrokerOperation {
    IdentifyDevice,
    SmartReadData,
    SmartReadThresholds,
    SmartReturnStatus,
    ReadGplDirectory,
}

impl AtaBrokerOperation {
    pub const fn timeout(self) -> Duration {
        match self {
            Self::IdentifyDevice | Self::SmartReadData | Self::SmartReadThresholds => {
                Duration::from_secs(5)
            }
            Self::SmartReturnStatus => Duration::from_secs(2),
            Self::ReadGplDirectory => Duration::from_secs(10),
        }
    }

    pub const fn response_limit(self) -> usize {
        match self {
            Self::SmartReturnStatus => 0,
            _ => 512,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerDeviceRef {
    pub node_id: String,
    pub generation: BrokerGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrokerGeneration {
    pub diskseq: u64,
    pub dev_major: u32,
    pub dev_minor: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerRequest {
    pub request_id: u64,
    pub device: BrokerDeviceRef,
    pub operation: AtaBrokerOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantedBackend {
    AtaSat,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AtaCapabilityGrant {
    pub smart: bool,
    pub smart_thresholds: bool,
    pub gpl: bool,
}

/// Broker-owned authorization state. This must be derived from discovery and
/// capability negotiation inside the trusted process, never supplied by IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerDeviceGrant {
    pub node_id: String,
    pub generation: BrokerGeneration,
    pub backend: GrantedBackend,
    pub ata: AtaCapabilityGrant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerDenyReason {
    InvalidRequestId,
    InvalidDeviceId,
    MissingGeneration,
    PartialInventory,
    UnknownDevice,
    DuplicateDevice,
    NotWholeDevice,
    WrongProtocolView,
    GrantDeviceMismatch,
    StaleGeneration,
    CapabilityNotGranted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedBrokerRequest {
    pub(crate) request_id: u64,
    pub(crate) node_id: String,
    pub(crate) generation: BrokerGeneration,
    pub(crate) operation: AtaBrokerOperation,
    pub(crate) timeout: Duration,
    pub(crate) response_limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenedDeviceEvidence {
    pub is_block_device: bool,
    pub is_partition: bool,
    pub opened_read_only: bool,
    pub generation: BrokerGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionDenyReason {
    NotBlockDevice,
    Partition,
    NotReadOnly,
    IdentityChangedAfterOpen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedExecutionPlan {
    pub(crate) request_id: u64,
    pub(crate) node_id: String,
    pub(crate) generation: BrokerGeneration,
    pub(crate) operation: AtaBrokerOperation,
    pub(crate) timeout: Duration,
    pub(crate) response_limit: usize,
}

impl VerifiedExecutionPlan {
    pub const fn request_id(&self) -> u64 {
        self.request_id
    }

    pub const fn operation(&self) -> AtaBrokerOperation {
        self.operation
    }
}

/// Revalidates broker-owned evidence collected from an already opened file
/// descriptor. No path or client-provided evidence is accepted here.
pub fn revalidate_opened_device(
    authorized: &AuthorizedBrokerRequest,
    evidence: OpenedDeviceEvidence,
) -> Result<VerifiedExecutionPlan, ExecutionDenyReason> {
    if !evidence.is_block_device {
        return Err(ExecutionDenyReason::NotBlockDevice);
    }
    if evidence.is_partition {
        return Err(ExecutionDenyReason::Partition);
    }
    if !evidence.opened_read_only {
        return Err(ExecutionDenyReason::NotReadOnly);
    }
    if evidence.generation != authorized.generation {
        return Err(ExecutionDenyReason::IdentityChangedAfterOpen);
    }
    Ok(VerifiedExecutionPlan {
        request_id: authorized.request_id,
        node_id: authorized.node_id.clone(),
        generation: authorized.generation,
        operation: authorized.operation,
        timeout: authorized.timeout,
        response_limit: authorized.response_limit,
    })
}

pub fn authorize_ata_request(
    inventory: &StorageInventory,
    grant: &BrokerDeviceGrant,
    request: &BrokerRequest,
) -> Result<AuthorizedBrokerRequest, BrokerDenyReason> {
    if request.request_id == 0 {
        return Err(BrokerDenyReason::InvalidRequestId);
    }
    if !valid_device_id(&request.device.node_id) {
        return Err(BrokerDenyReason::InvalidDeviceId);
    }
    if inventory.partial {
        return Err(BrokerDenyReason::PartialInventory);
    }

    let mut nodes = inventory
        .nodes
        .iter()
        .filter(|node| node.id == request.device.node_id);
    let node = nodes.next().ok_or(BrokerDenyReason::UnknownDevice)?;
    if nodes.next().is_some() {
        return Err(BrokerDenyReason::DuplicateDevice);
    }
    if node.materialization != Materialization::BlockDevice {
        return Err(BrokerDenyReason::NotWholeDevice);
    }
    if node.kind != StorageKind::ScsiLike {
        return Err(BrokerDenyReason::WrongProtocolView);
    }
    if grant.backend != GrantedBackend::AtaSat
        || grant.node_id != request.device.node_id
        || grant.node_id != node.id
    {
        return Err(BrokerDenyReason::GrantDeviceMismatch);
    }
    let node_generation =
        broker_generation(&node.generation).ok_or(BrokerDenyReason::MissingGeneration)?;
    if request.device.generation != node_generation || grant.generation != node_generation {
        return Err(BrokerDenyReason::StaleGeneration);
    }
    if !operation_granted(request.operation, grant.ata) {
        return Err(BrokerDenyReason::CapabilityNotGranted);
    }

    Ok(AuthorizedBrokerRequest {
        request_id: request.request_id,
        node_id: node.id.clone(),
        generation: node_generation,
        operation: request.operation,
        timeout: request.operation.timeout(),
        response_limit: request.operation.response_limit(),
    })
}

const fn broker_generation(generation: &Generation) -> Option<BrokerGeneration> {
    match (generation.diskseq, generation.dev_t) {
        (Some(diskseq), Some((dev_major, dev_minor))) => Some(BrokerGeneration {
            diskseq,
            dev_major,
            dev_minor,
        }),
        _ => None,
    }
}

fn valid_device_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_DEVICE_ID_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-' | b'.'))
}

const fn operation_granted(operation: AtaBrokerOperation, grant: AtaCapabilityGrant) -> bool {
    match operation {
        AtaBrokerOperation::IdentifyDevice => true,
        AtaBrokerOperation::SmartReadData | AtaBrokerOperation::SmartReturnStatus => grant.smart,
        AtaBrokerOperation::SmartReadThresholds => grant.smart && grant.smart_thresholds,
        AtaBrokerOperation::ReadGplDirectory => grant.gpl,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageNode;

    fn generation() -> BrokerGeneration {
        BrokerGeneration {
            diskseq: 42,
            dev_major: 8,
            dev_minor: 0,
        }
    }

    fn storage_generation() -> Generation {
        Generation {
            diskseq: Some(42),
            dev_t: Some((8, 0)),
        }
    }

    fn inventory(materialization: Materialization) -> StorageInventory {
        StorageInventory {
            nodes: vec![StorageNode {
                id: "block:sda".to_owned(),
                name: "sda".to_owned(),
                kind: StorageKind::ScsiLike,
                materialization,
                removable: Some(false),
                identities: Vec::new(),
                generation: storage_generation(),
            }],
            edges: Vec::new(),
            partial: false,
        }
    }

    fn grant() -> BrokerDeviceGrant {
        BrokerDeviceGrant {
            node_id: "block:sda".to_owned(),
            generation: generation(),
            backend: GrantedBackend::AtaSat,
            ata: AtaCapabilityGrant {
                smart: true,
                smart_thresholds: false,
                gpl: false,
            },
        }
    }

    fn request(operation: AtaBrokerOperation) -> BrokerRequest {
        BrokerRequest {
            request_id: 7,
            device: BrokerDeviceRef {
                node_id: "block:sda".to_owned(),
                generation: generation(),
            },
            operation,
        }
    }

    #[test]
    fn authorized_plan_uses_fixed_limits_not_client_controlled_fields() {
        let plan = authorize_ata_request(
            &inventory(Materialization::BlockDevice),
            &grant(),
            &request(AtaBrokerOperation::SmartReadData),
        )
        .unwrap();
        assert_eq!(plan.timeout, Duration::from_secs(5));
        assert_eq!(plan.response_limit, 512);
        assert_eq!(plan.node_id, "block:sda");
    }

    #[test]
    fn partition_stale_generation_and_path_like_ids_are_denied() {
        assert_eq!(
            authorize_ata_request(
                &inventory(Materialization::Partition),
                &grant(),
                &request(AtaBrokerOperation::IdentifyDevice),
            ),
            Err(BrokerDenyReason::NotWholeDevice)
        );

        let mut stale = request(AtaBrokerOperation::IdentifyDevice);
        stale.device.generation.diskseq = 41;
        assert_eq!(
            authorize_ata_request(&inventory(Materialization::BlockDevice), &grant(), &stale),
            Err(BrokerDenyReason::StaleGeneration)
        );

        let mut path = request(AtaBrokerOperation::IdentifyDevice);
        path.device.node_id = "/dev/sda".to_owned();
        assert_eq!(
            authorize_ata_request(&inventory(Materialization::BlockDevice), &grant(), &path),
            Err(BrokerDenyReason::InvalidDeviceId)
        );
    }

    #[test]
    fn capability_claims_cannot_be_smuggled_in_a_request() {
        assert_eq!(
            authorize_ata_request(
                &inventory(Materialization::BlockDevice),
                &grant(),
                &request(AtaBrokerOperation::ReadGplDirectory),
            ),
            Err(BrokerDenyReason::CapabilityNotGranted)
        );
        assert_eq!(
            authorize_ata_request(
                &inventory(Materialization::BlockDevice),
                &grant(),
                &request(AtaBrokerOperation::SmartReadThresholds),
            ),
            Err(BrokerDenyReason::CapabilityNotGranted)
        );
    }

    #[test]
    fn missing_generation_and_wrong_protocol_are_denied() {
        let missing = request(AtaBrokerOperation::IdentifyDevice);
        let mut missing_inventory = inventory(Materialization::BlockDevice);
        missing_inventory.nodes[0].generation.diskseq = None;
        assert_eq!(
            authorize_ata_request(&missing_inventory, &grant(), &missing),
            Err(BrokerDenyReason::MissingGeneration)
        );

        let mut wrong = inventory(Materialization::BlockDevice);
        wrong.nodes[0].kind = StorageKind::Nvme;
        assert_eq!(
            authorize_ata_request(
                &wrong,
                &grant(),
                &request(AtaBrokerOperation::IdentifyDevice),
            ),
            Err(BrokerDenyReason::WrongProtocolView)
        );

        let mut partial = inventory(Materialization::BlockDevice);
        partial.partial = true;
        assert_eq!(
            authorize_ata_request(
                &partial,
                &grant(),
                &request(AtaBrokerOperation::IdentifyDevice),
            ),
            Err(BrokerDenyReason::PartialInventory)
        );
    }

    #[test]
    fn post_open_revalidation_denies_partition_write_access_and_generation_change() {
        let authorized = authorize_ata_request(
            &inventory(Materialization::BlockDevice),
            &grant(),
            &request(AtaBrokerOperation::SmartReadData),
        )
        .unwrap();
        let valid = OpenedDeviceEvidence {
            is_block_device: true,
            is_partition: false,
            opened_read_only: true,
            generation: generation(),
        };
        assert!(revalidate_opened_device(&authorized, valid).is_ok());
        assert_eq!(
            revalidate_opened_device(
                &authorized,
                OpenedDeviceEvidence {
                    is_partition: true,
                    ..valid
                },
            ),
            Err(ExecutionDenyReason::Partition)
        );
        assert_eq!(
            revalidate_opened_device(
                &authorized,
                OpenedDeviceEvidence {
                    opened_read_only: false,
                    ..valid
                },
            ),
            Err(ExecutionDenyReason::NotReadOnly)
        );
        assert_eq!(
            revalidate_opened_device(
                &authorized,
                OpenedDeviceEvidence {
                    generation: BrokerGeneration {
                        diskseq: 43,
                        ..generation()
                    },
                    ..valid
                },
            ),
            Err(ExecutionDenyReason::IdentityChangedAfterOpen)
        );
    }
}
