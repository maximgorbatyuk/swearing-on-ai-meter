//! GitHub-style contribution heatmap: columns are weeks, rows are weekdays
//! (Mon top .. Sun bottom), cells shaded by swear intensity. The base hue
//! follows the active filter (green for combined, the app's color when one app
//! is selected).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use super::{empty_cell, heatmap_base, put_str, scale};
use crate::stats::{weekday_index, Dashboard};

const GUTTER: u16 = 4; // room for "Mon" labels + a space

pub fn render(buf: &mut Buffer, area: Rect, dash: &Dashboard) {
    if area.width <= GUTTER || area.height == 0 || dash.daily.is_empty() {
        return;
    }

    let base = heatmap_base(dash.filter);
    let max = dash.max_day();

    let start = dash.daily[0].date;
    // The Monday on/before the first day, so columns align to calendar weeks.
    let start_monday = start - chrono::Duration::days(weekday_index(start) as i64);
    let n_cols = dash
        .daily
        .iter()
        .map(|c| ((c.date - start_monday).num_days() / 7) as u16)
        .max()
        .unwrap_or(0)
        + 1;

    let avail = area.width - GUTTER;
    let cell_w: u16 = if n_cols.saturating_mul(2) <= avail {
        2
    } else {
        1
    };
    let grid_x = area.x + GUTTER;

    // Weekday labels down the left side (Mon/Wed/Fri).
    let label_style = Style::new().fg(Color::Rgb(120, 120, 120));
    for (row, name) in [(0u16, "Mon"), (2, "Wed"), (4, "Fri")] {
        put_str(buf, area, area.x, area.y + row, name, label_style);
    }

    for cell in &dash.daily {
        let row = weekday_index(cell.date) as u16;
        let col = ((cell.date - start_monday).num_days() / 7) as u16;
        let x = grid_x + col * cell_w;
        let y = area.y + row;
        if y >= area.bottom() || x >= area.right() {
            continue;
        }
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
        let block = "█".repeat(cell_w as usize);
        let avail_w = (area.right() - x) as usize;
        let block: String = block.chars().take(avail_w).collect();
        buf.set_string(x, y, block, Style::new().fg(color));
    }

    // Legend (if there's a spare row below the 7 weekday rows).
    let legend_y = area.y + 7;
    if legend_y < area.bottom() {
        let mut x = grid_x;
        put_str(buf, area, area.x, legend_y, "less", label_style);
        x += 5;
        for (i, factor) in [0.0_f32, 0.40, 0.60, 0.80, 1.0].iter().enumerate() {
            let c = if i == 0 {
                empty_cell()
            } else {
                scale(base, *factor)
            };
            if x < area.right() {
                buf.set_string(x, legend_y, "█", Style::new().fg(c));
            }
            x += 1;
        }
        x += 1;
        put_str(buf, area, x, legend_y, "more", label_style);
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
