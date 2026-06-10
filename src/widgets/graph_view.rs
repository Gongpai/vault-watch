use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, BorderType, Borders, Chart, Dataset, GraphType},
    Frame,
};

use crate::app::{AppState, FocusedPanel};

const DISK_COLORS: [Color; 6] = [
    Color::Cyan,
    Color::Yellow,
    Color::Green,
    Color::Magenta,
    Color::Blue,
    Color::Red,
];

pub fn render(f: &mut Frame, area: Rect, state: &mut AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(cols[0]);

    render_temp_graph(f, left_rows[0], state);
    render_raid_graph(f, left_rows[1], state);
    render_throughput_graph(f, cols[1], state);
}

fn history_to_points(history: &std::collections::VecDeque<u64>, scale: f64) -> Vec<(f64, f64)> {
    let len = history.len();
    history
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = -((len - 1 - i) as f64) * 2.0;
            let y = v as f64 / scale;
            (x, y)
        })
        .collect()
}

fn render_temp_graph(f: &mut Frame, area: Rect, state: &mut AppState) {
    let focused = state.focused_panel == FocusedPanel::TempGraph;
    state.panel_rects.insert(FocusedPanel::TempGraph, area);

    let (border_type, border_color) = if focused {
        (BorderType::Double, Color::Cyan)
    } else {
        (BorderType::Plain, Color::White)
    };

    let block = Block::default()
        .title(" Temperature (°C) ")
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color));

    // Build one dataset per disk
    let datasets_data: Vec<Vec<(f64, f64)>> = state
        .disk_devices
        .iter()
        .map(|dev| {
            if let Some(hist) = state.temp_history.get(dev) {
                history_to_points(hist, 1.0)
            } else {
                vec![]
            }
        })
        .collect();

    let datasets: Vec<Dataset> = state
        .disk_devices
        .iter()
        .enumerate()
        .zip(datasets_data.iter())
        .map(|((i, dev), data)| {
            Dataset::default()
                .name(dev.as_str())
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(DISK_COLORS[i % DISK_COLORS.len()]))
                .data(data)
        })
        .collect();

    let max_len = state
        .disk_devices
        .iter()
        .filter_map(|d| state.temp_history.get(d))
        .map(|h| h.len())
        .max()
        .unwrap_or(1);

    let x_min = -((max_len.saturating_sub(1)) as f64) * 2.0;

    let chart = Chart::new(datasets)
        .block(block)
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([x_min, 0.0]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .labels(vec![
                    Span::raw("0"),
                    Span::raw("30"),
                    Span::raw("60"),
                    Span::raw("90"),
                ])
                .bounds([0.0, 90.0]),
        );

    f.render_widget(chart, area);
}

fn render_raid_graph(f: &mut Frame, area: Rect, state: &mut AppState) {
    let focused = state.focused_panel == FocusedPanel::RaidGraph;
    state.panel_rects.insert(FocusedPanel::RaidGraph, area);

    let (border_type, border_color) = if focused {
        (BorderType::Double, Color::Cyan)
    } else {
        (BorderType::Plain, Color::White)
    };

    let block = Block::default()
        .title(" RAID Rebuild Speed (MB/s) ")
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color));

    // raid_speed_history is stored ×10 (same scale as read/write history)
    let raid_data = history_to_points(&state.raid_speed_history, 10.0);
    let max_val = state
        .raid_speed_history
        .iter()
        .copied()
        .max()
        .unwrap_or(1000) as f64
        / 10.0;
    let y_max = (max_val * 1.2).max(100.0);

    let max_len = state.raid_speed_history.len().max(1);
    let x_min = -((max_len.saturating_sub(1)) as f64) * 2.0;

    let datasets = vec![Dataset::default()
        .name("rebuild")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Yellow))
        .data(&raid_data)];

    let chart = Chart::new(datasets)
        .block(block)
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([x_min, 0.0]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .labels(vec![Span::raw("0"), Span::raw(format!("{:.0}", y_max / 2.0)), Span::raw(format!("{:.0}", y_max))])
                .bounds([0.0, y_max]),
        );

    f.render_widget(chart, area);
}

fn render_throughput_graph(f: &mut Frame, area: Rect, state: &mut AppState) {
    let focused = state.focused_panel == FocusedPanel::ThroughputGraph;
    state.panel_rects.insert(FocusedPanel::ThroughputGraph, area);

    let (border_type, border_color) = if focused {
        (BorderType::Double, Color::Cyan)
    } else {
        (BorderType::Plain, Color::White)
    };

    let block = Block::default()
        .title(" Throughput (MB/s) ")
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color));

    // Read series (solid) + Write series (dimmed) per disk
    let read_data: Vec<Vec<(f64, f64)>> = state
        .disk_devices
        .iter()
        .map(|dev| {
            if let Some(hist) = state.read_history.get(dev) {
                history_to_points(hist, 10.0)
            } else {
                vec![]
            }
        })
        .collect();

    let write_data: Vec<Vec<(f64, f64)>> = state
        .disk_devices
        .iter()
        .map(|dev| {
            if let Some(hist) = state.write_history.get(dev) {
                history_to_points(hist, 10.0)
            } else {
                vec![]
            }
        })
        .collect();

    let mut datasets: Vec<Dataset> = Vec::new();

    for (i, dev) in state.disk_devices.iter().enumerate() {
        let color = DISK_COLORS[i % DISK_COLORS.len()];
        datasets.push(
            Dataset::default()
                .name(format!("{} R", dev))
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(color))
                .data(&read_data[i]),
        );
        datasets.push(
            Dataset::default()
                .name(format!("{} W", dev))
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::DarkGray))
                .data(&write_data[i]),
        );
    }

    let max_len = state
        .disk_devices
        .iter()
        .filter_map(|d| state.read_history.get(d))
        .map(|h| h.len())
        .max()
        .unwrap_or(1);

    let x_min = -((max_len.saturating_sub(1)) as f64) * 2.0;

    let chart = Chart::new(datasets)
        .block(block)
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([x_min, 0.0]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .labels(vec![Span::raw("0"), Span::raw("100"), Span::raw("200")])
                .bounds([0.0, 200.0]),
        );

    f.render_widget(chart, area);
}
