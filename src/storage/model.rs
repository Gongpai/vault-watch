use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    ScsiLike,
    Nvme,
    Mmc,
    MdRaid,
    DeviceMapper,
    Virtual,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Materialization {
    BlockDevice,
    Partition,
    Stacked,
    Virtual,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityScope {
    KernelObject,
    BlockDevice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentitySource {
    ClassName,
    DevNumber,
    DiskSequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityClaim {
    pub value: String,
    pub scope: IdentityScope,
    pub source: IdentitySource,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Generation {
    pub diskseq: Option<u64>,
    pub dev_t: Option<(u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageNode {
    pub id: String,
    pub name: String,
    pub kind: StorageKind,
    pub materialization: Materialization,
    pub removable: Option<bool>,
    pub identities: Vec<IdentityClaim>,
    pub generation: Generation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageEdgeKind {
    ContainsPartition,
    BackedBy,
    MemberOf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageEdge {
    pub from: String,
    pub to: String,
    pub kind: StorageEdgeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphViolation {
    DuplicateNode(String),
    DanglingEdge { from: String, to: String },
    PartitionContainsNode(String),
}

#[derive(Debug, Clone, Default)]
pub struct StorageInventory {
    pub nodes: Vec<StorageNode>,
    pub edges: Vec<StorageEdge>,
    pub partial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThroughputSubject {
    pub name: String,
    pub dev_t: Option<(u32, u32)>,
    pub diskseq: Option<u64>,
}

impl StorageInventory {
    pub fn replaced_device_names(&self, next: &StorageInventory) -> HashSet<String> {
        let current: HashMap<&str, &Generation> = self
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), &node.generation))
            .collect();
        next.nodes
            .iter()
            .filter(|node| {
                current
                    .get(node.id.as_str())
                    .is_some_and(|generation| *generation != &node.generation)
            })
            .map(|node| node.name.clone())
            .collect()
    }

    /// Atomically publish a newly discovered topology generation. A completely
    /// empty partial snapshot means discovery itself failed, so retain the last
    /// known graph and mark it partial instead of reporting every device gone.
    pub fn reconcile(&mut self, next: StorageInventory) {
        if next.partial && next.nodes.is_empty() && !self.nodes.is_empty() {
            self.partial = true;
            return;
        }
        *self = next;
    }

    pub fn count_whole_kind(&self, kind: StorageKind) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.kind == kind && node.materialization != Materialization::Partition)
            .count()
    }

    pub fn partition_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.materialization == Materialization::Partition)
            .count()
    }

    pub fn virtual_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.materialization == Materialization::Virtual)
            .count()
    }

    pub fn whole_block_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| {
                !matches!(
                    node.materialization,
                    Materialization::Partition | Materialization::Virtual
                )
            })
            .count()
    }

    pub fn whole_device_names(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|node| node.materialization == Materialization::BlockDevice)
            .map(|node| node.name.clone())
            .collect()
    }

    pub fn throughput_subjects(&self) -> Vec<ThroughputSubject> {
        self.nodes
            .iter()
            .filter(|node| node.materialization == Materialization::BlockDevice)
            .map(|node| ThroughputSubject {
                name: node.name.clone(),
                dev_t: node.generation.dev_t,
                diskseq: node.generation.diskseq,
            })
            .collect()
    }

    pub fn removable_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.removable == Some(true))
            .count()
    }

    pub fn reachable_from(&self, start: &str) -> HashSet<String> {
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in &self.edges {
            adjacency
                .entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([start]);
        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.to_owned()) {
                continue;
            }
            if let Some(neighbours) = adjacency.get(current) {
                queue.extend(neighbours.iter().copied());
            }
        }
        visited
    }

    pub fn validate(&self) -> Result<(), GraphViolation> {
        let mut nodes = HashMap::new();
        for node in &self.nodes {
            if nodes.insert(node.id.as_str(), node).is_some() {
                return Err(GraphViolation::DuplicateNode(node.id.clone()));
            }
        }

        for edge in &self.edges {
            let (Some(from), Some(_)) =
                (nodes.get(edge.from.as_str()), nodes.get(edge.to.as_str()))
            else {
                return Err(GraphViolation::DanglingEdge {
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                });
            };
            if from.materialization == Materialization::Partition
                && edge.kind == StorageEdgeKind::ContainsPartition
            {
                return Err(GraphViolation::PartitionContainsNode(from.id.clone()));
            }
        }

        // Exercise cycle-safe traversal for every component. Cycles are tolerated
        // defensively because sysfs is not an atomic snapshot.
        for node in &self.nodes {
            self.reachable_from(&node.id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> StorageNode {
        StorageNode {
            id: id.to_owned(),
            name: id.to_owned(),
            kind: StorageKind::Other,
            materialization: Materialization::BlockDevice,
            removable: None,
            identities: Vec::new(),
            generation: Generation::default(),
        }
    }

    #[test]
    fn traversal_terminates_when_graph_contains_cycle() {
        let graph = StorageInventory {
            nodes: vec![node("a"), node("b")],
            edges: vec![
                StorageEdge {
                    from: "a".into(),
                    to: "b".into(),
                    kind: StorageEdgeKind::BackedBy,
                },
                StorageEdge {
                    from: "b".into(),
                    to: "a".into(),
                    kind: StorageEdgeKind::BackedBy,
                },
            ],
            partial: false,
        };

        assert_eq!(
            graph.reachable_from("a"),
            HashSet::from(["a".into(), "b".into()])
        );
        assert_eq!(graph.validate(), Ok(()));
    }

    #[test]
    fn validation_rejects_dangling_edges() {
        let graph = StorageInventory {
            nodes: vec![node("a")],
            edges: vec![StorageEdge {
                from: "a".into(),
                to: "missing".into(),
                kind: StorageEdgeKind::BackedBy,
            }],
            partial: false,
        };

        assert!(matches!(
            graph.validate(),
            Err(GraphViolation::DanglingEdge { .. })
        ));
    }

    #[test]
    fn throughput_scope_selects_only_direct_whole_devices() {
        let mut whole = node("block:sda");
        whole.name = "sda".into();
        whole.kind = StorageKind::ScsiLike;
        whole.generation = Generation {
            diskseq: Some(10),
            dev_t: Some((8, 0)),
        };
        let mut partition = node("block:sda1");
        partition.name = "sda1".into();
        partition.materialization = Materialization::Partition;
        let mut virtual_node = node("block:loop0");
        virtual_node.name = "loop0".into();
        virtual_node.materialization = Materialization::Virtual;
        let mut stacked = node("block:dm-0");
        stacked.name = "dm-0".into();
        stacked.materialization = Materialization::Stacked;
        let inventory = StorageInventory {
            nodes: vec![whole, partition, virtual_node, stacked],
            edges: Vec::new(),
            partial: false,
        };

        assert_eq!(
            inventory.throughput_subjects(),
            vec![ThroughputSubject {
                name: "sda".into(),
                dev_t: Some((8, 0)),
                diskseq: Some(10),
            }]
        );
    }

    #[test]
    fn reconciliation_replaces_a_device_incarnation_atomically() {
        let mut current = StorageInventory {
            nodes: vec![node_with_generation("block:sda", 10, (8, 0))],
            edges: Vec::new(),
            partial: false,
        };
        let next = StorageInventory {
            nodes: vec![node_with_generation("block:sda", 11, (8, 0))],
            edges: Vec::new(),
            partial: false,
        };

        assert_eq!(
            current.replaced_device_names(&next),
            HashSet::from(["block:sda".into()])
        );
        current.reconcile(next);

        assert_eq!(current.nodes.len(), 1);
        assert_eq!(current.nodes[0].generation.diskseq, Some(11));
        assert!(!current.partial);
    }

    #[test]
    fn failed_empty_snapshot_retains_last_known_graph_as_partial() {
        let mut current = StorageInventory {
            nodes: vec![node_with_generation("block:sda", 10, (8, 0))],
            edges: Vec::new(),
            partial: false,
        };

        current.reconcile(StorageInventory {
            partial: true,
            ..StorageInventory::default()
        });

        assert_eq!(current.nodes.len(), 1);
        assert_eq!(current.nodes[0].generation.diskseq, Some(10));
        assert!(current.partial);
    }

    #[test]
    fn complete_empty_snapshot_removes_all_devices() {
        let mut current = StorageInventory {
            nodes: vec![node_with_generation("block:sda", 10, (8, 0))],
            edges: Vec::new(),
            partial: false,
        };

        current.reconcile(StorageInventory::default());

        assert!(current.nodes.is_empty());
        assert!(!current.partial);
    }

    fn node_with_generation(id: &str, diskseq: u64, dev_t: (u32, u32)) -> StorageNode {
        let mut node = node(id);
        node.generation = Generation {
            diskseq: Some(diskseq),
            dev_t: Some(dev_t),
        };
        node
    }
}
