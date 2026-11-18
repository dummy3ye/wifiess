//! Detail pane for selected network.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::State;
use crate::ui::theme::Theme;

pub fn render_detail(state: &State, theme: &Theme, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            " Network Details ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    block.render(area, buf);

    let Some(ap) = state.selected_ap() else {
        let p = Paragraph::new("No network selected").style(Style::default().fg(theme.muted));
        p.render(inner, buf);
        return;
    };

    let channel_str = ap.channel().map_or_else(|| "Unknown".to_string(), |c| c.to_string());

    let bssid_count = ap.bssids.len();

    let mut lines = vec![
        Line::from(vec![
            Span::styled("SSID:       ", Style::default().fg(theme.muted)),
            Span::styled(
                ap.ssid_display(),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Signal:     ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} ({}%)", Theme::signal_bar_glyph(ap.strength), ap.strength),
                Style::default().fg(theme.signal_color(ap.strength)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Frequency:  ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} MHz (Channel {})", ap.freq_mhz, channel_str),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("Band:       ", Style::default().fg(theme.muted)),
            Span::styled(format!("{}", ap.band()), Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("Security:   ", Style::default().fg(theme.muted)),
            Span::styled(format!("{}", ap.security), Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("Known:      ", Style::default().fg(theme.muted)),
            Span::styled(
                if ap.known { "Yes (Saved)" } else { "No" },
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("Active:     ", Style::default().fg(theme.muted)),
            Span::styled(
                if ap.active { "Yes (Connected)" } else { "No" },
                Style::default().fg(if ap.active { theme.good } else { theme.text }),
            ),
        ]),
        Line::from(vec![
            Span::styled("BSSIDs:     ", Style::default().fg(theme.muted)),
            Span::styled(format!("{bssid_count} access point(s)"), Style::default().fg(theme.text)),
        ]),
    ];

    if let Some(history) = state.signal_history.get(&ap.ssid) {
        let history_str: String = history
            .iter()
            .map(|s| match s {
                0..=20 => " ",
                21..=40 => "▂",
                41..=60 => "▄",
                61..=80 => "▆",
                _ => "█",
            })
            .collect();
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Signal History: ", Style::default().fg(theme.muted)),
            Span::styled(history_str, Style::default().fg(theme.accent)),
        ]));
    }

    let p = Paragraph::new(lines);
    p.render(inner, buf);
}
