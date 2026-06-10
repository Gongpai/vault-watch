use std::io;
use std::sync::Arc;
use std::time::Duration;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::{Mutex, Notify};
use tokio::time;

mod app;
mod collectors;
mod ui;
mod widgets;

use app::{AppState, FocusedPanel, ViewMode, HISTORY_SIZE};

const DISK_DEVICES: &[&str] = &["sdc", "sdd", "sde"];
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

    let state = Arc::new(Mutex::new(AppState::new(
        DISK_DEVICES.iter().map(|s| s.to_string()).collect(),
    )));
    let refresh_notify = Arc::new(Notify::new());

    let collector_state = Arc::clone(&state);
    let collector_notify = Arc::clone(&refresh_notify);
    tokio::spawn(async move {
        collector_loop(collector_state, collector_notify).await;
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
                match s.focused_panel {
                    FocusedPanel::TempGraph => FocusedPanel::ThroughputGraph,
                    FocusedPanel::ThroughputGraph => FocusedPanel::RaidGraph,
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
                    FocusedPanel::ThroughputGraph => FocusedPanel::TempGraph,
                    FocusedPanel::RaidGraph => FocusedPanel::ThroughputGraph,
                    _ => FocusedPanel::RaidGraph,
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
                FocusedPanel::SmartDetails => (s.disks.len() + 1).saturating_sub(1),
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
        if col >= rect.x
            && col < rect.x + rect.width
            && row >= rect.y
            && row < rect.y + rect.height
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
    .unwrap()
}

async fn collector_loop(state: Arc<Mutex<AppState>>, notify: Arc<Notify>) {
    loop {
        let devices = {
            let s = state.lock().await;
            s.disk_devices.clone()
        };

        let (raid_result, disks_result, iostat_result) = tokio::join!(
            collectors::raid::collect(),
            collectors::smart::collect_all(&devices),
            collectors::iostat::collect(&devices),
        );

        {
            let mut s = state.lock().await;

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
            for stat in &iostat_result {
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
            // Store ×10 for consistency with read/write history (supports 0.1 MB/s precision)
            let raid_speed = raid_result.as_ref().and_then(|r| r.rebuild_speed_mb).unwrap_or(0) * 10;
            s.raid_speed_history.push_back(raid_speed);
            if s.raid_speed_history.len() > HISTORY_SIZE {
                s.raid_speed_history.pop_front();
            }

            s.raid = raid_result;
            s.disks = disks_result;
            s.io_stats = iostat_result;
            s.last_updated = std::time::Instant::now();
            s.last_updated_str = chrono::Local::now().format("%H:%M:%S").to_string();
        }

        tokio::select! {
            _ = time::sleep(COLLECTOR_INTERVAL) => {}
            _ = notify.notified() => {}
        }
    }
}
