//! GitHub-style contribution heatmap: columns are weeks, rows are weekdays
//! (Mon top .. Sun bottom), each day drawn as a small square (`■`) with a
//! one-column gap so cells read as separated squares. The base hue follows the
//! active filter (green for combined, the app's color when one app is
//! selected); intensity is shaded into four buckets.

use chrono::Datelike;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use super::{empty_cell, heatmap_base, put, put_str, scale};
use crate::stats::{weekday_index, Dashboard};

const GUTTER: u16 = 4; // room for "Mon" labels + a space
const SQUARE: &str = "■";
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub fn render(buf: &mut Buffer, area: Rect, dash: &Dashboard) {
    if area.width <= GUTTER || area.height == 0 || dash.daily.is_empty() {
        return;
    }

    let base = heatmap_base(dash.filter);
    let max = dash.max_day();

    let start = dash.daily[0].date;
    let last = dash.daily[dash.daily.len() - 1].date;
    // The Monday on/before the first day, so columns align to calendar weeks.
    let start_monday = start - chrono::Duration::days(weekday_index(start) as i64);
    let n_cols = dash
        .daily
        .iter()
        .map(|c| ((c.date - start_monday).num_days() / 7) as u16)
        .max()
        .unwrap_or(0)
        + 1;

    // Two columns per week (square + margin) when it fits, else one.
    let avail = area.width - GUTTER;
    let step: u16 = if n_cols.saturating_mul(2) <= avail {
        2
    } else {
        1
    };
    let grid_x = area.x + GUTTER;

    let label_style = Style::new().fg(Color::Rgb(140, 140, 140));
    let dim_style = Style::new().fg(Color::Rgb(105, 105, 105));

    // Lay out top-down, adding the header rows only when there's vertical room.
    // Minimum is 7 weekday rows + 1 legend row = 8.
    let mut row_y = area.y;
    let range_y = if area.height >= 10 {
        let y = row_y;
        row_y += 1;
        Some(y)
    } else {
        None
    };
    let months_y = if area.height >= 9 {
        let y = row_y;
        row_y += 1;
        Some(y)
    } else {
        None
    };
    let grid_top = row_y;
    let legend_y = grid_top + 7;

    // Date range, e.g. "2025-06-09 .. 2026-06-13".
    if let Some(y) = range_y {
        let range = format!(
            "{} .. {}",
            start.format("%Y-%m-%d"),
            last.format("%Y-%m-%d")
        );
        put_str(buf, area, grid_x, y, &range, dim_style);
    }

    // Month labels across the top, at the first week of each month.
    if let Some(y) = months_y {
        let mut prev_month = 0u32;
        let mut last_label_end: i32 = -1;
        for col in 0..n_cols {
            let monday = start_monday + chrono::Duration::days(col as i64 * 7);
            let m = monday.month();
            if m != prev_month {
                prev_month = m;
                let lx = grid_x + col * step;
                if (lx as i32) > last_label_end && lx < area.right() {
                    let label = MONTHS[(m - 1) as usize];
                    put_str(buf, area, lx, y, label, label_style);
                    last_label_end = lx as i32 + label.len() as i32;
                }
            }
        }
    }

    // Weekday labels down the left side (Mon/Wed/Fri/Sun).
    for (row, name) in [(0u16, "Mon"), (2, "Wed"), (4, "Fri"), (6, "Sun")] {
        put_str(buf, area, area.x, grid_top + row, name, label_style);
    }

    // The day squares.
    for cell in &dash.daily {
        let row = weekday_index(cell.date) as u16;
        let col = ((cell.date - start_monday).num_days() / 7) as u16;
        let x = grid_x + col * step;
        let y = grid_top + row;
        let t = cell.total();
        let color = if t == 0 {
            empty_cell()
        } else {
            let factor = match level(t, max) {
                1 => 0.40,
                2 => 0.60,
                3 => 0.80,
                _ => 1.00,
            };
            scale(base, factor)
        };
        put(buf, area, x, y, SQUARE, Style::new().fg(color));
    }

    // Legend: "■ 0  ■ 1  ■ 2  ■ 3  ■ 4+".
    if legend_y < area.bottom() {
        let mut x = grid_x;
        for (i, factor) in [0.0_f32, 0.40, 0.60, 0.80, 1.0].iter().enumerate() {
            let c = if i == 0 {
                empty_cell()
            } else {
                scale(base, *factor)
            };
            put(buf, area, x, legend_y, SQUARE, Style::new().fg(c));
            x += 2; // square + space
            let label = if i == 4 {
                "4+".to_string()
            } else {
                i.to_string()
            };
            put_str(buf, area, x, legend_y, &label, label_style);
            x += label.len() as u16 + 2; // label + gap to next swatch
        }
    }
}

/// Intensity bucket 1..=4 for a non-zero value relative to `max`.
fn level(value: u32, max: u32) -> u8 {
    if max == 0 || value == 0 {
        return 0;
    }
    let ratio = value as f32 / max as f32;
    match ratio {
        r if r <= 0.25 => 1,
        r if r <= 0.50 => 2,
        r if r <= 0.75 => 3,
        _ => 4,
    }
}
