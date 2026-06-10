use std::sync::LazyLock;

use regex::Regex;
use tokio::fs;

use crate::app::{RaidState, RaidStatus};

static ARRAY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\w+)\s*:\s*(active|inactive)\s+\w+").unwrap());
static DISK_COUNT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[(\d+)/(\d+)\]").unwrap());
static REBUILD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[[=>\.]+\]\s+(?:resync|recovery|check|repair)\s*=\s*([\d.]+)%").unwrap()
});
static SPEED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"speed=(\d+)K/sec").unwrap());
static FINISH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"finish=([\d.]+)min").unwrap());

pub async fn collect() -> Option<RaidStatus> {
    let content = fs::read_to_string("/proc/mdstat").await.ok()?;
    parse_mdstat(&content)
}

fn parse_mdstat(content: &str) -> Option<RaidStatus> {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        if let Some(caps) = ARRAY_RE.captures(lines[i]) {
            let name = caps[1].to_string();
            let is_active_status = &caps[2] == "active";
            let mut active_disks = 0u8;
            let mut total_disks = 0u8;
            let mut rebuild_pct = None;
            let mut rebuild_speed_mb = None;
            let mut eta_minutes = None;
            let mut has_rebuild = false;

            i += 1;
            while i < lines.len() && !lines[i].trim().is_empty() {
                let line = lines[i];

                if let Some(caps) = DISK_COUNT_RE.captures(line) {
                    active_disks = caps[1].parse().unwrap_or(0);
                    total_disks = caps[2].parse().unwrap_or(0);
                }
                if let Some(caps) = REBUILD_RE.captures(line) {
                    has_rebuild = true;
                    rebuild_pct = caps[1].parse().ok();
                }
                if let Some(caps) = SPEED_RE.captures(line) {
                    let k_per_sec: u64 = caps[1].parse().unwrap_or(0);
                    rebuild_speed_mb = Some(k_per_sec / 1024);
                }
                if let Some(caps) = FINISH_RE.captures(line) {
                    let mins: f64 = caps[1].parse().unwrap_or(0.0);
                    eta_minutes = Some(mins.ceil() as u64);
                }

                i += 1;
            }

            let state = if !is_active_status {
                RaidState::Unknown
            } else if has_rebuild {
                RaidState::Rebuilding
            } else if active_disks < total_disks {
                RaidState::Degraded
            } else {
                RaidState::Active
            };

            return Some(RaidStatus {
                name,
                state,
                rebuild_pct,
                rebuild_speed_mb,
                eta_minutes,
                active_disks,
                total_disks,
            });
        }
        i += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const MDSTAT_ACTIVE: &str = "Personalities : [raid10]
md0 : active raid10 sdc[0] sdd[1] sde[2]
      11718504448 blocks super 1.2 512K chunks 2 near-copies [3/3] [UUU]

unused devices: <none>
";

    const MDSTAT_REBUILDING: &str = "Personalities : [raid10]
md0 : active raid10 sdc[0] sdd[1] sde[2]
      11718504448 blocks super 1.2 512K chunks 2 near-copies [3/3] [UUU]
      [==>..................]  resync =  9.3% (1090263040/11718504448) finish=60.5min speed=178031K/sec

unused devices: <none>
";

    const MDSTAT_DEGRADED: &str = "Personalities : [raid10]
md0 : active raid10 sdc[0] sdd[1]
      11718504448 blocks super 1.2 512K chunks 2 near-copies [2/3] [UU_]

unused devices: <none>
";

    const MDSTAT_NO_ARRAY: &str = "Personalities : [raid10]
unused devices: <none>
";

    #[test]
    fn test_active() {
        let r = parse_mdstat(MDSTAT_ACTIVE).unwrap();
        assert_eq!(r.name, "md0");
        assert_eq!(r.state, RaidState::Active);
        assert_eq!(r.active_disks, 3);
        assert_eq!(r.total_disks, 3);
        assert!(r.rebuild_pct.is_none());
        assert!(r.rebuild_speed_mb.is_none());
        assert!(r.eta_minutes.is_none());
    }

    #[test]
    fn test_rebuilding() {
        let r = parse_mdstat(MDSTAT_REBUILDING).unwrap();
        assert_eq!(r.name, "md0");
        assert_eq!(r.state, RaidState::Rebuilding);
        assert!((r.rebuild_pct.unwrap() - 9.3).abs() < 0.01);
        // 178031 / 1024 = 173
        assert_eq!(r.rebuild_speed_mb.unwrap(), 173);
        // ceil(60.5) = 61
        assert_eq!(r.eta_minutes.unwrap(), 61);
    }

    #[test]
    fn test_degraded() {
        let r = parse_mdstat(MDSTAT_DEGRADED).unwrap();
        assert_eq!(r.state, RaidState::Degraded);
        assert_eq!(r.active_disks, 2);
        assert_eq!(r.total_disks, 3);
    }

    const MDSTAT_INACTIVE: &str = "Personalities : [raid10]
md0 : inactive sdc[0](S) sdd[1](S) sde[2](S)

unused devices: <none>
";

    #[test]
    fn test_no_array() {
        assert!(parse_mdstat(MDSTAT_NO_ARRAY).is_none());
    }

    #[test]
    fn test_inactive() {
        let r = parse_mdstat(MDSTAT_INACTIVE).unwrap();
        assert_eq!(r.name, "md0");
        assert_eq!(r.state, RaidState::Unknown);
    }
}
