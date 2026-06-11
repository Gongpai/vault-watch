use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
    Frame,
};
use ratatui::widgets::canvas::{Canvas, Context, Line as CanvasLine};

use crate::app::{AppState, FocusedPanel};

const DISK_COLORS: [Color; 6] = [
    Color::Cyan,
    Color::Yellow,
    Color::Green,
    Color::Magenta,
    Color::Blue,
    Color::Red,
];

/// Temperature zone backgrounds: (y_min inclusive, y_max exclusive, fill color).
const TEMP_ZONES: [(f64, f64, Color); 5] = [
    (0.0,  30.0, Color::Rgb(8,  53, 77)),
    (30.0, 40.0, Color::Rgb(2,  55, 15)),
    (40.0, 50.0, Color::Rgb(71, 57,  0)),
    (50.0, 60.0, Color::Rgb(64,  0,  0)),
    (60.0, 90.0, Color::Rgb(39,  0, 52)),
];

/// Solid dark background for I/O and RAID speed graphs.
const IO_BG: Color = Color::Rgb(10, 13, 20);

// ── Background widget ─────────────────────────────────────────────────────────

/// Paints per-row background colors derived from canvas zone boundaries.
///
/// Render AFTER `Canvas` so zone colors overwrite the Canvas's uniform
/// background while leaving the braille characters Canvas already drew
/// intact (only `set_bg` is called — character and foreground are untouched).
struct ZoneBackground<'a> {
    zones: &'a [(f64, f64, Color)],
    y_min: f64,
    y_max: f64,
}

impl Widget for ZoneBackground<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let range = self.y_max - self.y_min;
        if range <= 0.0 || area.height == 0 {
            return;
        }
        for row_offset in 0..area.height {
            // Canvas y increases upward; terminal row 0 corresponds to y_max.
            // Sample the midpoint of each terminal row.
            let canvas_y =
                self.y_max - ((row_offset as f64 + 0.5) / area.height as f64) * range;
            let color = self
                .zones
                .iter()
                .find(|(lo, hi, _)| canvas_y >= *lo && canvas_y < *hi)
                .map(|(_, _, c)| *c)
                .unwrap_or(Color::Black);
            let row = area.top() + row_offset;
            for col in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((col, row)) {
                    cell.set_bg(color);
                }
            }
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, area: Rect, state: &mut AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

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

// ── Helpers ───────────────────────────────────────────────────────────────────

fn history_to_points(
    history: &std::collections::VecDeque<u64>,
    scale: f64,
) -> Vec<(f64, f64)> {
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

fn panel_block(title: &str, focused: bool) -> Block<'_> {
    let (border_type, border_color) = if focused {
        (BorderType::Double, Color::Cyan)
    } else {
        (BorderType::Plain, Color::White)
    };
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color))
}

fn draw_line_series(ctx: &mut Context<'_>, pts: &[(f64, f64)], color: Color) {
    for w in pts.windows(2) {
        ctx.draw(&CanvasLine {
            x1: w[0].0,
            y1: w[0].1,
            x2: w[1].0,
            y2: w[1].1,
            color,
        });
    }
}

/// Place Y-axis labels at their proportional row positions.
/// `labels`: `(canvas_y_value, color, text)` — top-of-chart = y_max, bottom = y_min.
fn render_y_labels(
    f: &mut Frame,
    area: Rect,
    labels: &[(f64, Color, &str)],
    y_min: f64,
    y_max: f64,
) {
    let range = y_max - y_min;
    if range <= 0.0 || area.height == 0 {
        return;
    }
    for &(val, color, text) in labels {
        let ratio = (val - y_min) / range;
        let row = ((1.0 - ratio) * area.height as f64) as u16;
        let row = row.min(area.height.saturating_sub(1));
        let row_area = Rect {
            x: area.x,
            y: area.y + row,
            width: area.width,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(Span::styled(text, Style::default().fg(color))),
            row_area,
        );
    }
}

