use std::collections::HashSet;
use std::path::Path;

use super::model::{
    Confidence, Generation, IdentityClaim, IdentityScope, IdentitySource, Materialization,
    StorageEdge, StorageEdgeKind, StorageInventory, StorageKind, StorageNode,
};

/// Read-only discovery from the flat block class. This phase does not open any
/// device node, filesystem, mount, or user file and issues no raw commands.
pub fn discover_storage() -> StorageInventory {
    discover_storage_from(Path::new("/sys/class/block"))
}

/// Discover storage beneath an injectable sysfs block-class root.
pub fn discover_storage_from(root: &Path) -> StorageInventory {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => {
            return StorageInventory {
                partial: true,
                ..StorageInventory::default()
            };
        }
    };

    let mut inventory = StorageInventory::default();
    let mut paths = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => paths.push(entry.path()),
            Err(_) => inventory.partial = true,
        }
    }
    paths.sort();

    for path in &paths {
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            inventory.partial = true;
            continue;
        };
        inventory.nodes.push(read_node(path, name));
    }

    let known: HashSet<String> = inventory
        .nodes
        .iter()
        .map(|node| node.name.clone())
        .collect();
    for path in &paths {
        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        if is_partition(path) {
            match partition_parent(path) {
                Some(parent) if known.contains(&parent) => inventory.edges.push(StorageEdge {
                    from: node_id(&parent),
                    to: node_id(&name),
                    kind: StorageEdgeKind::ContainsPartition,
                }),
                _ => inventory.partial = true,
            }
        }

        let slaves = match std::fs::read_dir(path.join("slaves")) {
            Ok(slaves) => slaves,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                inventory.partial = true;
                continue;
            }
        };
        for slave in slaves {
            let Ok(slave) = slave else {
                inventory.partial = true;
                continue;
            };
            let slave_name = slave.file_name().to_string_lossy().into_owned();
            if !known.contains(&slave_name) {
                inventory.partial = true;
                continue;
            }
            let (from, to, kind) = if classify(&name) == StorageKind::MdRaid {
                (slave_name, name.clone(), StorageEdgeKind::MemberOf)
            } else {
                (name.clone(), slave_name, StorageEdgeKind::BackedBy)
            };
            inventory.edges.push(StorageEdge {
                from: node_id(&from),
                to: node_id(&to),
                kind,
            });
        }
    }

    inventory.nodes.sort_by(|a, b| a.name.cmp(&b.name));
    inventory
        .edges
        .sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    inventory.edges.dedup();
    if inventory.validate().is_err() {
        inventory.partial = true;
    }
    inventory
}

fn read_node(path: &Path, name: String) -> StorageNode {
    let partition = is_partition(path);
    let kind = classify(&name);
    let dev_t = read_dev_t(&path.join("dev"));
    let diskseq = read_u64(&path.join("diskseq"));
    let mut identities = vec![IdentityClaim {
        value: name.clone(),
        scope: IdentityScope::KernelObject,
        source: IdentitySource::ClassName,
        confidence: Confidence::Low,
    }];
    if let Some((major, minor)) = dev_t {
        identities.push(IdentityClaim {
            value: format!("{major}:{minor}"),
            scope: IdentityScope::BlockDevice,
            source: IdentitySource::DevNumber,
            confidence: Confidence::Medium,
        });
    }
    if let Some(sequence) = diskseq {
        identities.push(IdentityClaim {
            value: sequence.to_string(),
            scope: IdentityScope::KernelObject,
            source: IdentitySource::DiskSequence,
            confidence: Confidence::High,
        });
    }

    StorageNode {
        id: node_id(&name),
        name,
        kind,
        materialization: if partition {
            Materialization::Partition
        } else {
            match kind {
                StorageKind::MdRaid | StorageKind::DeviceMapper => Materialization::Stacked,
                StorageKind::Virtual => Materialization::Virtual,
                StorageKind::Other => Materialization::Unknown,
                _ => Materialization::BlockDevice,
            }
        },
        removable: read_bool(&path.join("removable")),
        identities,
        generation: Generation { diskseq, dev_t },
    }
}

fn node_id(name: &str) -> String {
    format!("block:{name}")
}

fn is_partition(path: &Path) -> bool {
    path.join("partition").exists()
        || std::fs::read_to_string(path.join("uevent"))
            .map(|value| value.lines().any(|line| line == "DEVTYPE=partition"))
            .unwrap_or(false)
}

