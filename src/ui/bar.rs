//! Status bar widget rendering connection state and quick-help shortcuts.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::app::State;
use crate::nm::model::ConnState;
use crate::ui::theme::Theme;

pub fn render_status_bar(state: &State, theme: &Theme, area: Rect, buf: &mut Buffer) {
    let (state_icon, state_text, state_color) = match &state.conn_state {
        ConnState::Connected { ssid, ip } => {
            let ssid_str = String::from_utf8_lossy(ssid);
            let ip_str = ip.as_deref().unwrap_or("No IP");
            ("●", format!(" Connected to {ssid_str} ({ip_str})"), theme.good)
        }
        ConnState::Connecting { ssid, stage } => {
            let ssid_str = String::from_utf8_lossy(ssid);
            ("◌", format!(" Connecting to {ssid_str}: {stage}"), theme.warn)
        }
        ConnState::Disconnecting => ("◌", " Disconnecting...".to_string(), theme.warn),
        ConnState::Disconnected => ("○", " Disconnected".to_string(), theme.muted),
    };

    let filter_text = if state.filter_active {
        format!(" | Filter: {}_ ", state.filter)
    } else if !state.filter.is_empty() {
        format!(" | Filter: {} ", state.filter)
    } else {
        String::new()
    };

    let shortcuts =
        " [⏎ connect] [s scan] [d disconnect] [/ filter] [o sort] [t theme] [? help] [q quit]";

    let line = Line::from(vec![
        Span::styled(state_icon, Style::default().fg(state_color).add_modifier(Modifier::BOLD)),
        Span::styled(state_text, Style::default().fg(state_color)),
        Span::styled(filter_text, Style::default().fg(theme.accent)),
        Span::styled(shortcuts, Style::default().fg(theme.muted)),
    ]);

    let p = Paragraph::new(line);
    p.render(area, buf);
}
