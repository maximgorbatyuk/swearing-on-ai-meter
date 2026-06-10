//! Per-weekday (Mon–Sun) stacked bar chart.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::stacked_bars;
use crate::stats::{weekday_labels, Dashboard};

pub fn render(buf: &mut Buffer, area: Rect, dash: &Dashboard) {
    let columns: Vec<_> = dash.weekday.to_vec();
    let labels: Vec<String> = weekday_labels().iter().map(|s| s.to_string()).collect();
    stacked_bars(buf, area, &columns, &labels, 2, 1);
}
