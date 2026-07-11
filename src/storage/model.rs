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

#[derive(Debug, Clone)]
pub struct StorageNode {
    pub name: String,
    pub kind: StorageKind,
    pub removable: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct StorageInventory {
    pub nodes: Vec<StorageNode>,
    pub partial: bool,
}

impl StorageInventory {
    pub fn count_kind(&self, kind: StorageKind) -> usize {
        self.nodes.iter().filter(|node| node.kind == kind).count()
    }

    pub fn removable_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.removable == Some(true))
            .count()
    }
}
