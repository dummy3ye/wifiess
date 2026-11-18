//! Top-level UI layout and view renderer.

pub mod anim;
pub mod confirm;
pub mod detail;
pub mod help;
pub mod nwlist;
pub mod password;
pub mod schart;
pub mod spectrum;
pub mod status_bar;
pub mod theme;
pub mod toast;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
};

use self::theme::Theme;
use crate::app::{Popup, State};

/// Layout presentation mode based on terminal dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    Medium,
    Narrow,
    TooSmall,
}

impl LayoutMode {
    pub fn from_size(width: u16, height: u16) -> Self {
        if width < 40 || height < 10 {
            Self::TooSmall
        } else if width >= 100 {
            Self::Wide
        } else if width >= 60 {
            Self::Medium
        } else {
            Self::Narrow
        }
    }
}

/// Render the complete user interface to the ratatui buffer.
pub fn view(state: &State, area: Rect, buf: &mut Buffer) {
    let themes = [Theme::dark(), Theme::light(), Theme::pastel(), Theme::high_contrast()];
    let theme = &themes[state.theme_index % themes.len()];

    let mode = LayoutMode::from_size(area.width, area.height);

    if mode == LayoutMode::TooSmall {
        let msg = "Terminal too small for wifiess.\nPlease enlarge window.";
        let p = Paragraph::new(msg).style(Style::default().fg(theme.warn));
        p.render(area, buf);
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(1)])
        .split(area);

    let content_area = main_chunks[0];
    let status_area = main_chunks[1];

    match mode {
        LayoutMode::Wide => {
            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints(if state.show_spectrum {
                    vec![Constraint::Percentage(70), Constraint::Percentage(30)]
                } else {
                    vec![Constraint::Percentage(100)]
                })
                .split(content_area);

            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(vert[0]);

            nwlist::render_network_list(state, theme, columns[0], buf);
            detail::render_detail(state, theme, columns[1], buf);

            if state.show_spectrum && vert.len() > 1 {
                spectrum::render_spectrum(state, theme, vert[1], buf);
            }
        }
        LayoutMode::Medium => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(content_area);

            nwlist::render_network_list(state, theme, rows[0], buf);
            if state.show_spectrum {
                spectrum::render_spectrum(state, theme, rows[1], buf);
            } else {
                detail::render_detail(state, theme, rows[1], buf);
            }
        }
        LayoutMode::Narrow => {
            nwlist::render_network_list(state, theme, content_area, buf);
        }
        LayoutMode::TooSmall => unreachable!(),
    }

    status_bar::render_status_bar(state, theme, status_area, buf);

    if let Some(ref popup) = state.popup {
        match popup {
            Popup::Password { .. } => {
                password::render_password_popup(popup, theme, area, buf);
            }
            Popup::Help { .. } => {
                help::render_help_overlay(theme, area, buf);
            }
            Popup::Confirm { .. } => {
                confirm::render_confirm_popup(popup, theme, area, buf);
            }
            Popup::HiddenSsid { input } => {
                let p = Paragraph::new(format!("\nSSID: {input}_\n\n[Enter connect] [Esc cancel]"))
                    .style(Style::default().fg(theme.text))
                    .block(
                        Block::default().borders(Borders::ALL).title(" Connect to Hidden Network "),
                    );
                let box_area = Rect {
                    x: area.x + (area.width.saturating_sub(44)) / 2,
                    y: area.y + (area.height.saturating_sub(7)) / 2,
                    width: 44.min(area.width),
                    height: 7.min(area.height),
                };
                p.render(box_area, buf);
            }
        }
    }

    let toast_slice: Vec<_> = state.toasts.iter().cloned().collect();
    let toast_stack = toast::ToastStack::new(&toast_slice);
    toast_stack.render(area, buf);
}
