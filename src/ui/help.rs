//! Help overlay modal displaying application keybindings.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::ui::theme::Theme;

pub fn render_help_overlay(theme: &Theme, area: Rect, buf: &mut Buffer) {
    let width = 64.min(area.width.saturating_sub(4)).max(30);
    let height = 22.min(area.height.saturating_sub(2)).max(10);

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    Clear.render(popup_area, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(
            " Keybindings & Navigation ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(popup_area);
    block.render(popup_area, buf);

    let bindings = [
        ("j / ↓", "Select next network"),
        ("k / ↑", "Select previous network"),
        ("g / Home", "Jump to top of list"),
        ("G / End", "Jump to bottom of list"),
        ("Enter", "Connect to selected network"),
        ("s", "Trigger WiFi scan"),
        ("d", "Disconnect current network"),
        ("f", "Forget saved network"),
        ("/", "Filter networks by SSID"),
        ("o", "Cycle sort mode (Signal, Name, Known)"),
        ("S", "Toggle channel spectrum view"),
        ("t", "Cycle theme (Dark, Light, Pastel, High Contrast)"),
        ("h", "Connect to hidden SSID"),
        ("?", "Toggle this help overlay"),
        ("Esc", "Dismiss modal / clear filter"),
        ("q / Ctrl+C", "Quit wifiess"),
    ];

    let mut lines = Vec::new();
    for (key, desc) in bindings {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{key:<14}"),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(desc, Style::default().fg(theme.text)),
        ]));
    }

    let p = Paragraph::new(lines).alignment(Alignment::Left);
    p.render(inner, buf);
}