/// Color-keyed legend in the top-right corner of `area`.
fn render_legend(f: &mut Frame, area: Rect, entries: &[(String, Color)]) {
    if entries.is_empty() || area.height < 3 || area.width < 8 {
        return;
    }
    let max_name = entries.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    let leg_w = ((max_name + 3) as u16).min(area.width / 2);
    let leg_h = (entries.len() as u16).min(area.height.saturating_sub(2));
    if leg_w < 4 || leg_h == 0 {
        return;
    }
    let leg_x = area.right().saturating_sub(leg_w + 1);
    let leg_y = area.top() + 1;

    for (i, (name, color)) in entries.iter().enumerate().take(leg_h as usize) {
        let row_area = Rect {
            x: leg_x,
            y: leg_y + i as u16,
            width: leg_w,
            height: 1,
        };
        let line = Line::from(vec![
            Span::styled("█ ", Style::default().fg(*color)),
            Span::styled(name.as_str(), Style::default().fg(Color::White)),
        ]);
        f.render_widget(
            Paragraph::new(line).style(Style::default().bg(Color::Black)),
            row_area,
        );
    }
}

// ── Graph renderers ───────────────────────────────────────────────────────────

fn render_temp_graph(f: &mut Frame, area: Rect, state: &mut AppState) {
    let focused = state.focused_panel == FocusedPanel::TempGraph;
    state.panel_rects.insert(FocusedPanel::TempGraph, area);

    let block = panel_block(" Temperature (°C) ", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width < 8 || inner.height < 3 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(inner);
    let (y_col, canvas_area) = (chunks[0], chunks[1]);

    // Snapshot all data before moving into the Canvas closure.
    let devices: Vec<String> = state.disk_devices.clone();
    let data: Vec<Vec<(f64, f64)>> = devices
        .iter()
        .map(|d| {
            state
                .temp_history
                .get(d)
                .map(|h| history_to_points(h, 1.0))
                .unwrap_or_default()
        })
        .collect();
    let max_len = data.iter().map(|pts| pts.len()).max().unwrap_or(1);
    let x_min = -((max_len.saturating_sub(1)) as f64) * 2.0;

    // 1. Canvas — braille lines; bg=Reset so zone colors show through.
    f.render_widget(
        Canvas::default()
            .x_bounds([x_min, 0.0])
            .y_bounds([0.0, 90.0])
            .background_color(Color::Reset)
            .marker(symbols::Marker::Braille)
            .paint(move |ctx| {
                ctx.draw(&CanvasLine {
                    x1: x_min, y1: 45.0, x2: 0.0, y2: 45.0,
                    color: Color::Yellow,
                });
                ctx.draw(&CanvasLine {
                    x1: x_min, y1: 55.0, x2: 0.0, y2: 55.0,
                    color: Color::Red,
                });
                for (i, pts) in data.iter().enumerate() {
                    draw_line_series(ctx, pts, DISK_COLORS[i % DISK_COLORS.len()]);
                }
            }),
        canvas_area,
    );

    // 2. Zone backgrounds — overwrites Canvas bg per row, braille chars intact.
    f.render_widget(
        ZoneBackground { zones: &TEMP_ZONES, y_min: 0.0, y_max: 90.0 },
        canvas_area,
    );

    // 3. Y-axis labels.
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(Color::Black)),
        y_col,
    );
    render_y_labels(
        f,
        y_col,
        &[
            (90.0, Color::Gray,    "90"),
            (55.0, Color::Red,     "55°"),
            (45.0, Color::Yellow,  "45°"),
            (0.0,  Color::DarkGray, "0"),
        ],
        0.0,
        90.0,
    );

    // 4. Legend.
    let legend: Vec<(String, Color)> = devices
        .iter()
        .enumerate()
        .map(|(i, d)| (d.clone(), DISK_COLORS[i % DISK_COLORS.len()]))
        .collect();
    render_legend(f, canvas_area, &legend);
}

