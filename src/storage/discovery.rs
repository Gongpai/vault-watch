use std::path::Path;

use super::model::{StorageInventory, StorageKind, StorageNode};

/// Read-only discovery from the flat block class. This phase does not open any
/// device node, filesystem, mount, or user file and issues no raw commands.
pub fn discover_storage() -> StorageInventory {
    discover_at(Path::new("/sys/class/block"))
}

fn discover_at(root: &Path) -> StorageInventory {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => {
            return StorageInventory {
                nodes: Vec::new(),
                partial: true,
            };
        }
    };

    let mut inventory = StorageInventory::default();
    for entry in entries {
        let Ok(entry) = entry else {
            inventory.partial = true;
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_partition(&entry.path()) {
            continue;
        }
        let removable = read_bool(&entry.path().join("removable"));
        inventory.nodes.push(StorageNode {
            kind: classify(&name),
            name,
            removable,
        });
    }
    inventory.nodes.sort_by(|a, b| a.name.cmp(&b.name));
    inventory
}

fn is_partition(path: &Path) -> bool {
    std::fs::read_to_string(path.join("uevent"))
        .map(|value| value.lines().any(|line| line == "DEVTYPE=partition"))
        .unwrap_or(false)
}

fn read_bool(path: &Path) -> Option<bool> {
    match std::fs::read_to_string(path).ok()?.trim() {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn classify(name: &str) -> StorageKind {
    if name.starts_with("nvme") {
        StorageKind::Nvme
    } else if name.starts_with("mmcblk") {
        StorageKind::Mmc
    } else if name.starts_with("md") {
        StorageKind::MdRaid
    } else if name.starts_with("dm-") {
        StorageKind::DeviceMapper
    } else if name.starts_with("sd") || name.starts_with("hd") {
        StorageKind::ScsiLike
    } else if ["loop", "ram", "zram", "nbd", "vd", "xvd", "ublk"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
    {
        StorageKind::Virtual
    } else {
        StorageKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_protocol_views_without_claiming_physical_media() {
        assert_eq!(classify("sda"), StorageKind::ScsiLike);
        assert_eq!(classify("nvme0n1"), StorageKind::Nvme);
        assert_eq!(classify("mmcblk0"), StorageKind::Mmc);
        assert_eq!(classify("dm-0"), StorageKind::DeviceMapper);
        assert_eq!(classify("loop0"), StorageKind::Virtual);
    }
}
