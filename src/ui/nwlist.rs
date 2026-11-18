//! Network list widget rendering access point rows with badges and signals.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

use crate::app::State;
use crate::nm::model::Security;
use crate::ui::anim::scan_indicator;
use crate::ui::theme::Theme;

pub fn render_network_list(state: &State, theme: &Theme, area: Rect, buf: &mut Buffer) {
    let title = if state.scanning {
        format!(" Networks {} Scanning... ", scan_indicator(state.scan_anim_progress))
    } else {
        format!(" Networks ({} found) ", state.visible_aps().len())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(title, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    block.render(area, buf);

    let aps = state.visible_aps();
    if aps.is_empty() {
        let msg = if state.filter.is_empty() {
            "No networks found. Press 's' to scan."
        } else {
            "No networks match the current filter."
        };
        let p = Paragraph::new(msg).style(Style::default().fg(theme.muted));
        p.render(inner, buf);
        return;
    }

    let items: Vec<ListItem> = aps
        .iter()
        .enumerate()
        .map(|(idx, ap)| {
            let is_selected = idx == state.selected_index;
            let active_marker = if ap.active { "● " } else { "  " };

            let name = ap.ssid_display();
            let mut truncated_name = name;
            let max_name_len = (inner.width.saturating_sub(28) as usize).max(8);
            if truncated_name.len() > max_name_len {
                truncated_name.truncate(max_name_len.saturating_sub(1));
                truncated_name.push('…');
            }

            let signal_glyph = Theme::signal_bar_glyph(ap.strength);
            let signal_color = theme.signal_color(ap.strength);

            let sec_badge = match ap.security {
                Security::Open => Span::styled(" Open ", Style::default().fg(theme.warn)),
                Security::Wpa2 => Span::styled(" WPA2 ", Style::default().fg(theme.text)),
                Security::Wpa3 => Span::styled(" WPA3 ", Style::default().fg(theme.good)),
                Security::Enterprise => Span::styled(" Ent  ", Style::default().fg(theme.bad)),
                Security::Unsupported => Span::styled(" Unk  ", Style::default().fg(theme.muted)),
            };

            let band_badge = match ap.band() {
                crate::nm::model::Band::TwoGhz => "2.4G",
                crate::nm::model::Band::FiveGhz => " 5G ",
                crate::nm::model::Band::SixGhz => " 6G ",
                crate::nm::model::Band::Unknown => " ?? ",
            };

            let mut style = Style::default().fg(theme.text);
            if is_selected {
                style = style
                    .bg(theme.selection_bg)
                    .fg(theme.selection_fg)
                    .add_modifier(Modifier::BOLD);
            }

            let line = Line::from(vec![
                Span::styled(active_marker, Style::default().fg(theme.good)),
                Span::raw(format!("{truncated_name:<max_name_len$} ")),
                Span::styled(signal_glyph, Style::default().fg(signal_color)),
                Span::raw(format!(" {:>3}% ", ap.strength)),
                sec_badge,
                Span::styled(band_badge, Style::default().fg(theme.muted)),
            ]);

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items);
    list.render(inner, buf);
}
