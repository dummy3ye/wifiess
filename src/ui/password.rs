//! Password input popup modal.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::app::Popup;
use crate::ui::theme::Theme;

pub fn render_password_popup(popup: &Popup, theme: &Theme, area: Rect, buf: &mut Buffer) {
    let Popup::Password { ssid_display, security, input, revealed, .. } = popup else {
        return;
    };

    let popup_width = 54.min(area.width.saturating_sub(4)).max(30);
    let popup_height = 9.min(area.height.saturating_sub(2)).max(7);

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    Clear.render(popup_area, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(
            " Connect to Network ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(popup_area);
    block.render(popup_area, buf);

    let masked_input = if *revealed { input.clone() } else { "•".repeat(input.len()) };

    let lines = vec![
        Line::from(vec![
            Span::styled("Network:  ", Style::default().fg(theme.muted)),
            Span::styled(
                ssid_display,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" ({security})"), Style::default().fg(theme.muted)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Password: ", Style::default().fg(theme.text)),
            Span::styled(
                format!("{masked_input}_"),
                Style::default().fg(theme.selection_fg).bg(theme.selection_bg),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter: ", Style::default().fg(theme.accent)),
            Span::styled("Connect  ", Style::default().fg(theme.muted)),
            Span::styled("Tab: ", Style::default().fg(theme.accent)),
            Span::styled(
                if *revealed { "Hide  " } else { "Reveal  " },
                Style::default().fg(theme.muted),
            ),
            Span::styled("Esc: ", Style::default().fg(theme.accent)),
            Span::styled("Cancel", Style::default().fg(theme.muted)),
        ]),
    ];

    let p = Paragraph::new(lines).alignment(Alignment::Left);
    p.render(inner, buf);
}
