//! Theme definitions and color systems.

use ratatui::style::Color;

/// Semantic theme color palette.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme name.
    pub name: &'static str,
    /// Background color.
    pub bg: Color,
    /// Card / Surface background color.
    pub surface: Color,
    /// Border color.
    pub border: Color,
    /// Primary text color.
    pub text: Color,
    /// Muted / secondary text color.
    pub muted: Color,
    /// Accent color.
    pub accent: Color,
    /// Good / success status.
    pub good: Color,
    /// Warning status.
    pub warn: Color,
    /// Bad / error status.
    pub bad: Color,
    /// Row selection highlight background.
    pub selection_bg: Color,
    /// Row selection highlight text color.
    pub selection_fg: Color,
}

impl Theme {
    /// Dark default theme.
    pub fn dark() -> Self {
        Self {
            name: "Dark (Default)",
            bg: Color::Reset,
            surface: Color::Rgb(24, 25, 38),
            border: Color::Rgb(110, 115, 141),
            text: Color::Rgb(202, 211, 245),
            muted: Color::Rgb(128, 135, 162),
            accent: Color::Rgb(138, 173, 244),
            good: Color::Rgb(166, 218, 149),
            warn: Color::Rgb(238, 212, 159),
            bad: Color::Rgb(237, 135, 150),
            selection_bg: Color::Rgb(54, 58, 79),
            selection_fg: Color::Rgb(244, 184, 228),
        }
    }

    /// Clean light theme.
    pub fn light() -> Self {
        Self {
            name: "Light",
            bg: Color::Rgb(239, 241, 245),
            surface: Color::Rgb(230, 233, 239),
            border: Color::Rgb(172, 176, 190),
            text: Color::Rgb(76, 79, 105),
            muted: Color::Rgb(156, 160, 176),
            accent: Color::Rgb(30, 102, 245),
            good: Color::Rgb(64, 160, 43),
            warn: Color::Rgb(223, 142, 29),
            bad: Color::Rgb(210, 15, 57),
            selection_bg: Color::Rgb(204, 208, 218),
            selection_fg: Color::Rgb(114, 56, 174),
        }
    }

    /// Muted pastel / Nord-inspired theme.
    pub fn pastel() -> Self {
        Self {
            name: "Nordic Pastel",
            bg: Color::Rgb(46, 52, 64),
            surface: Color::Rgb(59, 66, 82),
            border: Color::Rgb(76, 86, 106),
            text: Color::Rgb(236, 239, 244),
            muted: Color::Rgb(216, 222, 233),
            accent: Color::Rgb(136, 192, 208),
            good: Color::Rgb(163, 190, 140),
            warn: Color::Rgb(235, 203, 139),
            bad: Color::Rgb(191, 97, 106),
            selection_bg: Color::Rgb(67, 76, 94),
            selection_fg: Color::Rgb(143, 188, 187),
        }
    }

    /// High contrast accessibility theme.
    pub fn high_contrast() -> Self {
        Self {
            name: "High Contrast",
            bg: Color::Black,
            surface: Color::Black,
            border: Color::White,
            text: Color::White,
            muted: Color::Gray,
            accent: Color::Yellow,
            good: Color::Green,
            warn: Color::Yellow,
            bad: Color::Red,
            selection_bg: Color::White,
            selection_fg: Color::Black,
        }
    }

    /// Map signal strength (0-100) to a smooth color ramp.
    pub fn signal_color(&self, strength: u8) -> Color {
        match strength {
            0..=25 => self.bad,
            26..=55 => self.warn,
            56..=75 => self.accent,
            _ => self.good,
        }
    }

    /// Get signal bar glyph based on strength (0-100).
    pub fn signal_bar_glyph(strength: u8) -> &'static str {
        match strength {
            0..=15 => "____",
            16..=35 => "▂___",
            36..=60 => "▂▄__",
            61..=85 => "▂▄▆_",
            _ => "▂▄▆█",
        }
    }
}
