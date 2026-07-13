use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

use vault_watch::ata::SmartStatus;
use vault_watch::broker::{
    AtaBrokerOperation, BrokerAtaResponse, BrokerClient, BrokerClientError, BrokerDeviceRef,
    BrokerGeneration, BrokerPeerPolicy, BrokerResponseError, DEFAULT_BROKER_SOCKET_PATH,
};

use crate::app::{DiskInfo, HealthStatus, MetricAvailability};
use crate::storage::{Materialization, StorageInventory, StorageKind};

const RECONNECT_BACKOFF: Duration = Duration::from_secs(30);

#[derive(Debug, Default)]
pub struct BrokerHealthSnapshot {
    pub disks: Vec<DiskInfo>,
    pub handled_devices: HashSet<String>,
    pub connected: bool,
}

#[derive(Debug, Default)]
pub struct BrokerHealthCollector {
    client: Option<BrokerClient>,
    next_connect_at: Option<Instant>,
}

impl BrokerHealthCollector {
    pub async fn collect(&mut self, inventory: &StorageInventory) -> BrokerHealthSnapshot {
        if inventory.partial {
            return BrokerHealthSnapshot::default();
        }
        let subjects = broker_subjects(inventory);
        if subjects.is_empty() {
            return BrokerHealthSnapshot::default();
        }
        if self.client.is_none() {
            if self
                .next_connect_at
                .is_some_and(|deadline| Instant::now() < deadline)
            {
                return BrokerHealthSnapshot::default();
            }
            match BrokerClient::connect(
                Path::new(DEFAULT_BROKER_SOCKET_PATH),
                BrokerPeerPolicy {
                    allowed_uid: 0,
                    allowed_gid: 0,
                },
            )
            .await
            {
                Ok(client) => {
                    self.client = Some(client);
                    self.next_connect_at = None;
                }
                Err(_) => {
                    self.next_connect_at = Some(Instant::now() + RECONNECT_BACKOFF);
                }
            }
        }
        let Some(client) = self.client.as_mut() else {
            return BrokerHealthSnapshot::default();
        };

        let mut snapshot = BrokerHealthSnapshot {
            connected: true,
            ..BrokerHealthSnapshot::default()
        };
        let mut reset_connection = false;
        for subject in subjects {
            match client
                .execute(subject.device_ref, AtaBrokerOperation::SmartReturnStatus)
                .await
            {
                Ok(BrokerAtaResponse::SmartStatus(status)) => {
                    snapshot
                        .disks
                        .push(disk_from_status(subject.device.clone(), status));
                    snapshot.handled_devices.insert(subject.device);
                }
                Ok(_) => {
                    reset_connection = true;
                    snapshot.connected = false;
                    break;
                }
                Err(error) if connection_is_unusable(error) => {
                    reset_connection = true;
                    snapshot.connected = false;
                    break;
                }
                Err(_) => {}
            }
        }
        if reset_connection {
            self.client = None;
            self.next_connect_at = Some(Instant::now() + RECONNECT_BACKOFF);
        }
        snapshot
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BrokerHealthSubject {
    device: String,
    device_ref: BrokerDeviceRef,
}

fn broker_subjects(inventory: &StorageInventory) -> Vec<BrokerHealthSubject> {
    inventory
        .nodes
        .iter()
        .filter(|node| {
            node.kind == StorageKind::ScsiLike
                && node.materialization == Materialization::BlockDevice
        })
        .filter_map(|node| {
            let (Some(diskseq), Some((dev_major, dev_minor))) =
                (node.generation.diskseq, node.generation.dev_t)
            else {
                return None;
            };
            Some(BrokerHealthSubject {
                device: node.name.clone(),
                device_ref: BrokerDeviceRef {
                    node_id: node.id.clone(),
                    generation: BrokerGeneration {
                        diskseq,
                        dev_major,
                        dev_minor,
                    },
                },
            })
        })
        .collect()
}

fn disk_from_status(device: String, status: SmartStatus) -> DiskInfo {
    let mut disk = DiskInfo::unavailable(device);
    match status {
        SmartStatus::Passed => {
            disk.health = HealthStatus::Healthy;
            disk.health_availability = MetricAvailability::Available;
        }
        SmartStatus::PredictingFailure => {
            disk.health = HealthStatus::Failed;
            disk.health_availability = MetricAvailability::Available;
        }
        SmartStatus::Unknown => {
            disk.health_availability = MetricAvailability::Malformed;
        }
    }
    disk
}

const fn connection_is_unusable(error: BrokerClientError) -> bool {
    matches!(
        error,
        BrokerClientError::Connect(_)
            | BrokerClientError::UnauthorizedServer
            | BrokerClientError::RequestIdExhausted
            | BrokerClientError::Transport(_)
            | BrokerClientError::TimedOut
            | BrokerClientError::InvalidResponse(_)
            | BrokerClientError::MismatchedResponse
            | BrokerClientError::ConnectionClosed
            | BrokerClientError::ServerDenied(BrokerResponseError::InvalidRequest)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageNode;
    use crate::storage::model::Generation;

    fn node(name: &str, kind: StorageKind, generation: Generation) -> StorageNode {
        StorageNode {
            id: format!("block:{name}"),
            name: name.to_owned(),
            kind,
            materialization: Materialization::BlockDevice,
            removable: Some(false),
            identities: Vec::new(),
            generation,
        }
    }

    #[test]
    fn subjects_require_complete_whole_scsi_like_generations() {
        let complete = Generation {
            diskseq: Some(42),
            dev_t: Some((8, 0)),
        };
        let inventory = StorageInventory {
            nodes: vec![
                node("sda", StorageKind::ScsiLike, complete),
                node("nvme0n1", StorageKind::Nvme, Generation::default()),
                node("sdb", StorageKind::ScsiLike, Generation::default()),
            ],
            edges: Vec::new(),
            partial: false,
        };

        let subjects = broker_subjects(&inventory);

        assert_eq!(subjects.len(), 1);
        assert_eq!(subjects[0].device, "sda");
        assert_eq!(subjects[0].device_ref.generation.diskseq, 42);
    }

    #[test]
    fn smart_status_maps_without_interpreting_vendor_attributes() {
        let passed = disk_from_status("sda".into(), SmartStatus::Passed);
        assert_eq!(passed.health, HealthStatus::Healthy);
        assert_eq!(passed.health_availability, MetricAvailability::Available);
        assert_eq!(passed.temperature_c, None);
        assert_eq!(passed.power_on_hours, None);

        let failed = disk_from_status("sdb".into(), SmartStatus::PredictingFailure);
        assert_eq!(failed.health, HealthStatus::Failed);

        let unknown = disk_from_status("sdc".into(), SmartStatus::Unknown);
        assert_eq!(unknown.health, HealthStatus::Unavailable);
        assert_eq!(unknown.health_availability, MetricAvailability::Malformed);
    }

    #[tokio::test]
    async fn partial_inventory_is_never_routed_to_the_broker() {
        let inventory = StorageInventory {
            nodes: vec![node(
                "sda",
                StorageKind::ScsiLike,
                Generation {
                    diskseq: Some(42),
                    dev_t: Some((8, 0)),
                },
            )],
            edges: Vec::new(),
            partial: true,
        };

        let snapshot = BrokerHealthCollector::default().collect(&inventory).await;
        assert!(!snapshot.connected);
        assert!(snapshot.disks.is_empty());
        assert!(snapshot.handled_devices.is_empty());
    }

    #[tokio::test]
    async fn reconnect_backoff_skips_repeated_socket_attempts() {
        let inventory = StorageInventory {
            nodes: vec![node(
                "sda",
                StorageKind::ScsiLike,
                Generation {
                    diskseq: Some(42),
                    dev_t: Some((8, 0)),
                },
            )],
            edges: Vec::new(),
            partial: false,
        };
        let mut collector = BrokerHealthCollector {
            client: None,
            next_connect_at: Some(Instant::now() + RECONNECT_BACKOFF),
        };

        let snapshot = collector.collect(&inventory).await;

        assert!(!snapshot.connected);
        assert!(snapshot.disks.is_empty());
    }
}
