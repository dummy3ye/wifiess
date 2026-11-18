//! Toast notification system for transient UI messages.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::time::{Duration, Instant};

/// Severity level of a toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastSeverity {
    /// Informational message (blue/cyan).
    Info,
    /// Success message (green).
    Success,
    /// Warning message (yellow).
    Warning,
    /// Error message (red).
    Error,
}

impl ToastSeverity {
    /// Color associated with this toast severity.
    pub fn color(&self) -> Color {
        match self {
            Self::Info => Color::Cyan,
            Self::Success => Color::Green,
            Self::Warning => Color::Yellow,
            Self::Error => Color::Red,
        }
    }

    /// Prefix icon for the toast.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Info => "ℹ ",
            Self::Success => "✓ ",
            Self::Warning => "⚠ ",
            Self::Error => "✗ ",
        }
    }
}

/// A transient notification toast.
#[derive(Debug, Clone)]
pub struct Toast {
    /// The message to display.
    pub message: String,
    /// Severity level.
    pub severity: ToastSeverity,
    /// When the toast was created.
    pub created_at: Instant,
    /// Lifespan duration.
    pub duration: Duration,
}

impl Toast {
    /// Create a new toast with default 4-second duration.
    pub fn new(message: String, severity: ToastSeverity) -> Self {
        Self { message, severity, created_at: Instant::now(), duration: Duration::from_secs(4) }
    }

    /// Create a new toast with custom duration.
    pub fn with_duration(message: String, severity: ToastSeverity, duration: Duration) -> Self {
        Self { message, severity, created_at: Instant::now(), duration }
    }

    /// Check if the toast is still alive (not expired).
    pub fn alive(&self) -> bool {
        self.created_at.elapsed() < self.duration
    }
}

/// Widget to render a list of toasts stacked in a corner.
pub struct ToastStack<'a> {
    toasts: &'a [Toast],
}

impl<'a> ToastStack<'a> {
    /// Create a new toast stack widget.
    pub fn new(toasts: &'a [Toast]) -> Self {
        Self { toasts }
    }
}

impl Widget for ToastStack<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.toasts.is_empty() || area.width < 10 || area.height < 3 {
            return;
        }

        let mut y = area.y + 1;
        for toast in self.toasts.iter().rev() {
            if y + 3 > area.bottom() {
                break;
            }

            let text = format!("{}{}", toast.severity.icon(), toast.message);
            let toast_width = ((text.len() + 4) as u16).min(area.width.saturating_sub(4)).max(20);
            let toast_area = Rect {
                x: area.right().saturating_sub(toast_width + 2),
                y,
                width: toast_width,
                height: 3,
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(toast.severity.color()))
                .style(Style::default().bg(Color::Reset));

            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(toast.severity.color()).add_modifier(Modifier::BOLD));

            paragraph.render(toast_area, buf);
            y += 3;
        }
    }
}
