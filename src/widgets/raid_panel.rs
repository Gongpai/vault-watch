use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

fn format_eta(minutes: u64) -> String {
    if minutes >= 60 {
        format!("{}h {}m", minutes / 60, minutes % 60)
    } else {
        format!("{}m", minutes)
    }
}

use crate::app::{Alert, AppState, RaidAvailability, RaidState};
use crate::widgets::sparkline_cell::sparkline;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let raid_degraded = state
        .alerts
        .iter()
        .any(|a| matches!(a, Alert::RaidDegraded { .. }));
    let border_color = if raid_degraded {
        Color::Red
    } else {
        Color::White
    };

    // Prefer the array that needs attention: rebuilding first, then degraded,
    // then the first one. Title shows how many others exist.
    let raid = state
        .raids
        .iter()
        .find(|r| r.state == RaidState::Rebuilding)
        .or_else(|| state.raids.iter().find(|r| r.state == RaidState::Degraded))
        .or_else(|| state.raids.first());

    let mut title = match (raid, state.raids.len()) {
        (Some(r), n) if n > 1 => format!(" RAID Array {} (+{} more) ", r.name, n - 1),
        _ => " RAID Array ".to_string(),
    };
    if state.raid_availability != RaidAvailability::Complete {
        title = format!("{} [{:?}] ", title.trim_end(), state.raid_availability);
    }

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(raid) = raid else {
        let message = match state.raid_availability {
            RaidAvailability::Complete => " No RAID array detected",
            RaidAvailability::Partial => {
                " RAID inventory PARTIAL — last complete state unavailable"
            }
            RaidAvailability::Unavailable => {
                " RAID inventory UNAVAILABLE — retaining last known state"
            }
        };
        let p = Paragraph::new(Line::from(Span::styled(
            message,
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(p, inner);
        return;
    };

    // Split inner area: left info | right gauge+sparkline
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(10)])
        .split(inner);

    // ── Left column: name / state badge / disk count ──────────────────────
    let state_color = match raid.state {
        RaidState::Active => Color::Green,
        RaidState::Rebuilding => Color::Yellow,
        RaidState::Degraded => Color::Red,
        RaidState::Unknown => Color::DarkGray,
    };
    let state_label = match raid.state {
        RaidState::Active => "Active",
        RaidState::Rebuilding => "Rebuilding",
        RaidState::Degraded => "Degraded",
        RaidState::Unknown => "Unknown",
    };

    let left_lines = vec![
        Line::from(vec![
            Span::raw(" Array: "),
            Span::styled(
                raid.name.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw(" State: "),
            Span::styled(
                format!("[{}]", state_label),
                Style::default()
                    .fg(state_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw(" Disks: "),
            Span::styled(
                format!("{}/{}", raid.active_disks, raid.total_disks),
                Style::default().fg(Color::White),
            ),
        ]),
    ];
    let left_p = Paragraph::new(left_lines);
    f.render_widget(left_p, cols[0]);

    // ── Right column: progress bar + ETA/speed on top, sparkline on bottom ─
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(cols[1]);

    // Progress bar (only meaningful when rebuilding)
    if raid.state == RaidState::Rebuilding {
        let pct = raid.rebuild_pct.unwrap_or(0.0);
        let label = match (raid.rebuild_speed_mb, raid.eta_minutes) {
            (Some(spd), Some(eta)) => format!(" {:.1}%  {spd} MB/s  ETA:{} ", pct, format_eta(eta)),
            (Some(spd), None) => format!(" {:.1}%  {spd} MB/s ", pct),
            _ => format!(" {:.1}% ", pct),
        };
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Yellow).bg(Color::DarkGray))
            .ratio(pct / 100.0)
            .label(label);
        f.render_widget(gauge, right_rows[0]);
    } else {
        let msg = match raid.state {
            RaidState::Active => " Healthy ",
            RaidState::Degraded => " DEGRADED — disk missing ",
            _ => " — ",
        };
        let p = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(state_color),
        )));
        f.render_widget(p, right_rows[0]);
    }

    // RAID rebuild-speed sparkline row — capped at 20 samples per spec
    let spk_width = (right_rows[1].width as usize).min(20);
    let spk_data: Vec<u64> = state
        .raid_speed_history
        .get(&raid.name)
        .map(|h| h.iter().copied().collect())
        .unwrap_or_default();
    let spk_str = sparkline(&spk_data, spk_width);
    let spk_p = Paragraph::new(Line::from(Span::styled(
        spk_str,
        Style::default().fg(Color::Yellow),
    )));
    f.render_widget(spk_p, right_rows[1]);
}
