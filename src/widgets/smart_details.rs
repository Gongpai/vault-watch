use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};

use crate::app::{Alert, AppState, FocusedPanel};

pub fn render(f: &mut Frame, area: Rect, state: &mut AppState) {
    let focused = state.focused_panel == FocusedPanel::SmartDetails;
    let has_critical = state
        .alerts
        .iter()
        .any(|a| matches!(a, Alert::DiskFail { .. }));
    let has_warn = state.alerts.iter().any(|a| {
        matches!(
            a,
            Alert::GrownDefects { .. } | Alert::HighTemperature { .. }
        )
    });
    let (border_type, border_color) = if focused {
        (BorderType::Double, Color::Cyan)
    } else if has_critical {
        (BorderType::Plain, Color::Red)
    } else if has_warn {
        (BorderType::Plain, Color::Yellow)
    } else {
        (BorderType::Plain, Color::White)
    };

    let block = Block::default()
        .title(" Device Details · health + block I/O ")
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color));

    state.panel_rects.insert(FocusedPanel::SmartDetails, area);

    let inner = block.inner(area);

    // Two labeled lines per device: health metadata and scoped block I/O.
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

        let rw_err = |v: Option<u64>| match v {
            Some(n) if n > 0 => (format!("{:>5}", n), Color::Yellow),
            Some(n) => (format!("{:>5}", n), Color::White),
            None => (format!("{:>5}", "--"), Color::White),
        };
        let (rerr_str, rerr_color) = rw_err(disk.read_errors);
        let (werr_str, werr_color) = rw_err(disk.write_errors);

        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<12}", disk.device),
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
            Span::styled("  RdErr: ", Style::default().fg(Color::DarkGray)),
            Span::styled(rerr_str, Style::default().fg(rerr_color)),
            Span::styled("  WrErr: ", Style::default().fg(Color::DarkGray)),
            Span::styled(werr_str, Style::default().fg(werr_color)),
        ]));

        let io_line = match state
            .io_stats
            .iter()
            .find(|stat| stat.device == disk.device)
        {
            Some(stat) => Line::from(vec![
                Span::styled(format!("{:<12}", ""), Style::default()),
                Span::styled(
                    format!(" I/O [{}/{}] ", stat.source.label(), stat.scope.label()),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(format!(
                    "RIOPS {:>6.1} WIOPS {:>6.1} U {:>5.1}% RL {:>7} WL {:>7} QD {:>5.2} A {:>3}",
                    stat.read_iops,
                    stat.write_iops,
                    stat.utilization_percent,
                    format_latency(stat.average_read_latency_ms),
                    format_latency(stat.average_write_latency_ms),
                    stat.average_queue_depth,
                    stat.ios_in_progress,
                )),
            ]),
            None => Line::from(vec![
                Span::styled(format!("{:<12}", ""), Style::default()),
                Span::styled(
                    " I/O [diskstats/whole] unavailable (baseline/reset/device absent)",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        };
        lines.push(io_line);
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
        let p = Paragraph::new(Span::styled(hint, Style::default().fg(Color::DarkGray)));
        f.render_widget(p, hint_area);
    }
}

fn format_latency(value: Option<f64>) -> String {
    value
        .map(|milliseconds| format!("{milliseconds:.2}ms"))
        .unwrap_or_else(|| "N/A".to_string())
}

#[cfg(test)]
mod tests {
    use super::format_latency;

    #[test]
    fn idle_latency_is_unavailable_not_zero() {
        assert_eq!(format_latency(None), "N/A");
        assert_eq!(format_latency(Some(0.0)), "0.00ms");
    }
}
