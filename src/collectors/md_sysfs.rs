use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::app::{RaidState, RaidStatus};

const SNAPSHOT_ATTEMPTS: usize = 2;
const MIN_DELTA_SAMPLE_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdArrayState {
    Clear,
    Inactive,
    Readonly,
    ReadAuto,
    Clean,
    Active,
    WritePending,
    ActiveIdle,
    Unknown(String),
}

impl MdArrayState {
    fn parse(value: &str) -> Self {
        match value {
            "clear" => Self::Clear,
            "inactive" => Self::Inactive,
            "readonly" => Self::Readonly,
            "read-auto" => Self::ReadAuto,
            "clean" => Self::Clean,
            "active" => Self::Active,
            "write-pending" => Self::WritePending,
            "active-idle" => Self::ActiveIdle,
            other => Self::Unknown(other.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdSyncAction {
    Idle,
    Resync,
    Recover,
    Check,
    Repair,
    Reshape,
    Unknown(String),
}

impl MdSyncAction {
    fn parse(value: &str) -> Self {
        match value {
            "idle" => Self::Idle,
            "resync" => Self::Resync,
            "recover" => Self::Recover,
            "check" => Self::Check,
            "repair" => Self::Repair,
            "reshape" => Self::Reshape,
            other => Self::Unknown(other.to_owned()),
        }
    }

    fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Resync | Self::Recover | Self::Check | Self::Repair | Self::Reshape
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdMemberState {
    pub flags: Vec<String>,
}

impl MdMemberState {
    fn parse(value: &str) -> Self {
        let mut flags: Vec<String> = value
            .split(',')
            .map(str::trim)
            .filter(|flag| !flag.is_empty())
            .map(str::to_owned)
            .collect();
        flags.sort();
        flags.dedup();
        Self { flags }
    }

    fn contains(&self, flag: &str) -> bool {
        self.flags.iter().any(|item| item == flag)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdProgress {
    pub completed_sectors: u64,
    pub total_sectors: u64,
}

impl MdProgress {
    pub fn percent(&self) -> f64 {
        self.completed_sectors as f64 / self.total_sectors as f64 * 100.0
    }

    pub fn eta_seconds(&self, speed_kib_per_sec: u64) -> Option<u64> {
        if speed_kib_per_sec == 0 {
            return None;
        }
        let remaining = self.total_sectors.checked_sub(self.completed_sectors)? as u128;
        let bytes = remaining.checked_mul(512)?;
        let bytes_per_second = speed_kib_per_sec as u128 * 1_024;
        let seconds = bytes.div_ceil(bytes_per_second);
        u64::try_from(seconds).ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdMemberSnapshot {
    pub kernel_entry: String,
    pub block_device: Option<String>,
    pub state: MdMemberState,
    pub slot: Option<u32>,
    pub errors: Option<u64>,
    pub recovery_start: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdArraySnapshot {
    pub name: String,
    pub level: String,
    pub state: MdArrayState,
    pub raid_disks: u32,
    pub degraded: u32,
    pub action: MdSyncAction,
    pub progress: Option<MdProgress>,
    pub sync_speed_kib_per_sec: Option<u64>,
    pub metadata_version: String,
    pub external_metadata: bool,
    pub members: Vec<MdMemberSnapshot>,
    pub consistent: bool,
}

impl MdArraySnapshot {
    fn legacy_status(&self) -> RaidStatus {
        let total = self.raid_disks.min(u8::MAX as u32) as u8;
        let active = self
            .raid_disks
            .saturating_sub(self.degraded)
            .min(u8::MAX as u32) as u8;
        let operation_active = self.action.is_active();
        let member_problem = self.members.iter().any(|member| {
            member.state.contains("faulty")
                || member.state.contains("blocked")
                || member.state.contains("write_error")
        });
        let state = if !self.consistent {
            RaidState::Unknown
        } else if operation_active {
            RaidState::Rebuilding
        } else if self.degraded > 0 || member_problem {
            RaidState::Degraded
        } else if matches!(
            self.state,
            MdArrayState::Active | MdArrayState::ActiveIdle | MdArrayState::Clean
        ) {
            RaidState::Active
        } else {
            RaidState::Unknown
        };
        let speed = self.sync_speed_kib_per_sec;
        RaidStatus {
            name: self.name.clone(),
            state,
            rebuild_pct: self.progress.as_ref().map(MdProgress::percent),
            rebuild_speed_mb: speed.map(|value| value / 1_024),
            eta_minutes: self
                .progress
                .as_ref()
                .and_then(|progress| progress.eta_seconds(speed?))
                .map(|seconds| seconds.div_ceil(60)),
            active_disks: active,
            total_disks: total,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MdInventory {
    pub arrays: Vec<MdArraySnapshot>,
    pub partial: bool,
}

#[derive(Debug, Default)]
pub struct MdOperationSampler {
    previous: HashMap<String, MdOperationBaseline>,
}

#[derive(Debug, Clone)]
struct MdOperationBaseline {
    action: MdSyncAction,
    total_sectors: u64,
    completed_sectors: u64,
    metadata_version: String,
    raid_disks: u32,
    observed_at: Instant,
}

impl MdOperationSampler {
    pub fn statuses(&mut self, inventory: &MdInventory, now: Instant) -> Vec<RaidStatus> {
        let present: HashSet<&str> = inventory
            .arrays
            .iter()
            .map(|array| array.name.as_str())
            .collect();
        self.previous
            .retain(|name, _| present.contains(name.as_str()));

        inventory
            .arrays
            .iter()
            .map(|array| {
                let mut status = array.legacy_status();
                let Some(progress) = array.progress.as_ref().filter(|_| array.action.is_active())
                else {
                    self.previous.remove(&array.name);
                    return status;
                };

                let mut update_baseline = true;
                if let Some(previous) = self.previous.get(&array.name)
                    && previous.action == array.action
                    && previous.total_sectors == progress.total_sectors
                    && previous.metadata_version == array.metadata_version
                    && previous.raid_disks == array.raid_disks
                    && let Some(delta_sectors) = progress
                        .completed_sectors
                        .checked_sub(previous.completed_sectors)
                {
                    let elapsed = now.saturating_duration_since(previous.observed_at);
                    if delta_sectors == 0 || elapsed < MIN_DELTA_SAMPLE_INTERVAL {
                        // sync_completed can remain unchanged across short
                        // polling intervals, while event hints can cause a
                        // sub-second sample that exaggerates delta speed. Keep
                        // kernel sync_speed/ETA and the older baseline until a
                        // representative observation window has elapsed.
                        update_baseline = false;
                    } else {
                        if let Some(speed_kib_per_sec) = delta_speed_kib(delta_sectors, elapsed) {
                            status.rebuild_speed_mb = Some(speed_kib_per_sec / 1_024);
                            status.eta_minutes = progress
                                .eta_seconds(speed_kib_per_sec)
                                .map(|seconds| seconds.div_ceil(60));
                        }
                    }
                }

                if update_baseline {
                    self.previous.insert(
                        array.name.clone(),
                        MdOperationBaseline {
                            action: array.action.clone(),
                            total_sectors: progress.total_sectors,
                            completed_sectors: progress.completed_sectors,
                            metadata_version: array.metadata_version.clone(),
                            raid_disks: array.raid_disks,
                            observed_at: now,
                        },
                    );
                }
                status
            })
            .collect()
    }
}

fn delta_speed_kib(delta_sectors: u64, elapsed: Duration) -> Option<u64> {
    let elapsed_seconds = elapsed.as_secs_f64();
    if delta_sectors == 0 || elapsed_seconds <= 0.0 {
        return None;
    }
    let speed = delta_sectors as f64 / 2.0 / elapsed_seconds;
    speed.is_finite().then_some(speed as u64)
}

#[derive(Debug)]
pub enum MdError {
    Io(std::io::Error),
    Malformed { path: PathBuf, value: String },
}

impl fmt::Display for MdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "MD sysfs I/O error: {error}"),
            Self::Malformed { path, value } => {
                write!(
                    formatter,
                    "malformed MD sysfs value at {} ({} bytes)",
                    path.display(),
                    value.len()
                )
            }
        }
    }
}

impl std::error::Error for MdError {}

/// Collect a read-only MD snapshot from an injectable block-class root.
pub fn collect(block_root: &Path) -> Result<MdInventory, MdError> {
    let entries = std::fs::read_dir(block_root).map_err(MdError::Io)?;
    let mut candidates = Vec::new();
    for entry in entries {
        let entry = entry.map_err(MdError::Io)?;
        if entry.path().join("md").is_dir() {
            candidates.push(entry.path());
        }
    }
    candidates.sort();

    let mut inventory = MdInventory::default();
    for path in candidates {
        match read_array(&path) {
            Ok(snapshot) => inventory.arrays.push(snapshot),
            Err(_) => inventory.partial = true,
        }
    }
    Ok(inventory)
}

fn read_array(block_path: &Path) -> Result<MdArraySnapshot, MdError> {
    let mut last = None;
    for _ in 0..SNAPSHOT_ATTEMPTS {
        let snapshot = read_array_once(block_path)?;
        if snapshot.consistent {
            return Ok(snapshot);
        }
        last = Some(snapshot);
    }
    Ok(last.expect("snapshot attempts is non-zero"))
}

fn read_array_once(block_path: &Path) -> Result<MdArraySnapshot, MdError> {
    let md = block_path.join("md");
    let first_state = read_required(&md.join("array_state"))?;
    let first_action = read_required(&md.join("sync_action"))?;
    let action = MdSyncAction::parse(&first_action);
    let progress = if action.is_active() {
        parse_progress(
            &md.join("sync_completed"),
            &read_required(&md.join("sync_completed"))?,
        )?
    } else {
        None
    };
    let metadata_version = read_required(&md.join("metadata_version"))?;
    let mut members = read_members(&md)?;
    members.sort_by(|left, right| {
        left.slot
            .cmp(&right.slot)
            .then_with(|| left.kernel_entry.cmp(&right.kernel_entry))
    });
    let last_state = read_required(&md.join("array_state"))?;
    let last_action = read_required(&md.join("sync_action"))?;

    Ok(MdArraySnapshot {
        name: block_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        level: read_required(&md.join("level"))?,
        state: MdArrayState::parse(&first_state),
        raid_disks: parse_required(&md.join("raid_disks"))?,
        degraded: parse_required(&md.join("degraded"))?,
        action,
        progress,
        sync_speed_kib_per_sec: read_optional_u64(&md.join("sync_speed"))?,
        external_metadata: metadata_version.starts_with("external:"),
        metadata_version,
        members,
        consistent: first_state == last_state && first_action == last_action,
    })
}

fn read_members(md: &Path) -> Result<Vec<MdMemberSnapshot>, MdError> {
    let mut members = Vec::new();
    for entry in std::fs::read_dir(md).map_err(MdError::Io)? {
        let entry = entry.map_err(MdError::Io)?;
        let kernel_entry = entry.file_name().to_string_lossy().into_owned();
        if !kernel_entry.starts_with("dev-") || !entry.path().is_dir() {
            continue;
        }
        let path = entry.path();
        members.push(MdMemberSnapshot {
            kernel_entry,
            block_device: resolve_block_name(&path.join("block")),
            state: MdMemberState::parse(&read_required(&path.join("state"))?),
            slot: read_optional_number(&path.join("slot"))?,
            errors: read_optional_u64(&path.join("errors"))?,
            recovery_start: read_optional_number(&path.join("recovery_start"))?,
        });
    }
    Ok(members)
}

fn resolve_block_name(path: &Path) -> Option<String> {
    std::fs::canonicalize(path)
        .ok()?
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn parse_progress(path: &Path, value: &str) -> Result<Option<MdProgress>, MdError> {
    if value == "none" || value.is_empty() {
        return Ok(None);
    }
    let Some((completed, total)) = value.split_once('/') else {
        return Err(malformed(path, value));
    };
    let completed = completed
        .trim()
        .parse::<u64>()
        .map_err(|_| malformed(path, value))?;
    let total = total
        .trim()
        .parse::<u64>()
        .map_err(|_| malformed(path, value))?;
    if total == 0 || completed > total {
        return Err(malformed(path, value));
    }
    Ok(Some(MdProgress {
        completed_sectors: completed,
        total_sectors: total,
    }))
}

fn read_required(path: &Path) -> Result<String, MdError> {
    std::fs::read_to_string(path)
        .map(|value| value.trim().to_owned())
        .map_err(MdError::Io)
}

fn parse_required<T: std::str::FromStr>(path: &Path) -> Result<T, MdError> {
    let value = read_required(path)?;
    value.parse().map_err(|_| malformed(path, &value))
}

fn read_optional_u64(path: &Path) -> Result<Option<u64>, MdError> {
    match std::fs::read_to_string(path) {
        Ok(value) => value
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| malformed(path, value.trim())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(MdError::Io(error)),
    }
}

fn read_optional_number<T: std::str::FromStr>(path: &Path) -> Result<Option<T>, MdError> {
    match std::fs::read_to_string(path) {
        Ok(value) if matches!(value.trim(), "none" | "") => Ok(None),
        Ok(value) => value
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| malformed(path, value.trim())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(MdError::Io(error)),
    }
}

fn malformed(path: &Path, value: &str) -> MdError {
    MdError::Malformed {
        path: path.to_owned(),
        value: value.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root =
                std::env::temp_dir().join(format!("vault-watch-md-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn array(&self, name: &str, action: &str, degraded: &str, metadata: &str) -> PathBuf {
            let md = self.0.join(name).join("md");
            fs::create_dir_all(&md).unwrap();
            for (attribute, value) in [
                ("array_state", "active"),
                ("sync_action", action),
                ("level", "raid1"),
                ("raid_disks", "2"),
                ("degraded", degraded),
                ("metadata_version", metadata),
            ] {
                fs::write(md.join(attribute), value).unwrap();
            }
            md
        }

        fn member(&self, md: &Path, name: &str, state: &str, slot: &str) {
            let member = md.join(name);
            fs::create_dir_all(&member).unwrap();
            fs::write(member.join("state"), state).unwrap();
            fs::write(member.join("slot"), slot).unwrap();
            fs::write(member.join("errors"), "0").unwrap();
            fs::write(member.join("recovery_start"), "none").unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn enumerates_nonstandard_name_and_preserves_member_flags() {
        let fixture = Fixture::new();
        let md = fixture.array("array-name", "idle", "0", "1.2");
        fixture.member(&md, "dev-alpha", "in_sync,write_error", "0");
        fixture.member(&md, "dev-spare", "spare", "none");

        let inventory = collect(&fixture.0).unwrap();

        assert!(!inventory.partial);
        assert_eq!(inventory.arrays.len(), 1);
        let array = &inventory.arrays[0];
        assert_eq!(array.name, "array-name");
        assert_eq!(array.members.len(), 2);
        let active = array
            .members
            .iter()
            .find(|member| member.kernel_entry == "dev-alpha")
            .unwrap();
        let spare = array
            .members
            .iter()
            .find(|member| member.kernel_entry == "dev-spare")
            .unwrap();
        assert!(active.state.contains("in_sync"));
        assert!(active.state.contains("write_error"));
        assert_eq!(spare.slot, None);
        assert_eq!(array.legacy_status().state, RaidState::Degraded);
    }

    #[test]
    fn parses_recovery_progress_speed_eta_and_external_metadata() {
        let fixture = Fixture::new();
        let md = fixture.array("md-any", "recover", "1", "external:imsm");
        fs::write(md.join("sync_completed"), "1024 / 2048").unwrap();
        fs::write(md.join("sync_speed"), "512").unwrap();

        let inventory = collect(&fixture.0).unwrap();
        let array = &inventory.arrays[0];
        let status = array.legacy_status();

        assert!(array.external_metadata);
        assert_eq!(array.action, MdSyncAction::Recover);
        assert_eq!(array.progress.as_ref().unwrap().percent(), 50.0);
        assert_eq!(status.state, RaidState::Rebuilding);
        assert_eq!(status.eta_minutes, Some(1));
    }

    #[test]
    fn malformed_safety_value_marks_inventory_partial_not_healthy() {
        let fixture = Fixture::new();
        fixture.array("md-bad", "idle", "not-a-number", "1.2");

        let inventory = collect(&fixture.0).unwrap();

        assert!(inventory.partial);
        assert!(inventory.arrays.is_empty());
    }

    #[test]
    fn unknown_states_and_invalid_progress_remain_explicit() {
        assert_eq!(
            MdArrayState::parse("future"),
            MdArrayState::Unknown("future".into())
        );
        assert_eq!(
            MdSyncAction::parse("future"),
            MdSyncAction::Unknown("future".into())
        );
        assert!(parse_progress(Path::new("sync_completed"), "9 / 8").is_err());
    }

    #[test]
    fn operation_sampler_uses_delta_speed_and_resets_on_action_change() {
        let fixture = Fixture::new();
        let md = fixture.array("md-delta", "recover", "1", "1.2");
        fs::write(md.join("sync_completed"), "1024 / 8192").unwrap();
        fs::write(md.join("sync_speed"), "4096").unwrap();
        let started = Instant::now();
        let mut sampler = MdOperationSampler::default();

        let first = collect(&fixture.0).unwrap();
        let first_status = sampler.statuses(&first, started);
        assert_eq!(first_status[0].rebuild_speed_mb, Some(4));

        fs::write(md.join("sync_completed"), "5120 / 8192").unwrap();
        let second = collect(&fixture.0).unwrap();
        let second_status = sampler.statuses(&second, started + Duration::from_secs(2));
        assert_eq!(second_status[0].rebuild_speed_mb, Some(1));
        assert_eq!(second_status[0].eta_minutes, Some(1));

        fs::write(md.join("sync_action"), "check").unwrap();
        fs::write(md.join("sync_completed"), "6144 / 8192").unwrap();
        let changed = collect(&fixture.0).unwrap();
        let changed_status = sampler.statuses(&changed, started + Duration::from_secs(3));
        assert_eq!(changed_status[0].rebuild_speed_mb, Some(4));
    }

    #[test]
    fn unchanged_progress_keeps_kernel_speed_and_eta_until_delta_advances() {
        let fixture = Fixture::new();
        let md = fixture.array("md-stalled-sample", "recover", "1", "1.2");
        fs::write(md.join("sync_completed"), "1024 / 8192").unwrap();
        fs::write(md.join("sync_speed"), "4096").unwrap();
        let started = Instant::now();
        let mut sampler = MdOperationSampler::default();

        let first = collect(&fixture.0).unwrap();
        assert_eq!(
            sampler.statuses(&first, started)[0].rebuild_speed_mb,
            Some(4)
        );

        let unchanged = collect(&fixture.0).unwrap();
        let unchanged_status = sampler.statuses(&unchanged, started + Duration::from_secs(2));
        assert_eq!(unchanged_status[0].rebuild_speed_mb, Some(4));
        assert_eq!(unchanged_status[0].eta_minutes, Some(1));

        fs::write(md.join("sync_completed"), "5120 / 8192").unwrap();
        let advanced = collect(&fixture.0).unwrap();
        let advanced_status = sampler.statuses(&advanced, started + Duration::from_secs(4));
        assert_eq!(advanced_status[0].rebuild_speed_mb, Some(0));
        assert_eq!(advanced_status[0].eta_minutes, Some(1));
    }

    #[test]
    fn event_driven_short_interval_uses_kernel_speed_instead_of_delta_spike() {
        let fixture = Fixture::new();
        let md = fixture.array("md-short-window", "recover", "1", "1.2");
        fs::write(md.join("sync_completed"), "1024 / 16384").unwrap();
        fs::write(md.join("sync_speed"), "4096").unwrap();
        let started = Instant::now();
        let mut sampler = MdOperationSampler::default();

        let first = collect(&fixture.0).unwrap();
        sampler.statuses(&first, started);

        fs::write(md.join("sync_completed"), "8192 / 16384").unwrap();
        let event_sample = collect(&fixture.0).unwrap();
        let status = sampler.statuses(&event_sample, started + Duration::from_millis(150));

        assert_eq!(status[0].rebuild_speed_mb, Some(4));
        assert_eq!(sampler.previous["md-short-window"].completed_sectors, 1024);
    }

    #[test]
    fn sysfs_and_legacy_oracle_agree_on_shared_recovery_fields() {
        let fixture = Fixture::new();
        let md = fixture.array("md0", "recover", "0", "1.2");
        fs::write(md.join("sync_completed"), "1024 / 2048").unwrap();
        fs::write(md.join("sync_speed"), "1024").unwrap();
        let inventory = collect(&fixture.0).unwrap();
        let native = MdOperationSampler::default()
            .statuses(&inventory, Instant::now())
            .remove(0);
        let legacy = crate::collectors::raid::parse_mdstat(
            "md0 : active raid1 sda[0] sdb[1]\n\
             2048 blocks [2/2] [UU]\n\
             [==========>.........] recovery = 50.0% (1024/2048) finish=0.1min speed=1024K/sec\n\n",
        )
        .remove(0);

        assert_eq!(native.name, legacy.name);
        assert_eq!(native.state, legacy.state);
        assert_eq!(native.active_disks, legacy.active_disks);
        assert_eq!(native.total_disks, legacy.total_disks);
        assert_eq!(native.rebuild_pct, legacy.rebuild_pct);
        assert_eq!(native.rebuild_speed_mb, legacy.rebuild_speed_mb);
        assert_eq!(native.eta_minutes, legacy.eta_minutes);
    }
}
