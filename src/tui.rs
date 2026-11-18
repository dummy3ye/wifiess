//! Terminal initialization, restoration, and capability detection.
//!
//! Provides [`Tui`] which manages raw mode, alternate screen,
//! and installs a panic hook to ensure the terminal is always restored.

use std::io::{self, Stdout, stdout};

use color_eyre::Result;
use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

/// Color depth capability of the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorDepth {
    /// 24-bit true color.
    TrueColor,
    /// 256 colors.
    Color256,
    /// 16 ANSI colors.
    Color16,
    /// No color (monochrome).
    NoColor,
}

impl ColorDepth {
    /// Detect the terminal's color depth from environment variables.
    pub fn detect() -> Self {
        // NO_COLOR takes precedence
        if std::env::var("NO_COLOR").is_ok() {
            return Self::NoColor;
        }

        // Check COLORTERM for truecolor
        if let Ok(ct) = std::env::var("COLORTERM") {
            if ct == "truecolor" || ct == "24bit" {
                return Self::TrueColor;
            }
        }

        // Check TERM for 256color
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("256color") {
                return Self::Color256;
            }
        }

        Self::Color16
    }
}

/// Whether the terminal supports Unicode box-drawing characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnicodeSupport {
    /// Full Unicode (rounded borders, braille, emoji).
    Full,
    /// ASCII only.
    AsciiOnly,
}

impl UnicodeSupport {
    /// Detect Unicode support from environment variables.
    pub fn detect() -> Self {
        if let Ok(lang) = std::env::var("LANG") {
            if lang.to_lowercase().contains("utf") {
                return Self::Full;
            }
        }
        if let Ok(lc) = std::env::var("LC_ALL") {
            if lc.to_lowercase().contains("utf") {
                return Self::Full;
            }
        }
        // Default to full — most modern terminals support UTF-8
        Self::Full
    }
}

/// Terminal wrapper that manages raw mode, alternate screen, and cleanup.
pub struct Tui {
    /// The ratatui terminal.
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Whether mouse capture is enabled.
    mouse_enabled: bool,
}

impl Tui {
    /// Create a new Tui, entering raw mode and the alternate screen.
    ///
    /// Installs a panic hook to ensure terminal cleanup on panic.
    pub fn new(enable_mouse: bool) -> Result<Self> {
        // Install panic hook BEFORE entering raw mode
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            // Best-effort terminal restore on panic
            let _ = terminal::disable_raw_mode();
            let _ = crossterm::execute!(
                io::stdout(),
                LeaveAlternateScreen,
                DisableMouseCapture,
                cursor::Show,
            );
            original_hook(panic_info);
        }));

        terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        crossterm::execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

        if enable_mouse {
            crossterm::execute!(stdout, EnableMouseCapture)?;
        }

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal, mouse_enabled: enable_mouse })
    }

    /// Get a mutable reference to the ratatui terminal for drawing.
    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    /// Get the current terminal size.
    pub fn size(&self) -> Result<ratatui::layout::Rect> {
        let size = self.terminal.size()?;
        Ok(ratatui::layout::Rect::new(0, 0, size.width, size.height))
    }

    /// Restore the terminal to its original state.
    fn restore(&mut self) {
        let _ = terminal::disable_raw_mode();
        let mut stdout = stdout();
        if self.mouse_enabled {
            let _ = crossterm::execute!(stdout, DisableMouseCapture);
        }
        let _ = crossterm::execute!(stdout, LeaveAlternateScreen, cursor::Show);
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.restore();
    }
}
