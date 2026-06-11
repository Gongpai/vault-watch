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

// Allow the legend up to half the panel — ratatui's default of ¼ hides it as
// soon as a handful of named datasets don't fit (US-MON-20).
const LEGEND_CONSTRAINTS: (Constraint, Constraint) =
    (Constraint::Ratio(1, 2), Constraint::Ratio(1, 2));

pub fn render(f: &mut Frame, area: Rect, state: &mut AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // RAID graph only occupies space while a rebuild is running (or its
    // history is still draining); otherwise temperature gets the full column.
    if state.raid_graph_visible() {
        let left_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(cols[0]);

        render_temp_graph(f, left_rows[0], state);
        render_raid_graph(f, left_rows[1], state);
    } else {
        if state.focused_panel == FocusedPanel::RaidGraph {
            state.focused_panel = FocusedPanel::TempGraph;
        }
        state.panel_rects.remove(&FocusedPanel::RaidGraph);
        render_temp_graph(f, cols[0], state);
    }

    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[1]);

    render_io_graph(f, right_rows[0], state, FocusedPanel::ReadGraph, " Read (MB/s) ");
    render_io_graph(f, right_rows[1], state, FocusedPanel::WriteGraph, " Write (MB/s) ");
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

    // Threshold reference lines at 45°C (warm) and 55°C (hot).
    // Unnamed on purpose: named datasets enter the legend, and the extra rows
    // can push it past ratatui's height limit, hiding the device legend.
    let threshold_warm: Vec<(f64, f64)> = vec![(x_min, 45.0), (0.0, 45.0)];
    let threshold_hot: Vec<(f64, f64)> = vec![(x_min, 55.0), (0.0, 55.0)];
    let mut all_datasets: Vec<Dataset> = datasets;
    all_datasets.push(
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Yellow))
            .data(&threshold_warm),
    );
    all_datasets.push(
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Red))
            .data(&threshold_hot),
    );

    let chart = Chart::new(all_datasets)
        .block(block)
        .hidden_legend_constraints(LEGEND_CONSTRAINTS)
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
                    Span::styled("45°", Style::default().fg(Color::Yellow)),
                    Span::styled("55°", Style::default().fg(Color::Red)),
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

    // One line per array, sorted by name so colors stay stable across frames.
    // Histories are stored ×10 (same scale as read/write history).
    let mut names: Vec<&String> = state.raid_speed_history.keys().collect();
    names.sort();

    let datasets_data: Vec<Vec<(f64, f64)>> = names
        .iter()
        .map(|name| history_to_points(&state.raid_speed_history[*name], 10.0))
        .collect();

    let datasets: Vec<Dataset> = names
        .iter()
        .enumerate()
        .zip(datasets_data.iter())
        .map(|((i, name), data)| {
            Dataset::default()
                .name(name.as_str())
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(DISK_COLORS[i % DISK_COLORS.len()]))
                .data(data)
        })
        .collect();

    let max_val = state
        .raid_speed_history
        .values()
        .flat_map(|h| h.iter().copied())
        .max()
        .unwrap_or(1000) as f64
        / 10.0;
    let y_max = (max_val * 1.2).max(100.0);

    let max_len = state
        .raid_speed_history
        .values()
        .map(|h| h.len())
        .max()
        .unwrap_or(1);
    let x_min = -((max_len.saturating_sub(1)) as f64) * 2.0;

    let chart = Chart::new(datasets)
        .block(block)
        .hidden_legend_constraints(LEGEND_CONSTRAINTS)
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

/// Shared renderer for the Read and Write charts — same per-device colors in
/// both panels, same fixed Y scale so they can be compared at a glance.
fn render_io_graph(
    f: &mut Frame,
    area: Rect,
    state: &mut AppState,
    panel: FocusedPanel,
    title: &str,
) {
    let focused = state.focused_panel == panel;
    state.panel_rects.insert(panel, area);

    let (border_type, border_color) = if focused {
        (BorderType::Double, Color::Cyan)
    } else {
        (BorderType::Plain, Color::White)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color));

    let history = match panel {
        FocusedPanel::ReadGraph => &state.read_history,
        _ => &state.write_history,
    };

    let datasets_data: Vec<Vec<(f64, f64)>> = state
        .disk_devices
        .iter()
        .map(|dev| {
            if let Some(hist) = history.get(dev) {
                history_to_points(hist, 10.0)
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
        .filter_map(|d| history.get(d))
        .map(|h| h.len())
        .max()
        .unwrap_or(1);

    let x_min = -((max_len.saturating_sub(1)) as f64) * 2.0;

    let chart = Chart::new(datasets)
        .block(block)
        .hidden_legend_constraints(LEGEND_CONSTRAINTS)
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
