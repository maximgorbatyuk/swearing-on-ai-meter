//! Custom render helpers and the per-app color scheme. The bar charts and the
//! heatmap are hand-drawn so per-app data can render as stacked colored
//! segments (bars) and intensity-shaded cells (heatmap).

pub mod heatmap;
pub mod hour;
pub mod weekday;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use crate::model::Source;

/// Fixed color per source, used consistently across charts and legend.
pub fn source_color(src: Source) -> Color {
    match src {
        Source::ClaudeCode => Color::Rgb(217, 119, 87), // Claude orange
        Source::ClaudeDesktop => Color::Rgb(150, 110, 220), // purple
        Source::CodexCli => Color::Rgb(72, 180, 130),   // green
        Source::CodexApp => Color::Rgb(90, 160, 225),   // blue
        Source::Opencode => Color::Rgb(222, 184, 70),   // yellow
    }
}

/// The base color for the heatmap under a given filter: GitHub green for the
/// combined view, the app's own color when a single app is selected.
pub fn heatmap_base(filter: crate::stats::AppFilter) -> Color {
    match filter {
        crate::stats::AppFilter::All => Color::Rgb(57, 211, 83),
        crate::stats::AppFilter::One(s) => source_color(s),
    }
}

/// Write a single styled cell, bounds-checked against `area`.
pub fn put(buf: &mut Buffer, area: Rect, x: u16, y: u16, ch: &str, style: Style) {
    if x >= area.x && x < area.right() && y >= area.y && y < area.bottom() {
        buf.set_string(x, y, ch, style);
    }
}

/// Write a string clipped to `area`'s right edge.
pub fn put_str(buf: &mut Buffer, area: Rect, x: u16, y: u16, s: &str, style: Style) {
    if y < area.y || y >= area.bottom() || x >= area.right() {
        return;
    }
    let max = (area.right() - x) as usize;
    let clipped: String = s.chars().take(max).collect();
    buf.set_string(x, y, clipped, style);
}

/// Scale `base` brightness by `factor` (0.0..=1.0). Non-RGB colors pass through.
pub fn scale(base: Color, factor: f32) -> Color {
    match base {
        Color::Rgb(r, g, b) => {
            let f = factor.clamp(0.0, 1.0);
            Color::Rgb(
                (r as f32 * f).round() as u8,
                (g as f32 * f).round() as u8,
                (b as f32 * f).round() as u8,
            )
        }
        other => other,
    }
}

/// The dim color used for empty heatmap cells.
pub fn empty_cell() -> Color {
    Color::Rgb(48, 54, 61)
}

use crate::stats::{PerSource, N_SOURCES};

/// Render stacked vertical bars: one column per entry in `columns`, each bar a
/// stack of per-source colored segments (in `Source::ALL` order, bottom-up).
/// The bottom row of `area` is reserved for `labels` (centered under each bar).
pub fn stacked_bars(
    buf: &mut Buffer,
    area: Rect,
    columns: &[PerSource],
    labels: &[String],
    bar_w: u16,
    gap: u16,
) {
    if area.height < 2 || area.width == 0 || columns.is_empty() {
        return;
    }
    let chart_h = area.height - 1; // reserve last row for labels
    let label_y = area.y + chart_h;
    let max: u32 = columns
        .iter()
        .map(|c| c.iter().sum::<u32>())
        .max()
        .unwrap_or(0);

    let step = bar_w + gap;
    let label_style = Style::new().fg(Color::Rgb(120, 120, 120));

    for (i, col) in columns.iter().enumerate() {
        let x0 = area.x + (i as u16) * step;
        if x0 >= area.right() {
            break;
        }

        // Total bar height in cells.
        let total: u32 = col.iter().sum();
        let bar_cells: u16 = if total == 0 || max == 0 {
            0
        } else {
            let h = ((total as f32 / max as f32) * chart_h as f32).round() as u16;
            h.max(1).min(chart_h)
        };

        // Per-source segment heights, summing to bar_cells.
        let segs = segment_heights(col, total, bar_cells);

        // Draw bottom-up.
        let mut y = area.y + chart_h; // one past bottom; decremented first
        for src in crate::model::Source::ALL {
            let h = segs[src.index()];
            let style = Style::new().fg(source_color(src));
            for _ in 0..h {
                if y == area.y {
                    break;
                }
                y -= 1;
                draw_bar_row(buf, area, x0, y, bar_w, style);
            }
        }

        // Centered label.
        if let Some(label) = labels.get(i) {
            if !label.is_empty() {
                let lw = label.chars().count() as u16;
                let lx = x0 + bar_w.saturating_sub(lw) / 2;
                put_str(buf, area, lx, label_y, label, label_style);
            }
        }
    }
}

fn draw_bar_row(buf: &mut Buffer, area: Rect, x0: u16, y: u16, bar_w: u16, style: Style) {
    for dx in 0..bar_w {
        let x = x0 + dx;
        if x < area.right() {
            buf.set_string(x, y, "█", style);
        }
    }
}

/// Distribute `bar_cells` across sources proportionally to their counts, using
/// largest-remainder rounding so the segments sum exactly to `bar_cells`.
fn segment_heights(col: &PerSource, total: u32, bar_cells: u16) -> [u16; N_SOURCES] {
    let mut out = [0u16; N_SOURCES];
    if total == 0 || bar_cells == 0 {
        return out;
    }
    let mut frac: Vec<(usize, f32)> = Vec::with_capacity(N_SOURCES);
    let mut assigned: u16 = 0;
    for (i, &c) in col.iter().enumerate() {
        let exact = (c as f32 / total as f32) * bar_cells as f32;
        let floor = exact.floor();
        out[i] = floor as u16;
        assigned += floor as u16;
        frac.push((i, exact - floor));
    }
    let mut leftover = bar_cells.saturating_sub(assigned);
    frac.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (i, _) in frac {
        if leftover == 0 {
            break;
        }
        // Only top up sources that actually have a count.
        if col[i] > 0 {
            out[i] += 1;
            leftover -= 1;
        }
    }
    out
}
