use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

use ratatui::layout::Rect;

use crate::security::SecurityPosture;
use crate::storage::StorageInventory;

pub const HISTORY_SIZE: usize = 60;

#[derive(Debug, Clone)]
pub enum Alert {
    HighTemperature { device: String, temp: u8 },
    DiskFail { device: String },
    GrownDefects { device: String, count: u64 },
    RaidDegraded { array: String },
}

impl Alert {
    pub fn message(&self) -> String {
        match self {
            Alert::HighTemperature { device, temp } => {
                format!("⚠  {device}: Temperature {temp}°C exceeds 55°C")
            }
            Alert::DiskFail { device } => format!("✗  {device}: SMART health FAIL"),
            Alert::GrownDefects { device, count } => {
                format!("⚠  {device}: Grown defects = {count}")
            }
            Alert::RaidDegraded { array } => {
                format!("✗  RAID: {array} DEGRADED — disk missing")
            }
        }
    }

    pub fn is_critical(&self) -> bool {
        matches!(self, Alert::DiskFail { .. } | Alert::RaidDegraded { .. })
    }
}

#[derive(Debug, Clone)]
pub struct DepError {
    pub tool: String,
    pub install_hint: String,
}

pub fn collect_alerts(state: &AppState) -> Vec<Alert> {
    let mut alerts = Vec::new();

    for raid in &state.raids {
        if raid.state == RaidState::Degraded {
            alerts.push(Alert::RaidDegraded {
                array: raid.name.clone(),
            });
        }
    }

    for disk in &state.disks {
        if disk.health == HealthStatus::Failed {
            alerts.push(Alert::DiskFail {
                device: disk.device.clone(),
            });
        }
        if let Some(t) = disk.temperature_c
            && t > 55
        {
            alerts.push(Alert::HighTemperature {
                device: disk.device.clone(),
                temp: t,
            });
        }
        if let Some(d) = disk.grown_defects
            && d > 0
        {
            alerts.push(Alert::GrownDefects {
                device: disk.device.clone(),
                count: d,
            });
        }
    }

    alerts
}

#[derive(Debug, Clone, PartialEq)]
pub enum RaidState {
    Active,
    Rebuilding,
    Degraded,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct RaidStatus {
    pub name: String,
    pub state: RaidState,
    pub rebuild_pct: Option<f64>,
    pub rebuild_speed_mb: Option<u64>,
    pub eta_minutes: Option<u64>,
    pub active_disks: u8,
    pub total_disks: u8,
}

#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub device: String,
    pub serial: Option<String>,
    pub temperature_c: Option<u8>,
    pub health: HealthStatus,
    pub power_on_hours: Option<u64>,
    pub grown_defects: Option<u64>,
    pub non_medium_errors: Option<u64>,
    pub read_errors: Option<u64>,
    pub write_errors: Option<u64>,
}

