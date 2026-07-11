use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{AppState, FocusedPanel, ViewMode};
use crate::storage::StorageKind;
use crate::widgets::{
    disk_table, error_screen, graph_view, raid_panel, smart_details, topology_view,
};

const MIN_WIDTH_TABLE: u16 = 100;
const MIN_HEIGHT_TABLE: u16 = 28;
const MIN_WIDTH_GRAPH: u16 = 110;
const MIN_HEIGHT_GRAPH: u16 = 30;
const MIN_WIDTH_TOPOLOGY: u16 = 100;
const MIN_HEIGHT_TOPOLOGY: u16 = 20;

pub fn draw(f: &mut Frame, state: &mut AppState) {
    let area = f.area();

    let (min_w, min_h) = match state.view_mode {
        ViewMode::Table => (MIN_WIDTH_TABLE, MIN_HEIGHT_TABLE),
        ViewMode::Graph => (MIN_WIDTH_GRAPH, MIN_HEIGHT_GRAPH),
        ViewMode::Topology => (MIN_WIDTH_TOPOLOGY, MIN_HEIGHT_TOPOLOGY),
    };

    if area.width < min_w || area.height < min_h {
        render_resize_message(f, area, min_w, min_h);
        return;
    }

    match state.view_mode {
        ViewMode::Table => render_table_view(f, area, state),
        ViewMode::Graph => render_graph_view(f, area, state),
        ViewMode::Topology => render_topology_view(f, area, state),
    }

    if !state.dep_errors.is_empty() {
        error_screen::render_dep_error_banner(f, &state.dep_errors);
    }
}

fn render_resize_message(f: &mut Frame, area: Rect, min_w: u16, min_h: u16) {
    let msg = format!(
        " Terminal too small — resize to at least {}×{} ",
        min_w, min_h
    );
    let current = format!(" Current: {}×{} ", area.width, area.height);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            msg,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(current, Style::default().fg(Color::DarkGray))),
    ];

    let p = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn alert_banner_height(alert_count: usize) -> u16 {
    if alert_count == 0 {
        0
    } else {
        (alert_count.min(2) + 2) as u16
    }
}

fn render_table_view(f: &mut Frame, area: Rect, state: &mut AppState) {
    let alert_h = alert_banner_height(state.alerts.len());

    let mut constraints: Vec<Constraint> = vec![
        Constraint::Length(1), // header
        Constraint::Length(1), // privacy and capability disclosure
    ];
    if alert_h > 0 {
        constraints.push(Constraint::Length(alert_h));
    }
    constraints.extend([
        Constraint::Length(4), // RAID panel
        Constraint::Min(4),    // disk table
        Constraint::Length(1), // status bar
        Constraint::Length(7), // smart details
        Constraint::Length(1), // key hint bar
    ]);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut idx = 0;
    render_header(f, chunks[idx], state);
    idx += 1;
    render_security_bar(f, chunks[idx], state);
    idx += 1;
    if alert_h > 0 {
        render_alert_banner(f, chunks[idx], state);
        idx += 1;
    }
    raid_panel::render(f, chunks[idx], state);
    idx += 1;
    disk_table::render(f, chunks[idx], state);
    idx += 1;
    render_status_bar(f, chunks[idx], state);
    idx += 1;
    smart_details::render(f, chunks[idx], state);
    idx += 1;
    render_key_bar(f, chunks[idx], state);
}

fn render_graph_view(f: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(f, chunks[0], state);
    render_security_bar(f, chunks[1], state);
    graph_view::render(f, chunks[2], state);
    render_key_bar(f, chunks[3], state);
}

fn render_topology_view(f: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(f, chunks[0], state);
    render_security_bar(f, chunks[1], state);
    topology_view::render(f, chunks[2], state);
    render_key_bar(f, chunks[3], state);
}

fn render_header(f: &mut Frame, area: Rect, state: &AppState) {
    let title = Span::styled(
        " VaultWatch — Storage Monitor ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    let last = Span::styled(
        format!("  Last update: {}", state.last_updated_str),
        Style::default().fg(Color::White),
    );

    let line = Line::from(vec![title, last]);
    let p = Paragraph::new(line);
    f.render_widget(p, area);
}

