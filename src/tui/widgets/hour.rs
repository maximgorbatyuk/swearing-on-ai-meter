//! Per-hour (0–23) stacked bar chart.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::stacked_bars;
use crate::stats::Dashboard;

pub fn render(buf: &mut Buffer, area: Rect, dash: &Dashboard) {
    let columns: Vec<_> = dash.hourly.to_vec();
    // Sparse axis: label 0, 6, 12, 18 only, to avoid clutter under 1-wide bars.
    let labels: Vec<String> = (0..24)
        .map(|h| {
            if h % 6 == 0 {
                format!("{h}")
            } else {
                String::new()
            }
        })
        .collect();
    stacked_bars(buf, area, &columns, &labels, 1, 0);
}
