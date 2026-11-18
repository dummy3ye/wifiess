//! Event loop: merges crossterm events, tick timer, and backend events.

use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{self as ct_event, Event as CrosstermEvent, KeyEvent, MouseEvent};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::nm::NmEvent;

/// Application events from all sources.
#[derive(Debug)]
pub enum Event {
    /// A key press.
    Key(KeyEvent),
    /// A mouse event.
    Mouse(MouseEvent),
    /// Terminal was resized.
    Resize(u16, u16),
    /// Periodic tick for animations and idle updates.
    Tick,
    /// An event from the NM backend.
    Backend(NmEvent),
    /// A render tick (for animations).
    Render,
}

/// The event loop that merges all event sources.
pub struct EventLoop {
    /// Receiver for backend NM events.
    backend_rx: mpsc::Receiver<NmEvent>,
    /// The slow tick rate (idle).
    idle_tick: Duration,
    /// The fast tick rate (animating).
    anim_tick: Duration,
    /// Whether animations are currently running.
    animating: bool,
}

impl EventLoop {
    /// Create a new event loop.
    pub fn new(backend_rx: mpsc::Receiver<NmEvent>) -> Self {
        Self {
            backend_rx,
            idle_tick: Duration::from_secs(1),
            anim_tick: Duration::from_millis(33), // ~30fps
            animating: false,
        }
    }

    /// Set whether animations are active (affects tick rate).
    pub fn set_animating(&mut self, animating: bool) {
        self.animating = animating;
    }

    /// Wait for and return the next event.
    pub async fn next(&mut self) -> Result<Event> {
        let tick_rate = if self.animating { self.anim_tick } else { self.idle_tick };

        let mut reader = ct_event::EventStream::new();

        loop {
            let crossterm_event = reader.next();
            let tick_delay = tokio::time::sleep(tick_rate);

            tokio::select! {
                maybe_event = crossterm_event => {
                    match maybe_event {
                        Some(Ok(evt)) => {
                            match evt {
                                CrosstermEvent::Key(key) => {
                                    // Only process key press events
                                    if key.kind == ct_event::KeyEventKind::Press {
                                        return Ok(Event::Key(key));
                                    }
                                }
                                CrosstermEvent::Mouse(mouse) => {
                                    return Ok(Event::Mouse(mouse));
                                }
                                CrosstermEvent::Resize(w, h) => {
                                    return Ok(Event::Resize(w, h));
                                }
                                _ => {}
                            }
                        }
                        Some(Err(e)) => {
                            return Err(e.into());
                        }
                        None => {
                            // Stream ended
                            return Err(color_eyre::eyre::eyre!("Event stream ended"));
                        }
                    }
                }
                Some(nm_event) = self.backend_rx.recv() => {
                    return Ok(Event::Backend(nm_event));
                }
                () = tick_delay => {
                    return Ok(Event::Tick);
                }
            }
        }
    }
}
