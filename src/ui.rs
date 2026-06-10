use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{AppState, FocusedPanel, ViewMode};
use crate::widgets::{disk_table, graph_view, raid_panel, smart_details};

const MIN_WIDTH_TABLE: u16 = 100;
const MIN_HEIGHT_TABLE: u16 = 28;
const MIN_WIDTH_GRAPH: u16 = 110;
const MIN_HEIGHT_GRAPH: u16 = 30;

pub fn draw(f: &mut Frame, state: &mut AppState) {
    let area = f.area();

    let (min_w, min_h) = if state.view_mode == ViewMode::Graph {
        (MIN_WIDTH_GRAPH, MIN_HEIGHT_GRAPH)
    } else {
        (MIN_WIDTH_TABLE, MIN_HEIGHT_TABLE)
    };

    if area.width < min_w || area.height < min_h {
        render_resize_message(f, area, min_w, min_h);
        return;
    }

    match state.view_mode {
        ViewMode::Table => render_table_view(f, area, state),
        ViewMode::Graph => render_graph_view(f, area, state),
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
        Line::from(Span::styled(
            current,
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let p = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}

fn render_table_view(f: &mut Frame, area: Rect, state: &mut AppState) {
    // Layout: header(1) + raid(4) + disk_table(fill) + status_bar(1) + smart_details(7)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // header
            Constraint::Length(4),  // RAID panel
            Constraint::Min(4),     // disk table
            Constraint::Length(1),  // status bar
            Constraint::Length(7),  // smart details
        ])
        .split(area);

    render_header(f, chunks[0], state);
    raid_panel::render(f, chunks[1], state);
    disk_table::render(f, chunks[2], state);
    render_status_bar(f, chunks[3], state);
    smart_details::render(f, chunks[4], state);
}

fn render_graph_view(f: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    render_header(f, chunks[0], state);
    graph_view::render(f, chunks[1], state);
}

fn render_header(f: &mut Frame, area: Rect, state: &AppState) {
    let view_hint = match state.view_mode {
        ViewMode::Table => "g:graph",
        ViewMode::Graph => "g:table",
    };

    let title = Span::styled(
        " VaultWatch — HDD Monitor ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    let keys = Span::styled(
        format!("  q:quit  r:refresh  {}  Tab:panel  ↑↓:scroll", view_hint),
        Style::default().fg(Color::DarkGray),
    );
    let last = Span::styled(
        format!("  Last update: {}", state.last_updated_str),
        Style::default().fg(Color::White),
    );

    let line = Line::from(vec![title, keys, last]);
    let p = Paragraph::new(line);
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
        let total = state.disks.len() + 1; // +1 for header
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
            format!("{} SmartDetails {}", smart_bullet, smart_scroll_info),
            Style::default().fg(smart_color),
        ),
    ]);

    let p = Paragraph::new(line);
    f.render_widget(p, area);
}
