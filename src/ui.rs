use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;

pub fn draw(f: &mut Frame, state: &AppState) {
    let area = f.area();

    let block = Block::default()
        .title(" VaultWatch — HDD Monitor ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let last_updated = format!(
        "Last update: {:?} ago",
        state.last_updated.elapsed()
    );

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Loading...",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Devices: {}", state.disk_devices.join(", ")),
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            last_updated,
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "q:quit   r:refresh   g:graph/table   Tab:next panel   ↑↓/jk:scroll",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(paragraph, inner);
}
