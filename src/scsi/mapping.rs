use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingAvailability {
    Complete,
    NoScsiGenericInterface,
    DeviceGone,
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScsiGenericMapping {
    /// Ephemeral kernel locators such as `sg0`; never persistent identities.
    pub entries: Vec<String>,
    pub rejected_entries: usize,
    pub availability: MappingAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingError {
    InvalidBlockName,
}

/// Discover `/sys/class/block/<name>/device/scsi_generic/` under an injected
/// block-class root. This function reads metadata only and never opens `/dev`.
pub fn discover_scsi_generic(
    block_class_root: &Path,
    block_name: &str,
) -> Result<ScsiGenericMapping, MappingError> {
    if !valid_block_name(block_name) {
        return Err(MappingError::InvalidBlockName);
    }

    let block = block_class_root.join(block_name);
    if !block.exists() {
        return Ok(mapping(MappingAvailability::DeviceGone));
    }
    let generic = block.join("device/scsi_generic");
    let entries = match fs::read_dir(&generic) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let availability = if block.exists() {
                MappingAvailability::NoScsiGenericInterface
            } else {
                MappingAvailability::DeviceGone
            };
            return Ok(mapping(availability));
        }
        Err(_) => return Ok(mapping(MappingAvailability::Unreadable)),
    };

    let mut names = Vec::new();
    let mut rejected_entries = 0;
    for entry in entries {
        let Ok(entry) = entry else {
            rejected_entries += 1;
            continue;
        };
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            rejected_entries += 1;
            continue;
        };
        if valid_sg_name(name) {
            names.push(name.to_owned());
        } else {
            rejected_entries += 1;
        }
    }
    names.sort();
    names.dedup();
    Ok(ScsiGenericMapping {
        entries: names,
        rejected_entries,
        availability: MappingAvailability::Complete,
    })
}

fn mapping(availability: MappingAvailability) -> ScsiGenericMapping {
    ScsiGenericMapping {
        entries: Vec::new(),
        rejected_entries: 0,
        availability,
    }
}

fn valid_block_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'!'))
}

fn valid_sg_name(name: &str) -> bool {
    name.strip_prefix("sg").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    fn fixture_root() -> std::path::PathBuf {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "vault-watch-scsi-mapping-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn discovers_zero_one_and_multiple_ephemeral_sg_locators() {
        let root = fixture_root();
        let generic = root.join("disk-fixture/device/scsi_generic");
        fs::create_dir_all(&generic).unwrap();
        assert_eq!(
            discover_scsi_generic(&root, "disk-fixture").unwrap(),
            ScsiGenericMapping {
                entries: vec![],
                rejected_entries: 0,
                availability: MappingAvailability::Complete,
            }
        );
        fs::create_dir(generic.join("sg9")).unwrap();
        assert_eq!(
            discover_scsi_generic(&root, "disk-fixture")
                .unwrap()
                .entries,
            ["sg9"]
        );
        fs::create_dir(generic.join("sg2")).unwrap();
        assert_eq!(
            discover_scsi_generic(&root, "disk-fixture")
                .unwrap()
                .entries,
            ["sg2", "sg9"]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_interface_and_hot_removed_device_are_distinct() {
        let root = fixture_root();
        fs::create_dir(root.join("disk-fixture")).unwrap();
        assert_eq!(
            discover_scsi_generic(&root, "disk-fixture")
                .unwrap()
                .availability,
            MappingAvailability::NoScsiGenericInterface
        );
        fs::remove_dir_all(root.join("disk-fixture")).unwrap();
        assert_eq!(
            discover_scsi_generic(&root, "disk-fixture")
                .unwrap()
                .availability,
            MappingAvailability::DeviceGone
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_names_cannot_escape_root_or_become_device_locators() {
        let root = fixture_root();
        assert_eq!(
            discover_scsi_generic(&root, "../outside"),
            Err(MappingError::InvalidBlockName)
        );
        let generic = root.join("disk-fixture/device/scsi_generic");
        fs::create_dir_all(&generic).unwrap();
        fs::create_dir(generic.join("not-a-device")).unwrap();
        fs::create_dir(generic.join("sgx")).unwrap();
        let mapping = discover_scsi_generic(&root, "disk-fixture").unwrap();
        assert!(mapping.entries.is_empty());
        assert_eq!(mapping.rejected_entries, 2);
        fs::remove_dir_all(root).unwrap();
    }
}