fn render_io_graph(
    f: &mut Frame,
    area: Rect,
    state: &mut AppState,
    panel: FocusedPanel,
    title: &str,
) {
    let focused = state.focused_panel == panel;
    state.panel_rects.insert(panel, area);

    let block = panel_block(title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width < 8 || inner.height < 3 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(inner);
    let (y_col, canvas_area) = (chunks[0], chunks[1]);

    let devices: Vec<String> = state.disk_devices.clone();
    let data: Vec<Vec<(f64, f64)>> = {
        let history = match panel {
            FocusedPanel::ReadGraph => &state.read_history,
            _ => &state.write_history,
        };
        devices
            .iter()
            .map(|d| {
                history
                    .get(d)
                    .map(|h| history_to_points(h, 10.0))
                    .unwrap_or_default()
            })
            .collect()
    };
    let max_len = data.iter().map(|pts| pts.len()).max().unwrap_or(1);
    let x_min = -((max_len.saturating_sub(1)) as f64) * 2.0;

    // IO background is uniform — set directly via Canvas.background_color.
    f.render_widget(
        Canvas::default()
            .x_bounds([x_min, 0.0])
            .y_bounds([0.0, 200.0])
            .background_color(IO_BG)
            .marker(symbols::Marker::Braille)
            .paint(move |ctx| {
                for (i, pts) in data.iter().enumerate() {
                    draw_line_series(ctx, pts, DISK_COLORS[i % DISK_COLORS.len()]);
                }
            }),
        canvas_area,
    );

    f.render_widget(
        Paragraph::new("").style(Style::default().bg(Color::Black)),
        y_col,
    );
    render_y_labels(
        f,
        y_col,
        &[
            (200.0, Color::Gray,    "200"),
            (100.0, Color::DarkGray, "100"),
            (0.0,   Color::DarkGray,   "0"),
        ],
        0.0,
        200.0,
    );

    let legend: Vec<(String, Color)> = devices
        .iter()
        .enumerate()
        .map(|(i, d)| (d.clone(), DISK_COLORS[i % DISK_COLORS.len()]))
        .collect();
    render_legend(f, canvas_area, &legend);
}

fn render_raid_graph(f: &mut Frame, area: Rect, state: &mut AppState) {
    let focused = state.focused_panel == FocusedPanel::RaidGraph;
    state.panel_rects.insert(FocusedPanel::RaidGraph, area);

    let block = panel_block(" RAID Rebuild Speed (MB/s) ", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width < 8 || inner.height < 3 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(inner);
    let (y_col, canvas_area) = (chunks[0], chunks[1]);

    let mut names: Vec<String> = state.raid_speed_history.keys().cloned().collect();
    names.sort();

    let data: Vec<Vec<(f64, f64)>> = names
        .iter()
        .map(|name| history_to_points(&state.raid_speed_history[name], 10.0))
        .collect();

    let max_val = state
        .raid_speed_history
        .values()
        .flat_map(|h| h.iter().copied())
        .max()
        .unwrap_or(1000) as f64
        / 10.0;
    let y_max = (max_val * 1.2).max(100.0);

    let max_len = data.iter().map(|pts| pts.len()).max().unwrap_or(1);
    let x_min = -((max_len.saturating_sub(1)) as f64) * 2.0;

    f.render_widget(
        Canvas::default()
            .x_bounds([x_min, 0.0])
            .y_bounds([0.0, y_max])
            .background_color(IO_BG)
            .marker(symbols::Marker::Braille)
            .paint(move |ctx| {
                for (i, pts) in data.iter().enumerate() {
                    draw_line_series(ctx, pts, DISK_COLORS[i % DISK_COLORS.len()]);
                }
            }),
        canvas_area,
    );

    let mid_label = format!("{:.0}", y_max / 2.0);
    let max_label = format!("{:.0}", y_max);
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(Color::Black)),
        y_col,
    );
    render_y_labels(
        f,
        y_col,
        &[
            (y_max,       Color::Gray,    max_label.as_str()),
            (y_max / 2.0, Color::DarkGray, mid_label.as_str()),
            (0.0,         Color::DarkGray,           "0"),
        ],
        0.0,
        y_max,
    );

    let legend: Vec<(String, Color)> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.clone(), DISK_COLORS[i % DISK_COLORS.len()]))
        .collect();
    render_legend(f, canvas_area, &legend);
}
