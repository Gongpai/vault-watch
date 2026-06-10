use std::io;
use std::sync::Arc;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
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

use app::{AppState, FocusedPanel, ViewMode};

const DISK_DEVICES: &[&str] = &["sdc", "sdd", "sde"];
const COLLECTOR_INTERVAL: Duration = Duration::from_secs(2);
const RENDER_INTERVAL: Duration = Duration::from_millis(250);

#[tokio::main]
async fn main() -> io::Result<()> {
    // Restore terminal if a panic occurs before cleanup runs
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = stdout.execute(LeaveAlternateScreen);
        default_hook(info);
    }));

    let state = Arc::new(Mutex::new(AppState::new(
        DISK_DEVICES.iter().map(|s| s.to_string()).collect(),
    )));
    let refresh_notify = Arc::new(Notify::new());

    // Collector task runs independently on its own tokio thread
    let collector_state = Arc::clone(&state);
    let collector_notify = Arc::clone(&refresh_notify);
    tokio::spawn(async move {
        collector_loop(collector_state, collector_notify).await;
    });

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let result = run_app(&mut terminal, state, refresh_notify).await;

    // Always restore terminal on clean exit
    let _ = terminal.show_cursor();
    let _ = disable_raw_mode();
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
                let state_guard = state.lock().await;
                terminal.draw(|f| ui::draw(f, &state_guard))?;
            }
            event = poll_event() => {
                if let Some(Event::Key(key)) = event?
                    && !handle_key(key, &state, &refresh_notify).await {
                    return Ok(());
                }
            }
        }
    }
}

// Returns false when the app should quit
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
            match s.focused_panel {
                FocusedPanel::DiskTable => {
                    s.disk_table_scroll = s.disk_table_scroll.saturating_sub(1);
                }
                FocusedPanel::SmartDetails => {
                    s.smart_details_scroll = s.smart_details_scroll.saturating_sub(1);
                }
                _ => {
                    s.graph_scroll = s.graph_scroll.saturating_sub(1);
                }
            }
        }

        KeyCode::Down | KeyCode::Char('j') => {
            let mut s = state.lock().await;
            match s.focused_panel {
                FocusedPanel::DiskTable => {
                    s.disk_table_scroll = s.disk_table_scroll.saturating_add(1);
                }
                FocusedPanel::SmartDetails => {
                    s.smart_details_scroll = s.smart_details_scroll.saturating_add(1);
                }
                _ => {
                    s.graph_scroll = s.graph_scroll.saturating_add(1);
                }
            }
        }

        _ => {}
    }
    true
}

// Poll for a crossterm event without blocking the async runtime
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

// Placeholder collector loop — replaced by real collectors in US-MON-01/02/03
async fn collector_loop(state: Arc<Mutex<AppState>>, notify: Arc<Notify>) {
    loop {
        {
            let mut s = state.lock().await;
            s.last_updated = std::time::Instant::now();
        }

        // Wait for 2s timer OR immediate wakeup from 'r' key
        tokio::select! {
            _ = time::sleep(COLLECTOR_INTERVAL) => {}
            _ = notify.notified() => {}
        }
    }
}
