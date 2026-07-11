use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::DepError;

pub fn render_dep_error_banner(frame: &mut Frame, errors: &[DepError]) {
    if errors.is_empty() {
        return;
    }

    let area = frame.area();
    let height = (errors.len() as u16 * 2 + 4).min(area.height);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(height), Constraint::Min(0)])
        .split(area);

    let banner_area = chunks[0];
    frame.render_widget(Clear, banner_area);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            " Missing required tools:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for err in errors {
        lines.push(Line::from(vec![Span::styled(
            format!("  ✗ {}", err.tool),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(vec![Span::styled(
            format!("    Install: {}", err.install_hint),
            Style::default().fg(Color::Gray),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Some features may not work. Press 'q' to quit.",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            " Dependency Warning ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

    let para = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(para, banner_area);
}
