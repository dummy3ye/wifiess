//! Confirmation modal for destructive actions (e.g. forgetting saved networks).

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::app::Popup;
use crate::ui::theme::Theme;

/// Render the confirmation dialog modal.
pub fn render_confirm_popup(popup: &Popup, theme: &Theme, area: Rect, buf: &mut Buffer) {
    let Popup::Confirm { message, .. } = popup else {
        return;
    };

    let popup_width = 58.min(area.width.saturating_sub(4)).max(36);
    let popup_height = 8.min(area.height.saturating_sub(2)).max(6);

    let popup_area = Rect {
        x: area.x + (area.width.saturating_sub(popup_width)) / 2,
        y: area.y + (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    // Erase content underneath the modal popup
    Clear.render(popup_area, buf);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.warn))
        .title(Span::styled(
            " Forget Network ",
            Style::default().fg(theme.warn).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(popup_area);
    block.render(popup_area, buf);

    let lines = vec![
        Line::from(vec![
            Span::styled("⚠ ", Style::default().fg(theme.warn).add_modifier(Modifier::BOLD)),
            Span::styled(message.as_str(), Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().fg(theme.bad).add_modifier(Modifier::BOLD)),
            Span::styled(" or ", Style::default().fg(theme.muted)),
            Span::styled("y", Style::default().fg(theme.bad).add_modifier(Modifier::BOLD)),
            Span::styled(": Confirm  ", Style::default().fg(theme.muted)),
            Span::styled("Esc", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" or ", Style::default().fg(theme.muted)),
            Span::styled("n", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(": Cancel", Style::default().fg(theme.muted)),
        ]),
    ];

    let p = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: true });
    p.render(inner, buf);
}
