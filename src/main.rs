//! wifiess - A remarkable `WiFi` manager for the terminal.

#![forbid(unsafe_code)]

mod action;
mod app;
mod config;
mod event;
mod nm;
mod tui;
mod ui;

use clap::Parser;
use color_eyre::Result;
use crossterm::event::KeyCode;

use crate::action::Action;
use crate::app::State;
use crate::event::{Event, EventLoop};
use crate::tui::Tui;

/// CLI arguments for wifiess.
#[derive(Parser, Debug)]
#[command(name = "wifiess", version, about = "A remarkable WiFi manager for the terminal")]
struct Cli {
    /// Run with synthetic demo `WiFi` data without connecting to `NetworkManager`.
    #[arg(long)]
    demo: bool,

    /// Specify log level (error, warn, info, debug, trace).
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let config = config::load_config();
    let (backend_handle, backend_rx) = nm::spawn(cli.demo);

    let mut state = State::new(config);
    let mut tui = Tui::new(state.config.mouse)?;
    let mut events = EventLoop::new(backend_rx);

    let initial_size = tui.size()?;
    state.term_width = initial_size.width;
    state.term_height = initial_size.height;

    while !state.should_quit {
        if state.dirty {
            tui.terminal_mut().draw(|frame| {
                ui::view(&state, frame.area(), frame.buffer_mut());
            })?;
            state.dirty = false;
        }

        events.set_animating(state.animating);

        let event = events.next().await?;

        // When a text-input popup or filter is active, most keys become character input.
        let text_input_active = matches!(
            state.popup,
            Some(app::Popup::Password { .. } | app::Popup::HiddenSsid { .. })
        ) || state.filter_active;

        let is_confirm_active = matches!(state.popup, Some(app::Popup::Confirm { .. }));

        let maybe_action = match event {
            Event::Key(key) => {
                if is_confirm_active {
                    // Confirmation dialog mode (e.g. forget network)
                    match key.code {
                        KeyCode::Char('c')
                            if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            Some(Action::Quit)
                        }
                        KeyCode::Enter | KeyCode::Char('y' | 'Y') => Some(Action::Confirm),
                        KeyCode::Esc | KeyCode::Char('n' | 'N') => Some(Action::Cancel),
                        _ => None,
                    }
                } else if text_input_active {
                    // In text-input mode, only a few keys have special meaning.
                    match key.code {
                        KeyCode::Char('c')
                            if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            Some(Action::Quit)
                        }
                        KeyCode::Esc => Some(Action::Cancel),
                        KeyCode::Enter => {
                            if state.popup.is_some() {
                                Some(Action::SubmitPassword)
                            } else {
                                // Filter mode: deactivate filter and activate selected
                                state.filter_active = false;
                                Some(Action::Activate)
                            }
                        }
                        KeyCode::Tab => Some(Action::TogglePasswordReveal),
                        KeyCode::Backspace => Some(Action::Backspace),
                        KeyCode::Char(c) => Some(Action::CharInput(c)),
                        KeyCode::Down => Some(Action::SelectNext),
                        KeyCode::Up => Some(Action::SelectPrev),
                        _ => None,
                    }
                } else {
                    // Normal mode: full keybindings.
                    match key.code {
                        KeyCode::Char('c')
                            if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                        {
                            Some(Action::Quit)
                        }
                        KeyCode::Esc => Some(Action::Cancel),
                        KeyCode::Enter => Some(Action::Activate),
                        KeyCode::Tab => Some(Action::TogglePasswordReveal),
                        KeyCode::Char('j') | KeyCode::Down => Some(Action::SelectNext),
                        KeyCode::Char('k') | KeyCode::Up => Some(Action::SelectPrev),
                        KeyCode::Char('g') | KeyCode::Home => Some(Action::SelectFirst),
                        KeyCode::Char('G') | KeyCode::End => Some(Action::SelectLast),
                        KeyCode::Char('s') => Some(Action::Scan),
                        KeyCode::Char('d') => Some(Action::Disconnect),
                        KeyCode::Char('f') => Some(Action::ForgetSelected),
                        KeyCode::Char('?') => Some(Action::ToggleHelp),
                        KeyCode::Char('S') => Some(Action::ToggleSpectrum),
                        KeyCode::Char('/') => Some(Action::StartFilter),
                        KeyCode::Char('o') => Some(Action::CycleSort),
                        KeyCode::Char('t') => Some(Action::NextTheme),
                        KeyCode::Char('h') => Some(Action::ConnectHidden),
                        KeyCode::Char('q') => Some(Action::Quit),
                        KeyCode::Backspace => Some(Action::Backspace),
                        _ => None,
                    }
                }
            }
            Event::Resize(w, h) => Some(Action::Resize { width: w, height: h }),
            Event::Tick => Some(Action::Tick),
            Event::Backend(nm_evt) => Some(Action::BackendEvent(nm_evt)),
            _ => None,
        };

        if let Some(action) = maybe_action {
            if let Some(cmd) = state.update(action) {
                let _ = backend_handle.send(cmd).await;
            }
        }
    }

    Ok(())
}
