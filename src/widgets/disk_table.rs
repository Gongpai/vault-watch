use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table,
    },
    Frame,
};

use crate::app::{AppState, FocusedPanel};
use crate::widgets::sparkline_cell::sparkline;

// Column widths: total 72 + 5 col_spacings = 77 chars (fits in 80-col terminal)
const COL_DISK: u16 = 5;
const COL_TEMP: u16 = 18; // 12 sparkline + 1 space + 5 value ("NNN°C")
const COL_READ: u16 = 18; // 12 sparkline + 1 space + 5 value
const COL_WRITE: u16 = 18; // 12 sparkline + 1 space + 5 value
const COL_HEALTH: u16 = 5;
const COL_DEFECTS: u16 = 8;

const SPARKLINE_W: usize = 12;

const DISK_COLORS: [Color; 6] = [
    Color::Cyan,
    Color::Yellow,
    Color::Green,
    Color::Magenta,
    Color::Blue,
    Color::Red,
];

pub fn render(f: &mut Frame, area: Rect, state: &mut AppState) {
    let focused = state.focused_panel == FocusedPanel::DiskTable;
    let (border_type, border_color) = if focused {
        (BorderType::Double, Color::Cyan)
    } else {
        (BorderType::Plain, Color::White)
    };

    let block = Block::default()
        .title(" Disk Summary ")
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color));

    state.panel_rects.insert(FocusedPanel::DiskTable, area);

    let inner = block.inner(area);

    let header = Row::new(vec![
        Cell::from("Disk").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Temperature").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Read MB/s").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Write MB/s").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Health").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Defects").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().fg(Color::White))
    .height(1);

    let total_disks = state.disks.len();

    // Visible rows = inner height minus 1 header row
    let visible = (inner.height as usize).saturating_sub(1);

    // Clamp scroll to valid range (authoritative — only one clamp)
    let max_scroll = total_disks.saturating_sub(visible);
    let scroll = state.disk_table_scroll.min(max_scroll);
    state.disk_table_scroll = scroll;

    let rows: Vec<Row> = state
        .disks
        .iter()
        .enumerate()
        .map(|(i, disk)| {
            let color = DISK_COLORS[i % DISK_COLORS.len()];

            // Disk name cell
            let disk_cell = Cell::from(disk.device.clone()).style(Style::default().fg(color));

            // Temperature cell: sparkline + value
            let temp_color = match disk.temperature_c {
                Some(t) if t > 55 => Color::Red,
                Some(t) if t >= 45 => Color::Yellow,
                _ => Color::Green,
            };
            let temp_spk = {
                let empty = std::collections::VecDeque::new();
                let hist = state.temp_history.get(&disk.device).unwrap_or(&empty);
                let data: Vec<u64> = hist.iter().copied().collect();
                sparkline(&data, SPARKLINE_W)
            };
            let temp_val = match disk.temperature_c {
                Some(t) => format!("{:>3}°C", t),
                None => format!("{:>5}", "--"),
            };
            let temp_cell = Cell::from(Line::from(vec![
                Span::styled(temp_spk, Style::default().fg(temp_color)),
                Span::raw(" "),
                Span::styled(temp_val, Style::default().fg(temp_color)),
            ]));

            // Read cell: sparkline + value
            let io_stat = state.io_stats.iter().find(|s| s.device == disk.device);
            let read_spk = {
                let empty = std::collections::VecDeque::new();
                let hist = state.read_history.get(&disk.device).unwrap_or(&empty);
                let data: Vec<u64> = hist.iter().copied().collect();
                sparkline(&data, SPARKLINE_W)
            };
            let read_val = match io_stat {
                Some(s) => format!("{:>5.1}", s.read_mb_s),
                None => format!("{:>5}", "--"),
            };
            let read_cell = Cell::from(Line::from(vec![
                Span::styled(read_spk, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(read_val, Style::default().fg(Color::Cyan)),
            ]));

            // Write cell: sparkline + value
            let write_spk = {
                let empty = std::collections::VecDeque::new();
                let hist = state.write_history.get(&disk.device).unwrap_or(&empty);
                let data: Vec<u64> = hist.iter().copied().collect();
                sparkline(&data, SPARKLINE_W)
            };
            let write_val = match io_stat {
                Some(s) => format!("{:>5.1}", s.write_mb_s),
                None => format!("{:>5}", "--"),
            };
            let write_cell = Cell::from(Line::from(vec![
                Span::styled(write_spk, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(write_val, Style::default().fg(Color::Magenta)),
            ]));

            // Health cell (binary OK / FAIL)
            let (health_str, health_color) = if disk.health_ok {
                ("OK   ", Color::Green)
            } else {
                ("FAIL ", Color::Red)
            };
            let health_cell =
                Cell::from(Span::styled(health_str, Style::default().fg(health_color)));

            // Defects cell (separate column, Yellow when > 0)
            let defects_cell = match disk.grown_defects {
                Some(d) if d > 0 => Cell::from(Span::styled(
                    format!("WARN {:>2}", d),
                    Style::default().fg(Color::Yellow),
                )),
                Some(d) => Cell::from(Span::styled(
                    format!("{:>8}", d),
                    Style::default().fg(Color::White),
                )),
                None => Cell::from(Span::styled(
                    format!("{:>8}", "--"),
                    Style::default().fg(Color::DarkGray),
                )),
            };

            Row::new(vec![
                disk_cell,
                temp_cell,
                read_cell,
                write_cell,
                health_cell,
                defects_cell,
            ])
            .height(1)
        })
        .collect();

    let constraints = [
        Constraint::Length(COL_DISK),
        Constraint::Length(COL_TEMP),
        Constraint::Length(COL_READ),
        Constraint::Length(COL_WRITE),
        Constraint::Length(COL_HEALTH),
        Constraint::Length(COL_DEFECTS),
    ];

    let visible_rows: Vec<Row> = rows.into_iter().skip(scroll).take(visible).collect();

    let table = Table::new(visible_rows, constraints)
        .header(header)
        .block(block)
        .column_spacing(1);

    f.render_widget(table, area);

    // Scrollbar
    if total_disks > visible {
        let mut scrollbar_state = ScrollbarState::new(total_disks).position(scroll);
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
    let remaining = total_disks.saturating_sub(scroll + visible);
    if remaining > 0 {
        let hint = format!("↓ {} more", remaining);
        let hint_x = area.x + area.width.saturating_sub(hint.len() as u16 + 2);
        let hint_y = area.y + area.height - 1;
        let hint_area = Rect::new(hint_x, hint_y, hint.len() as u16, 1);
        let p = ratatui::widgets::Paragraph::new(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(p, hint_area);
    }
}
