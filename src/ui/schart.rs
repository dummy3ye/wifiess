//! Signal chart rendering historical RSSI values.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Paragraph, Widget},
};

use crate::ui::theme::Theme;

/// Render a compact signal chart.
pub fn render_signal_chart(values: &[u8], theme: &Theme, area: Rect, buf: &mut Buffer) {
    if values.is_empty() || area.width == 0 || area.height == 0 {
        return;
    }

    let chart_str: String = values
        .iter()
        .map(|&v| match v {
            0..=20 => " ",
            21..=40 => "▂",
            41..=60 => "▄",
            61..=80 => "▆",
            _ => "█",
        })
        .collect();

    let p = Paragraph::new(chart_str).style(Style::default().fg(theme.accent));
    p.render(area, buf);
}
