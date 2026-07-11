use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::{
    ExecutableCommand,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::{Mutex, Notify};
use tokio::time;

mod app;
mod collectors;
mod config;
mod notifier;
mod security;
mod storage;
mod ui;
mod widgets;

use app::{
    AppState, FocusedPanel, HISTORY_SIZE, RaidAvailability, ViewMode, collect_alerts,
    merge_inventory_disks,
};

const COLLECTOR_INTERVAL: Duration = Duration::from_secs(2);
const RENDER_INTERVAL: Duration = Duration::from_millis(250);
const PAGE_SCROLL: usize = 5;

#[tokio::main]
async fn main() -> io::Result<()> {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = stdout.execute(LeaveAlternateScreen);
        let _ = stdout.execute(DisableMouseCapture);
        default_hook(info);
    }));

    let loaded = config::load_config();
    let config_error = loaded.error;
    let cfg = Arc::new(loaded.config);

    let devices = config::resolve_devices(&cfg);
    let inventory = storage::discover_storage();
    let outbound_notifications = cfg
        .discord
        .as_ref()
        .and_then(|discord| discord.webhook_url.as_deref())
        .is_some_and(|url| !url.trim().is_empty());
    let security = security::SecurityPosture::new(outbound_notifications);
    let state = Arc::new(Mutex::new(AppState::new(devices, inventory, security)));
    let refresh_notify = Arc::new(Notify::new());
    #[cfg(target_os = "linux")]
    storage::spawn_block_event_hints(Arc::clone(&refresh_notify));

    // Startup dependency check — results stored in AppState for UI display
    let mut dep_errors = config::check_dependencies(&cfg).await;
    if let Some(error) = config_error {
        dep_errors.push(app::DepError {
            tool: "config.toml".to_string(),
            install_hint: error,
        });
    }
    {
        let mut s = state.lock().await;
        s.dep_errors = dep_errors;
    }

    let collector_state = Arc::clone(&state);
    let collector_notify = Arc::clone(&refresh_notify);
    let collector_cfg = Arc::clone(&cfg);
    tokio::spawn(async move {
        collector_loop(collector_state, collector_notify, collector_cfg).await;
    });

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let result = run_app(&mut terminal, state, refresh_notify).await;

    let _ = terminal.show_cursor();
    let _ = disable_raw_mode();
    let _ = terminal.backend_mut().execute(DisableMouseCapture);
    let _ = terminal.backend_mut().execute(LeaveAlternateScreen);

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: Arc<Mutex<AppState>>,
    refresh_notify: Arc<Notify>,
) -> io::Result<()> {
    let mut render_interval = time::interval(RENDER_INTERVAL);

    loop {
        tokio::select! {
            _ = render_interval.tick() => {
                let mut state_guard = state.lock().await;
                terminal.draw(|f| ui::draw(f, &mut state_guard))?;
            }
            event = poll_event() => {
                match event? {
                    Some(Event::Key(key))
                        if !handle_key(key, &state, &refresh_notify).await =>
                    {
                        return Ok(());
                    }
                    Some(Event::Mouse(mouse)) => {
                        handle_mouse(mouse, &state).await;
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn handle_key(
    key: KeyEvent,
    state: &Arc<Mutex<AppState>>,
    refresh_notify: &Arc<Notify>,
) -> bool {
    match key.code {
        KeyCode::Char('q') => return false,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return false,

        KeyCode::Char('r') => {
            refresh_notify.notify_one();
        }

        KeyCode::Char('g') => {
            let mut s = state.lock().await;
            s.view_mode = match s.view_mode {
                ViewMode::Table => ViewMode::Graph,
                ViewMode::Graph => ViewMode::Table,
            };
        }

        KeyCode::Tab => {
            let mut s = state.lock().await;
            s.focused_panel = if matches!(s.view_mode, ViewMode::Table) {
                match s.focused_panel {
                    FocusedPanel::DiskTable => FocusedPanel::SmartDetails,
                    _ => FocusedPanel::DiskTable,
                }
            } else {
                // RAID graph participates in the cycle only while visible
                match s.focused_panel {
                    FocusedPanel::TempGraph => FocusedPanel::ReadGraph,
                    FocusedPanel::ReadGraph => FocusedPanel::WriteGraph,
                    FocusedPanel::WriteGraph if s.raid_graph_visible() => FocusedPanel::RaidGraph,
                    _ => FocusedPanel::TempGraph,
                }
            };
        }

        KeyCode::BackTab => {
            let mut s = state.lock().await;
            s.focused_panel = if matches!(s.view_mode, ViewMode::Table) {
                match s.focused_panel {
                    FocusedPanel::SmartDetails => FocusedPanel::DiskTable,
                    _ => FocusedPanel::SmartDetails,
                }
            } else {
                match s.focused_panel {
                    FocusedPanel::ReadGraph => FocusedPanel::TempGraph,
                    FocusedPanel::WriteGraph => FocusedPanel::ReadGraph,
                    FocusedPanel::RaidGraph => FocusedPanel::WriteGraph,
                    _ if s.raid_graph_visible() => FocusedPanel::RaidGraph,
                    _ => FocusedPanel::WriteGraph,
                }
            };
        }

        KeyCode::Up | KeyCode::Char('k') => {
            let mut s = state.lock().await;
            scroll_focused(&mut s, -1);
        }

        KeyCode::Down | KeyCode::Char('j') => {
            let mut s = state.lock().await;
            scroll_focused(&mut s, 1);
        }

        KeyCode::PageUp => {
            let mut s = state.lock().await;
            scroll_focused(&mut s, -(PAGE_SCROLL as i64));
        }

        KeyCode::PageDown => {
            let mut s = state.lock().await;
            scroll_focused(&mut s, PAGE_SCROLL as i64);
        }

        KeyCode::Home => {
            let mut s = state.lock().await;
            match s.focused_panel {
                FocusedPanel::DiskTable => s.disk_table_scroll = 0,
                FocusedPanel::SmartDetails => s.smart_details_scroll = 0,
                _ => s.graph_scroll = 0,
            }
        }

        KeyCode::End => {
            let mut s = state.lock().await;
            let max = match s.focused_panel {
                FocusedPanel::DiskTable => s.disks.len().saturating_sub(1),
                FocusedPanel::SmartDetails => (s.disks.len() * 2).saturating_sub(1),
                _ => 0,
            };
            match s.focused_panel {
                FocusedPanel::DiskTable => s.disk_table_scroll = max,
                FocusedPanel::SmartDetails => s.smart_details_scroll = max,
                _ => s.graph_scroll = max,
            }
        }

        _ => {}
    }
    true
}

fn scroll_focused(s: &mut AppState, delta: i64) {
    match s.focused_panel {
        FocusedPanel::DiskTable => {
            s.disk_table_scroll = (s.disk_table_scroll as i64 + delta).max(0) as usize;
        }
        FocusedPanel::SmartDetails => {
            s.smart_details_scroll = (s.smart_details_scroll as i64 + delta).max(0) as usize;
        }
        _ => {
            s.graph_scroll = (s.graph_scroll as i64 + delta).max(0) as usize;
        }
    }
}

async fn handle_mouse(mouse: MouseEvent, state: &Arc<Mutex<AppState>>) {
    let mut s = state.lock().await;

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            // Scroll the panel under the cursor, or the focused one
            let panel = panel_at(&s, mouse.column, mouse.row);
            let target = panel.unwrap_or(s.focused_panel);
            match target {
                FocusedPanel::DiskTable => {
                    s.disk_table_scroll = s.disk_table_scroll.saturating_sub(3);
                }
                FocusedPanel::SmartDetails => {
                    s.smart_details_scroll = s.smart_details_scroll.saturating_sub(3);
                }
                _ => {
                    s.graph_scroll = s.graph_scroll.saturating_sub(3);
                }
            }
        }
        MouseEventKind::ScrollDown => {
            let panel = panel_at(&s, mouse.column, mouse.row);
            let target = panel.unwrap_or(s.focused_panel);
            match target {
                FocusedPanel::DiskTable => {
                    s.disk_table_scroll = s.disk_table_scroll.saturating_add(3);
                }
                FocusedPanel::SmartDetails => {
                    s.smart_details_scroll = s.smart_details_scroll.saturating_add(3);
                }
                _ => {
                    s.graph_scroll = s.graph_scroll.saturating_add(3);
                }
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(panel) = panel_at(&s, mouse.column, mouse.row) {
                s.focused_panel = panel;
            }
        }
        _ => {}
    }
}

/// Return which panel contains the given terminal cell (col, row), if any.
fn panel_at(s: &AppState, col: u16, row: u16) -> Option<FocusedPanel> {
    for (panel, rect) in &s.panel_rects {
        if col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
        {
            return Some(*panel);
        }
    }
    None
}

async fn poll_event() -> io::Result<Option<Event>> {
    tokio::task::spawn_blocking(|| {
        if event::poll(Duration::from_millis(50))? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    })
    .await
    .map_err(|error| io::Error::other(format!("event reader task failed: {error}")))?
}

async fn collector_loop(
    state: Arc<Mutex<AppState>>,
    notify: Arc<Notify>,
    cfg: Arc<config::Config>,
) {
    let (smartctl_prog, smartctl_base_args) = config::smartctl_base_cmd(&cfg);
    let mut native_diskstats = collectors::diskstats::DiskstatsSampler::default();
    let mut native_md = collectors::md_sysfs::MdOperationSampler::default();

    loop {
        // Netlink events and manual refreshes are hints. This bounded sysfs
        // resnapshot remains the correctness path, with the periodic timer as
        // a safety net for missed or unavailable events.
        let next_inventory = storage::discover_storage();
        let (devices, throughput_subjects) = {
            let mut s = state.lock().await;
            s.reconcile_storage(next_inventory);
            let subjects = s
                .storage_inventory
                .throughput_subjects()
                .into_iter()
                .map(|subject| collectors::diskstats::DiskstatsSubject {
                    name: subject.name,
                    dev_t: subject.dev_t,
                    diskseq: subject.diskseq,
                })
                .collect::<Vec<_>>();
            (s.disk_devices.clone(), subjects)
        };
        let io_result = native_diskstats
            .sample(
                Path::new("/proc/diskstats"),
                Instant::now(),
                &throughput_subjects,
            )
            .unwrap_or_default()
            .into_iter()
            .map(|(device, metrics)| app::IoStats {
                device,
                read_mb_s: metrics.read_mib_per_sec,
                write_mb_s: metrics.write_mib_per_sec,
                read_iops: metrics.read_iops,
                write_iops: metrics.write_iops,
                utilization_percent: metrics.utilization_percent,
                average_read_latency_ms: metrics.average_read_latency_ms,
                average_write_latency_ms: metrics.average_write_latency_ms,
                average_queue_depth: metrics.average_queue_depth,
                ios_in_progress: metrics.ios_in_progress,
                source: app::IoMetricSource::ProcDiskstats,
                scope: app::IoMetricScope::DirectWholeDevice,
            })
            .collect::<Vec<_>>();
        let (raid_result, raid_availability) =
            match collectors::md_sysfs::collect(Path::new("/sys/class/block")) {
                Ok(inventory) => {
                    let availability = if inventory.partial {
                        RaidAvailability::Partial
                    } else {
                        RaidAvailability::Complete
                    };
                    (native_md.statuses(&inventory, Instant::now()), availability)
                }
                Err(_) => (Vec::new(), RaidAvailability::Unavailable),
            };

        let disks_result =
            collectors::smart::collect_all(&devices, &smartctl_prog, &smartctl_base_args).await;

        {
            let mut s = state.lock().await;
            s.reconcile_raids(raid_result, raid_availability);

            for disk in &disks_result {
                if let Some(temp) = disk.temperature_c
                    && let Some(buf) = s.temp_history.get_mut(&disk.device)
                {
                    buf.push_back(temp as u64);
                    if buf.len() > HISTORY_SIZE {
                        buf.pop_front();
                    }
                }
            }
            for stat in &io_result {
                if let Some(buf) = s.read_history.get_mut(&stat.device) {
                    buf.push_back((stat.read_mb_s * 10.0) as u64);
                    if buf.len() > HISTORY_SIZE {
                        buf.pop_front();
                    }
                }
                if let Some(buf) = s.write_history.get_mut(&stat.device) {
                    buf.push_back((stat.write_mb_s * 10.0) as u64);
                    if buf.len() > HISTORY_SIZE {
                        buf.pop_front();
                    }
                }
            }
            // Per-array rebuild speed, stored ×10 for consistency with
            // read/write history (supports 0.1 MB/s precision). Arrays without
            // an active rebuild push 0 so every line shares the same time axis.
            let raid_speeds: Vec<(String, u64)> = s
                .raids
                .iter()
                .map(|raid| (raid.name.clone(), raid.rebuild_speed_mb.unwrap_or(0) * 10))
                .collect();
            for (name, speed) in raid_speeds {
                let buf = s
                    .raid_speed_history
                    .entry(name)
                    .or_insert_with(|| VecDeque::with_capacity(HISTORY_SIZE));
                buf.push_back(speed);
                if buf.len() > HISTORY_SIZE {
                    buf.pop_front();
                }
            }
            // Arrays that vanished from mdstat (stopped) keep flowing zeros
            // until their history drains, then get dropped.
            let current: Vec<String> = s.raids.iter().map(|r| r.name.clone()).collect();
            s.raid_speed_history.retain(|name, buf| {
                if !current.contains(name) {
                    buf.push_back(0);
                    if buf.len() > HISTORY_SIZE {
                        buf.pop_front();
                    }
                }
                current.contains(name) || buf.iter().any(|&v| v > 0)
            });

            s.disks = merge_inventory_disks(&s.storage_inventory, disks_result);
            s.io_stats = io_result;
            s.last_updated = std::time::Instant::now();
            s.last_updated_str = chrono::Local::now().format("%H:%M:%S").to_string();
        }

        // Compute alerts and snapshot cooldowns (single lock, no HTTP yet)
        let (alerts, cooldowns) = {
            let mut s = state.lock().await;
            let alerts = collect_alerts(&s);
            let cooldowns = s.alert_cooldowns.clone();
            s.alerts = alerts.clone();
            (alerts, cooldowns)
        };

        // Send Discord notifications without holding the lock
        let updated_cooldowns = notifier::process_alerts(&alerts, &cooldowns, &cfg).await;

        {
            let mut s = state.lock().await;
            s.alert_cooldowns = updated_cooldowns;
        }

        tokio::select! {
            _ = time::sleep(COLLECTOR_INTERVAL) => {}
            _ = notify.notified() => {}
        }
    }
}
