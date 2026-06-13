//! Per-agent comparison: total swears per app over the last 14 days, drawn as
//! horizontal bars in each app's color with the count alongside. Always shows
//! all five apps regardless of the active filter, so it reads as a comparison.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use super::{empty_cell, put_str, source_color};
use crate::model::Source;
use crate::stats::Dashboard;

pub fn render(buf: &mut Buffer, area: Rect, dash: &Dashboard) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let data = &dash.agents_14d;
    let max = data.iter().copied().max().unwrap_or(0);

    let label_w = Source::ALL
        .iter()
        .map(|s| s.short().len() as u16)
        .max()
        .unwrap_or(0);
    let val_w = max.to_string().len().max(1) as u16;
    let label_style = Style::new().fg(Color::Rgb(140, 140, 140));

    let gap = 1u16;
    let bar_x = area.x + label_w + gap;
    // Reserve room on the right for "<gap><count>".
    let bar_max = area.width.saturating_sub(label_w + gap + gap + val_w);

    // Center the five rows vertically, spacing them out if there's room.
    let n = Source::ALL.len() as u16;
    let step = if area.height >= n * 2 { 2 } else { 1 };
    let used = (n - 1) * step + 1;
    let top = area.y + area.height.saturating_sub(used) / 2;

    for (i, src) in Source::ALL.iter().enumerate() {
        let y = top + i as u16 * step;
        if y >= area.bottom() {
            break;
        }
        let count = data[src.index()];
        let color = source_color(*src);

        put_str(buf, area, area.x, y, src.short(), label_style);

        let bar_len = if max == 0 || count == 0 {
            0
        } else {
            (((count as f32 / max as f32) * bar_max as f32).round() as u16)
                .max(1)
                .min(bar_max)
        };

        let val_x = if bar_len == 0 {
            // Faint placeholder so empty agents still have a marker.
            put_str(buf, area, bar_x, y, "░", Style::new().fg(empty_cell()));
            bar_x + 1 + gap
        } else {
            let bar: String = "█".repeat(bar_len as usize);
            put_str(buf, area, bar_x, y, &bar, Style::new().fg(color));
            bar_x + bar_len + gap
        };

        let val_style = if count == 0 {
            Style::new().fg(empty_cell())
        } else {
            Style::new().fg(color)
        };
        put_str(buf, area, val_x, y, &count.to_string(), val_style);
    }
}
