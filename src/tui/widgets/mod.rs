//! Custom render helpers and the per-app color scheme. The bar charts and the
//! heatmap are hand-drawn so per-app data can render as stacked colored
//! segments (bars) and intensity-shaded cells (heatmap).

pub mod agents;
pub mod heatmap;

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

/// The dim color used for empty heatmap cells. Kept visible enough that the
/// small `■` squares still read against the background.
pub fn empty_cell() -> Color {
    Color::Rgb(75, 83, 94)
}