fn render_security_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let inventory = &state.storage_inventory;
    let nodes = inventory.nodes.len();
    let whole = inventory.whole_block_count();
    let partitions = inventory.partition_count();
    let virtual_nodes = inventory.virtual_count();
    let nvme = inventory.count_whole_kind(StorageKind::Nvme);
    let mmc = inventory.count_whole_kind(StorageKind::Mmc);
    let removable = inventory.removable_count();
    let disclosure = if area.width >= 150 {
        state.security.disclosure()
    } else {
        state.security.compact_disclosure()
    };
    let summary = if area.width >= 150 {
        format!(
            " · nodes {nodes} · whole {whole} · NVMe {nvme} · MMC {mmc} · part {partitions} · virtual {virtual_nodes} · removable {removable}"
        )
    } else {
        format!(" · W:{whole} N:{nvme} P:{partitions} V:{virtual_nodes} R:{removable}")
    };
    let partial = if inventory.partial { " · PARTIAL" } else { "" };
    let line = Line::from(vec![
        Span::styled(
            " Privacy: ",
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Span::styled(disclosure, Style::default().fg(Color::Green)),
        Span::styled(
            format!("{partial}{summary}"),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_alert_banner(f: &mut Frame, area: Rect, state: &AppState) {
    let total = state.alerts.len();
    let title = if total > 2 {
        format!(" ⚠ Alerts ({total}) ")
    } else {
        " ⚠ Alerts ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(area);

    let lines: Vec<Line> = state
        .alerts
        .iter()
        .take(2)
        .map(|alert| {
            let color = if alert.is_critical() {
                Color::Red
            } else {
                Color::Yellow
            };
            Line::from(Span::styled(
                alert.message(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ))
        })
        .collect();

    let p = Paragraph::new(lines);
    f.render_widget(block, area);
    f.render_widget(p, inner);
}

fn render_key_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let graph_label = match state.view_mode {
        ViewMode::Table => "Graph",
        ViewMode::Graph => "Table",
        ViewMode::Topology => "Graph",
    };
    let topology_label = if state.view_mode == ViewMode::Topology {
        "Table"
    } else {
        "Topology"
    };

    let mut hints: Vec<(&str, &str)> = vec![
        ("q", "Quit"),
        ("r", "Refresh"),
        ("g", graph_label),
        ("t", topology_label),
        ("Tab", "Panel"),
        ("↑↓", "Scroll"),
        ("PgU/D", "Page"),
    ];
    if !matches!(state.view_mode, ViewMode::Graph) {
        hints.push(("Home/End", "Jump"));
    }

    let key_style = Style::default().fg(Color::Black).bg(Color::Cyan);
    let action_style = Style::default().fg(Color::DarkGray);

    let mut spans: Vec<Span> = Vec::new();
    for (key, action) in &hints {
        spans.push(Span::styled(format!(" {} ", key), key_style));
        spans.push(Span::styled(format!(" {}  ", action), action_style));
    }

    let p = Paragraph::new(Line::from(spans));
    f.render_widget(p, area);
}

fn render_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let disk_focused = state.focused_panel == FocusedPanel::DiskTable;
    let smart_focused = state.focused_panel == FocusedPanel::SmartDetails;

    let disk_scroll_info = if disk_focused {
        let total = state.disks.len();
        let scroll = state.disk_table_scroll;
        format!("[{}/{} — ↑↓:scroll]", scroll + 1, total.max(1))
    } else {
        String::new()
    };

    let smart_scroll_info = if smart_focused {
        let total = state.disks.len() * 2;
        let scroll = state.smart_details_scroll;
        format!("[{}/{} — ↑↓:scroll]", scroll + 1, total.max(1))
    } else {
        String::new()
    };

    let (disk_bullet, disk_color) = if disk_focused {
        ("●", Color::Cyan)
    } else {
        ("○", Color::DarkGray)
    };
    let (smart_bullet, smart_color) = if smart_focused {
        ("●", Color::Cyan)
    } else {
        ("○", Color::DarkGray)
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" {} DiskTable {}", disk_bullet, disk_scroll_info),
            Style::default().fg(disk_color),
        ),
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("{} DeviceDetails {}", smart_bullet, smart_scroll_info),
            Style::default().fg(smart_color),
        ),
    ]);

    let p = Paragraph::new(line);
    f.render_widget(p, area);
}
