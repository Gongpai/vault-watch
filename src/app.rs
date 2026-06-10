use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use ratatui::layout::Rect;

pub const HISTORY_SIZE: usize = 60;

#[derive(Debug, Clone)]
pub enum Alert {
    HighTemperature { device: String, temp: u8 },
    DiskFail { device: String },
    GrownDefects { device: String, count: u64 },
    RaidDegraded,
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
            Alert::RaidDegraded => "✗  RAID: Array DEGRADED — disk missing".to_string(),
        }
    }

    pub fn is_critical(&self) -> bool {
        matches!(self, Alert::DiskFail { .. } | Alert::RaidDegraded)
    }
}

pub fn collect_alerts(state: &AppState) -> Vec<Alert> {
    let mut alerts = Vec::new();

    if let Some(ref raid) = state.raid {
        if raid.state == RaidState::Degraded {
            alerts.push(Alert::RaidDegraded);
        }
    }

    for disk in &state.disks {
        if !disk.health_ok {
            alerts.push(Alert::DiskFail { device: disk.device.clone() });
        }
        if let Some(t) = disk.temperature_c {
            if t > 55 {
                alerts.push(Alert::HighTemperature { device: disk.device.clone(), temp: t });
            }
        }
        if let Some(d) = disk.grown_defects {
            if d > 0 {
                alerts.push(Alert::GrownDefects { device: disk.device.clone(), count: d });
            }
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
    pub health_ok: bool,
    pub power_on_hours: Option<u64>,
    pub grown_defects: Option<u64>,
    pub non_medium_errors: Option<u64>,
    pub read_errors: Option<u64>,
    pub write_errors: Option<u64>,
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
    ThroughputGraph,
    RaidGraph,
}

pub struct AppState {
    pub raid: Option<RaidStatus>,
    pub disks: Vec<DiskInfo>,
    pub io_stats: Vec<IoStats>,
    pub last_updated: Instant,
    pub last_updated_str: String,
    pub disk_devices: Vec<String>,

    pub temp_history: HashMap<String, VecDeque<u64>>,
    pub read_history: HashMap<String, VecDeque<u64>>,
    pub write_history: HashMap<String, VecDeque<u64>>,
    pub raid_speed_history: VecDeque<u64>,

    pub view_mode: ViewMode,
    pub focused_panel: FocusedPanel,
    pub disk_table_scroll: usize,
    pub smart_details_scroll: usize,
    pub graph_scroll: usize,

    pub panel_rects: HashMap<FocusedPanel, Rect>,

    pub alerts: Vec<Alert>,
    pub alert_cooldowns: HashMap<String, Instant>,
}

impl AppState {
    pub fn new(disk_devices: Vec<String>) -> Self {
        let mut temp_history = HashMap::new();
        let mut read_history = HashMap::new();
        let mut write_history = HashMap::new();

        for device in &disk_devices {
            temp_history.insert(device.clone(), VecDeque::with_capacity(HISTORY_SIZE));
            read_history.insert(device.clone(), VecDeque::with_capacity(HISTORY_SIZE));
            write_history.insert(device.clone(), VecDeque::with_capacity(HISTORY_SIZE));
        }

        Self {
            raid: None,
            disks: Vec::new(),
            io_stats: Vec::new(),
            last_updated: Instant::now(),
            last_updated_str: "--:--:--".to_string(),
            disk_devices,
            temp_history,
            read_history,
            write_history,
            raid_speed_history: VecDeque::with_capacity(HISTORY_SIZE),
            view_mode: ViewMode::Table,
            focused_panel: FocusedPanel::DiskTable,
            disk_table_scroll: 0,
            smart_details_scroll: 0,
            graph_scroll: 0,
            panel_rects: HashMap::new(),
            alerts: Vec::new(),
            alert_cooldowns: HashMap::new(),
        }
    }
}