impl DiskInfo {
    pub fn unavailable(device: String) -> Self {
        Self {
            device,
            serial: None,
            temperature_c: None,
            health: HealthStatus::Unavailable,
            power_on_hours: None,
            grown_defects: None,
            non_medium_errors: None,
            read_errors: None,
            write_errors: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Failed,
    Unavailable,
}

pub fn merge_inventory_disks(
    inventory: &StorageInventory,
    mut collected: Vec<DiskInfo>,
) -> Vec<DiskInfo> {
    for device in inventory.whole_device_names() {
        if !collected.iter().any(|disk| disk.device == device) {
            collected.push(DiskInfo::unavailable(device));
        }
    }
    collected.sort_by(|left, right| left.device.cmp(&right.device));
    collected
}

#[derive(Debug, Clone)]
pub struct IoStats {
    pub device: String,
    pub read_mb_s: f64,
    pub write_mb_s: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Table,
    Graph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusedPanel {
    DiskTable,
    SmartDetails,
    TempGraph,
    ReadGraph,
    WriteGraph,
    RaidGraph,
}

pub struct AppState {
    pub storage_inventory: StorageInventory,
    pub security: SecurityPosture,
    pub raids: Vec<RaidStatus>,
    pub disks: Vec<DiskInfo>,
    pub io_stats: Vec<IoStats>,
    pub last_updated: Instant,
    pub last_updated_str: String,
    pub disk_devices: Vec<String>,

    pub temp_history: HashMap<String, VecDeque<u64>>,
    pub read_history: HashMap<String, VecDeque<u64>>,
    pub write_history: HashMap<String, VecDeque<u64>>,
    /// Rebuild speed history per array name (×10 scale, like read/write).
    pub raid_speed_history: HashMap<String, VecDeque<u64>>,

    pub view_mode: ViewMode,
    pub focused_panel: FocusedPanel,
    pub disk_table_scroll: usize,
    pub smart_details_scroll: usize,
    pub graph_scroll: usize,

    pub panel_rects: HashMap<FocusedPanel, Rect>,

    pub alerts: Vec<Alert>,
    pub alert_cooldowns: HashMap<String, Instant>,
    pub dep_errors: Vec<DepError>,
}

impl AppState {
    pub fn new(
        disk_devices: Vec<String>,
        storage_inventory: StorageInventory,
        security: SecurityPosture,
    ) -> Self {
        let mut temp_history = HashMap::new();
        let mut read_history = HashMap::new();
        let mut write_history = HashMap::new();

        for device in &disk_devices {
            temp_history.insert(device.clone(), VecDeque::with_capacity(HISTORY_SIZE));
            read_history.insert(device.clone(), VecDeque::with_capacity(HISTORY_SIZE));
            write_history.insert(device.clone(), VecDeque::with_capacity(HISTORY_SIZE));
        }

        Self {
            storage_inventory,
            security,
            raids: Vec::new(),
            disks: Vec::new(),
            io_stats: Vec::new(),
            last_updated: Instant::now(),
            last_updated_str: "--:--:--".to_string(),
            disk_devices,
            temp_history,
            read_history,
            write_history,
            raid_speed_history: HashMap::new(),
            view_mode: ViewMode::Table,
            focused_panel: FocusedPanel::DiskTable,
            disk_table_scroll: 0,
            smart_details_scroll: 0,
            graph_scroll: 0,
            panel_rects: HashMap::new(),
            alerts: Vec::new(),
            alert_cooldowns: HashMap::new(),
            dep_errors: Vec::new(),
        }
    }

    /// The RAID graph panel is shown while any array is rebuilding, or while
    /// recent rebuild history is still draining out of the chart window —
    /// the delay keeps the layout from flickering when a rebuild finishes.
    pub fn raid_graph_visible(&self) -> bool {
        self.raids.iter().any(|r| r.state == RaidState::Rebuilding)
            || self
                .raid_speed_history
                .values()
                .any(|h| h.iter().any(|&v| v > 0))
    }

    pub fn reconcile_storage(&mut self, next: StorageInventory) {
        let replaced = self.storage_inventory.replaced_device_names(&next);
        self.storage_inventory.reconcile(next);
        let devices = self.storage_inventory.whole_device_names();
        let active: HashSet<&str> = devices.iter().map(String::as_str).collect();
        self.temp_history
            .retain(|device, _| active.contains(device.as_str()));
        self.read_history
            .retain(|device, _| active.contains(device.as_str()));
        self.write_history
            .retain(|device, _| active.contains(device.as_str()));
        for device in &replaced {
            self.temp_history.remove(device);
            self.read_history.remove(device);
            self.write_history.remove(device);
        }
        for device in devices {
            self.temp_history
                .entry(device.clone())
                .or_insert_with(|| VecDeque::with_capacity(HISTORY_SIZE));
            self.read_history
                .entry(device.clone())
                .or_insert_with(|| VecDeque::with_capacity(HISTORY_SIZE));
            self.write_history
                .entry(device)
                .or_insert_with(|| VecDeque::with_capacity(HISTORY_SIZE));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(health: HealthStatus) -> DiskInfo {
        let mut disk = DiskInfo::unavailable("sda".into());
        disk.health = health;
        disk
    }

    #[test]
    fn unavailable_health_does_not_create_disk_failure_alert() {
        let mut state = AppState::new(
            vec!["sda".into()],
            StorageInventory::default(),
            SecurityPosture::new(false),
        );
        state.disks = vec![disk(HealthStatus::Unavailable)];

        assert!(collect_alerts(&state).is_empty());
    }

    #[test]
    fn explicit_health_failure_creates_critical_alert() {
        let mut state = AppState::new(
            vec!["sda".into()],
            StorageInventory::default(),
            SecurityPosture::new(false),
        );
        state.disks = vec![disk(HealthStatus::Failed)];

        let alerts = collect_alerts(&state);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].is_critical());
    }

    #[test]
    fn merge_preserves_uncollected_inventory_devices_as_unavailable() {
        use crate::storage::{Generation, Materialization, StorageKind, StorageNode};

        let inventory = StorageInventory {
            nodes: vec![StorageNode {
                id: "block:nvme0n1".into(),
                name: "nvme0n1".into(),
                kind: StorageKind::Nvme,
                materialization: Materialization::BlockDevice,
                removable: Some(false),
                identities: Vec::new(),
                generation: Generation::default(),
            }],
            edges: Vec::new(),
            partial: false,
        };

        let merged = merge_inventory_disks(&inventory, vec![disk(HealthStatus::Healthy)]);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[1].device, "sda");
        assert_eq!(merged[0].device, "nvme0n1");
        assert_eq!(merged[0].health, HealthStatus::Unavailable);
    }

    #[test]
    fn reconciliation_prepares_histories_for_hot_added_whole_device() {
        use crate::storage::{Generation, Materialization, StorageKind, StorageNode};

        let mut state = AppState::new(
            Vec::new(),
            StorageInventory::default(),
            SecurityPosture::new(false),
        );
        state.reconcile_storage(StorageInventory {
            nodes: vec![StorageNode {
                id: "block:nvme0n1".into(),
                name: "nvme0n1".into(),
                kind: StorageKind::Nvme,
                materialization: Materialization::BlockDevice,
                removable: Some(false),
                identities: Vec::new(),
                generation: Generation::default(),
            }],
            edges: Vec::new(),
            partial: false,
        });

        assert!(state.temp_history.contains_key("nvme0n1"));
        assert!(state.read_history.contains_key("nvme0n1"));
        assert!(state.write_history.contains_key("nvme0n1"));

        state.temp_history.get_mut("nvme0n1").unwrap().push_back(42);
        state.reconcile_storage(StorageInventory {
            nodes: vec![StorageNode {
                id: "block:nvme0n1".into(),
                name: "nvme0n1".into(),
                kind: StorageKind::Nvme,
                materialization: Materialization::BlockDevice,
                removable: Some(false),
                identities: Vec::new(),
                generation: Generation {
                    diskseq: Some(2),
                    dev_t: Some((259, 0)),
                },
            }],
            edges: Vec::new(),
            partial: false,
        });

        assert!(state.temp_history["nvme0n1"].is_empty());

        state.reconcile_storage(StorageInventory::default());

        assert!(!state.temp_history.contains_key("nvme0n1"));
        assert!(!state.read_history.contains_key("nvme0n1"));
        assert!(!state.write_history.contains_key("nvme0n1"));
    }
}