fn partition_parent(path: &Path) -> Option<String> {
    let resolved = std::fs::canonicalize(path).ok()?;
    let parent = resolved
        .parent()?
        .file_name()?
        .to_string_lossy()
        .into_owned();
    (parent != resolved.file_name()?.to_string_lossy()).then_some(parent)
}

fn read_bool(path: &Path) -> Option<bool> {
    match std::fs::read_to_string(path).ok()?.trim() {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn read_u64(path: &Path) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_dev_t(path: &Path) -> Option<(u32, u32)> {
    let value = std::fs::read_to_string(path).ok()?;
    let (major, minor) = value.trim().split_once(':')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
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
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("vault-watch-sysfs-{}-{nonce}", std::process::id()));
            fs::create_dir_all(path.join("class/block")).unwrap();
            Self(path)
        }

        fn block(&self, name: &str, parent: Option<&str>, dev: &str, diskseq: u64) {
            let target = match parent {
                Some(parent) => self.0.join("devices").join(parent).join(name),
                None => self.0.join("devices").join(name),
            };
            fs::create_dir_all(&target).unwrap();
            fs::write(target.join("dev"), dev).unwrap();
            fs::write(target.join("diskseq"), diskseq.to_string()).unwrap();
            fs::write(target.join("removable"), "0").unwrap();
            if parent.is_some() {
                fs::write(target.join("partition"), "1").unwrap();
                fs::write(target.join("uevent"), "DEVTYPE=partition\n").unwrap();
            } else {
                fs::write(target.join("uevent"), "DEVTYPE=disk\n").unwrap();
            }
            symlink(&target, self.0.join("class/block").join(name)).unwrap();
        }

        fn slave(&self, owner: &str, slave: &str) {
            let owner = self.0.join("devices").join(owner);
            fs::create_dir_all(owner.join("slaves")).unwrap();
            symlink(
                self.0.join("devices").join(slave),
                owner.join("slaves").join(slave),
            )
            .unwrap();
        }

        fn root(&self) -> PathBuf {
            self.0.join("class/block")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn classifies_protocol_views_without_claiming_physical_media() {
        assert_eq!(classify("sda"), StorageKind::ScsiLike);
        assert_eq!(classify("nvme0n1"), StorageKind::Nvme);
        assert_eq!(classify("mmcblk0"), StorageKind::Mmc);
        assert_eq!(classify("dm-0"), StorageKind::DeviceMapper);
        assert_eq!(classify("loop0"), StorageKind::Virtual);
    }

    #[test]
    fn fixture_builds_partition_and_stacked_edges_with_generations() {
        let fixture = Fixture::new();
        fixture.block("sda", None, "8:0", 10);
        fixture.block("sda1", Some("sda"), "8:1", 10);
        fixture.block("dm-0", None, "253:0", 11);
        fixture.slave("dm-0", "sda");

        let inventory = discover_storage_from(&fixture.root());

        assert!(!inventory.partial);
        assert_eq!(inventory.nodes.len(), 3);
        assert!(inventory.edges.contains(&StorageEdge {
            from: "block:sda".into(),
            to: "block:sda1".into(),
            kind: StorageEdgeKind::ContainsPartition,
        }));
        assert!(inventory.edges.contains(&StorageEdge {
            from: "block:dm-0".into(),
            to: "block:sda".into(),
            kind: StorageEdgeKind::BackedBy,
        }));
        assert_eq!(
            inventory
                .nodes
                .iter()
                .filter(|node| node.generation.diskseq.is_some())
                .count(),
            3
        );
        assert_eq!(
            inventory
                .nodes
                .iter()
                .map(|node| node.identities.len())
                .sum::<usize>(),
            9
        );
    }

    #[test]
    fn missing_root_is_an_empty_partial_inventory() {
        let fixture = Fixture::new();
        let inventory = discover_storage_from(&fixture.0.join("missing"));

        assert!(inventory.partial);
        assert!(inventory.nodes.is_empty());
        assert!(inventory.edges.is_empty());
    }

    #[test]
    fn empty_root_is_a_complete_empty_inventory() {
        let fixture = Fixture::new();
        let inventory = discover_storage_from(&fixture.root());

        assert!(!inventory.partial);
        assert!(inventory.nodes.is_empty());
    }
}
