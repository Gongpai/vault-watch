use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

use crate::app::{AppState, FocusedPanel};

pub fn render(f: &mut Frame, area: Rect, state: &mut AppState) {
    let focused = state.focused_panel == FocusedPanel::SmartDetails;
    let (border_type, border_color) = if focused {
        (BorderType::Double, Color::Cyan)
    } else {
        (BorderType::Plain, Color::White)
    };

    let block = Block::default()
        .title(" SMART Details ")
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color));

    state.panel_rects.insert(FocusedPanel::SmartDetails, area);

    let inner = block.inner(area);

    // One labeled line per disk (no separate header — labels are inline per row)
    let mut lines: Vec<Line> = Vec::new();

    for disk in &state.disks {
        let serial = disk.serial.as_deref().unwrap_or("--");

        let hours_str = match disk.power_on_hours {
            Some(h) => format!("{:>7}", h),
            None => format!("{:>7}", "--"),
        };

        let (nme_str, nme_color) = match disk.non_medium_errors {
            Some(n) if n > 1000 => (format!("{:>6}", n), Color::Yellow),
            Some(n) => (format!("{:>6}", n), Color::White),
            None => (format!("{:>6}", "--"), Color::White),
        };

        let (defect_str, defect_color) = match disk.grown_defects {
            Some(d) if d > 0 => (format!("WARN {:>2}", d), Color::Red),
            Some(d) => (format!("{:>7}", d), Color::White),
            None => (format!("{:>7}", "--"), Color::White),
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<5}", disk.device),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" Serial: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{:<20}", serial)),
            Span::styled("  Hours: ", Style::default().fg(Color::DarkGray)),
            Span::styled(hours_str, Style::default().fg(Color::White)),
            Span::styled("  NME: ", Style::default().fg(Color::DarkGray)),
            Span::styled(nme_str, Style::default().fg(nme_color)),
            Span::styled("  Defects: ", Style::default().fg(Color::DarkGray)),
            Span::styled(defect_str, Style::default().fg(defect_color)),
        ]));
    }

    let total_lines = lines.len();
    let visible = inner.height as usize;
    let max_scroll = total_lines.saturating_sub(visible);
    let scroll = state.smart_details_scroll.min(max_scroll);
    state.smart_details_scroll = scroll;

    // Correct order: block border first, content on top
    f.render_widget(block, area);
    let paragraph = Paragraph::new(lines).scroll((scroll as u16, 0));
    f.render_widget(paragraph, inner);

    // Scrollbar
    if total_lines > visible {
        let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

    // Overflow hint
    let remaining = total_lines.saturating_sub(scroll + visible);
    if remaining > 0 {
        let hint = format!("↓ {} more", remaining);
        let hint_x = area.x + area.width.saturating_sub(hint.len() as u16 + 2);
        let hint_y = area.y + area.height - 1;
        let hint_area = Rect::new(hint_x, hint_y, hint.len() as u16, 1);
        let p = Paragraph::new(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(p, hint_area);
    }
}
