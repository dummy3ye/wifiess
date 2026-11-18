//! Spectrum / channel allocation view across 2.4 GHz, 5 GHz, and 6 GHz bands.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::State;
use crate::nm::model::Band;
use crate::ui::theme::Theme;

pub fn render_spectrum(state: &State, theme: &Theme, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(
            " 2.4 GHz Spectrum & Channel Overlap ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.height < 3 || inner.width < 20 {
        return;
    }

    let channels = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    let mut channel_counts = [0usize; 12]; // 1-11

    for ap in state.visible_aps() {
        if ap.band() == Band::TwoGhz {
            if let Some(ch) = ap.channel() {
                if (1..=11).contains(&ch) {
                    channel_counts[ch as usize] += 1;
                }
            }
        }
    }

    let mut header_spans = Vec::new();
    header_spans.push(Span::raw("Channel:  "));
    for ch in channels {
        let is_non_overlapping = ch == 1 || ch == 6 || ch == 11;
        let style = if is_non_overlapping {
            Style::default().fg(theme.good).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted)
        };
        header_spans.push(Span::styled(format!("{ch:>3} "), style));
    }

    let mut count_spans = Vec::new();
    count_spans.push(Span::raw("Networks: "));
    for ch in channels {
        let count = channel_counts[ch as usize];
        let color = match count {
            0 => theme.good,
            1..=2 => theme.accent,
            3..=4 => theme.warn,
            _ => theme.bad,
        };
        count_spans.push(Span::styled(format!("{count:>3} "), Style::default().fg(color)));
    }

    let lines = vec![
        Line::from(header_spans),
        Line::from(count_spans),
        Line::from(vec![
            Span::styled("Hint: ", Style::default().fg(theme.accent)),
            Span::styled(
                "Channels 1, 6, and 11 do not overlap. Choose the channel with lowest network count.",
                Style::default().fg(theme.muted),
            ),
        ]),
    ];

    let p = Paragraph::new(lines);
    p.render(inner, buf);
}
