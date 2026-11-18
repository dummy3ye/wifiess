//! Application state and update reducer.
//!
//! This is the single source of truth for the UI. The [`State`] struct holds
//! all application state, and [`State::update`] is the only function that mutates it.

use std::collections::{HashMap, VecDeque};
use std::time::Instant;

pub use crate::action::Action;
use crate::config::AppConfig;
use crate::nm::model::{AccessPoint, ApId, ConnId, ConnState, Security};
use crate::nm::{Command, NmError, NmEvent};
use crate::ui::toast::{Toast, ToastSeverity};

/// How the network list is sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    /// By signal strength (strongest first).
    Signal,
    /// Alphabetically by SSID.
    Name,
    /// Known networks first, then by signal.
    KnownFirst,
}

impl SortMode {
    /// Cycle to the next sort mode.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Signal => Self::Name,
            Self::Name => Self::KnownFirst,
            Self::KnownFirst => Self::Signal,
        }
    }

    /// Display label.
    pub fn label(&self) -> &str {
        match self {
            Self::Signal => "Signal",
            Self::Name => "Name",
            Self::KnownFirst => "Known",
        }
    }
}

/// The currently active popup/modal.
#[derive(Debug, Clone)]
pub enum Popup {
    /// Password input dialog.
    Password {
        /// The AP we're connecting to.
        ap_id: ApId,
        /// The SSID for display.
        ssid_display: String,
        /// The security type.
        security: Security,
        /// Current input text.
        input: String,
        /// Whether password is visible.
        revealed: bool,
    },
    /// Confirmation dialog (e.g., forget network).
    Confirm {
        /// The message to show.
        message: String,
        /// Action to take on confirmation.
        on_confirm: ConfirmAction,
    },
    /// Help overlay.
    Help {
        /// Scroll offset.
        scroll: u16,
    },
    /// Hidden SSID input.
    HiddenSsid {
        /// Current input text.
        input: String,
    },
}

/// What to do when a confirmation is accepted.
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    /// Forget a saved network.
    ForgetNetwork(ConnId),
}

/// Application state — the single source of truth.
pub struct State {
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Whether the screen needs redrawing.
    pub dirty: bool,
    /// Whether animations are currently running.
    pub animating: bool,

    // -- Network data --
    /// All visible access points (deduped by SSID).
    pub access_points: Vec<AccessPoint>,
    /// Current connection state.
    pub conn_state: ConnState,
    /// Whether a scan is in progress.
    pub scanning: bool,

    // -- Selection --
    /// Index of the selected AP in the filtered/sorted list.
    pub selected_index: usize,
    /// SSID of the selected AP (for pinning selection across reorders).
    pub selected_ssid: Option<Vec<u8>>,

    // -- UI state --
    /// Current sort mode.
    pub sort_mode: SortMode,
    /// Filter string (empty = no filter).
    pub filter: String,
    /// Whether filter input is active.
    pub filter_active: bool,
    /// Active popup, if any.
    pub popup: Option<Popup>,
    /// Whether the spectrum view is visible.
    pub show_spectrum: bool,
    /// Toast notifications.
    pub toasts: VecDeque<Toast>,

    // -- Signal history --
    /// Per-SSID signal history (ring buffer of recent samples).
    pub signal_history: HashMap<Vec<u8>, VecDeque<u8>>,

    // -- Animation state --
    /// Scan animation progress (0.0 to 1.0).
    pub scan_anim_progress: f64,
    /// Time of last tick.
    pub last_tick: Instant,

    // -- Terminal size --
    /// Current terminal width.
    pub term_width: u16,
    /// Current terminal height.
    pub term_height: u16,

    // -- Config --
    /// Application configuration.
    pub config: AppConfig,

    // -- Theme --
    /// Current theme index.
    pub theme_index: usize,
}

impl State {
    /// Create a new state with the given configuration.
    pub fn new(config: AppConfig) -> Self {
        Self {
            should_quit: false,
            dirty: true,
            animating: false,
            access_points: Vec::new(),
            conn_state: ConnState::default(),
            scanning: false,
            selected_index: 0,
            selected_ssid: None,
            sort_mode: SortMode::Signal,
            filter: String::new(),
            filter_active: false,
            popup: None,
            show_spectrum: false,
            toasts: VecDeque::new(),
            signal_history: HashMap::new(),
            scan_anim_progress: 0.0,
            last_tick: Instant::now(),
            term_width: 80,
            term_height: 24,
            config,
            theme_index: 0,
        }
    }

    /// The filtered and sorted list of APs to display.
    pub fn visible_aps(&self) -> Vec<&AccessPoint> {
        let mut aps: Vec<&AccessPoint> = self
            .access_points
            .iter()
            .filter(|ap| {
                if self.filter.is_empty() {
                    return true;
                }
                let display = ap.ssid_display().to_lowercase();
                let filter = self.filter.to_lowercase();
                display.contains(&filter)
            })
            .filter(|ap| !ap.is_hidden())
            .collect();

        match self.sort_mode {
            SortMode::Signal => {
                aps.sort_by(|a, b| b.active.cmp(&a.active).then(b.strength.cmp(&a.strength)));
            }
            SortMode::Name => {
                aps.sort_by(|a, b| {
                    a.ssid_display().to_lowercase().cmp(&b.ssid_display().to_lowercase())
                });
            }
            SortMode::KnownFirst => {
                aps.sort_by(|a, b| {
                    b.known
                        .cmp(&a.known)
                        .then(b.active.cmp(&a.active))
                        .then(b.strength.cmp(&a.strength))
                });
            }
        }

        aps
    }

    /// Get the currently selected AP, if any.
    pub fn selected_ap(&self) -> Option<&AccessPoint> {
        let visible = self.visible_aps();
        visible.get(self.selected_index).copied()
    }

    /// Update state in response to an action. Returns an optional command to send to the backend.
    pub fn update(&mut self, action: Action) -> Option<Command> {
        self.dirty = true;

        match action {
            Action::Quit => {
                if self.popup.is_some() {
                    self.popup = None;
                } else if self.filter_active {
                    self.filter_active = false;
                    self.filter.clear();
                } else {
                    self.should_quit = true;
                }
            }
            Action::SelectNext => {
                let count = self.visible_aps().len();
                if count > 0 {
                    self.selected_index = (self.selected_index + 1).min(count - 1);
                    self.pin_selection();
                }
            }
            Action::SelectPrev => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.pin_selection();
                }
            }
            Action::SelectFirst => {
                self.selected_index = 0;
                self.pin_selection();
            }
            Action::SelectLast => {
                let count = self.visible_aps().len();
                if count > 0 {
                    self.selected_index = count - 1;
                }
                self.pin_selection();
            }
            Action::Activate => {
                if let Some(ap) = self.selected_ap() {
                    if ap.active {
                        return None;
                    }
                    if !ap.security.is_connectable() {
                        self.add_toast(
                            "This network type is not supported".to_string(),
                            ToastSeverity::Warning,
                        );
                        return None;
                    }
                    if ap.security.needs_password() && !ap.known {
                        self.popup = Some(Popup::Password {
                            ap_id: ap.id.clone(),
                            ssid_display: ap.ssid_display(),
                            security: ap.security,
                            input: String::new(),
                            revealed: false,
                        });
                    } else {
                        return Some(Command::Connect { ap: ap.id.clone(), psk: None });
                    }
                }
            }
            Action::Scan => {
                return Some(Command::Scan);
            }
            Action::Disconnect => {
                return Some(Command::Disconnect);
            }
            Action::ForgetSelected => {
                if let Some(ap) = self.selected_ap() {
                    if ap.known {
                        self.popup = Some(Popup::Confirm {
                            message: format!(
                                "Forget network \"{}\"? You will need to re-enter the password to reconnect.",
                                ap.ssid_display()
                            ),
                            on_confirm: ConfirmAction::ForgetNetwork(ConnId(ap.ssid_display())),
                        });
                    } else {
                        self.add_toast("Network is not saved".to_string(), ToastSeverity::Info);
                    }
                }
            }
            Action::Confirm => {
                if let Some(Popup::Confirm { on_confirm, .. }) = self.popup.take() {
                    match on_confirm {
                        ConfirmAction::ForgetNetwork(id) => {
                            self.add_toast(
                                format!("Forgot network \"{}\"", id.0),
                                ToastSeverity::Info,
                            );
                            return Some(Command::Forget(id));
                        }
                    }
                }
            }
            Action::Cancel => {
                if let Some(Popup::Password { ref mut input, .. }) = self.popup {
                    zeroize::Zeroize::zeroize(input);
                }
                self.popup = None;
            }
            Action::TogglePasswordReveal => {
                if let Some(Popup::Password { ref mut revealed, .. }) = self.popup {
                    *revealed = !*revealed;
                }
            }
            Action::SubmitPassword => {
                if let Some(Popup::Password { ap_id, mut input, .. }) = self.popup.take() {
                    let psk = secrecy::SecretString::from(std::mem::take(&mut input));
                    zeroize::Zeroize::zeroize(&mut input);
                    return Some(Command::Connect { ap: ap_id, psk: Some(psk) });
                }
            }
            Action::ToggleHelp => {
                if matches!(self.popup, Some(Popup::Help { .. })) {
                    self.popup = None;
                } else {
                    self.popup = Some(Popup::Help { scroll: 0 });
                }
            }
            Action::ToggleSpectrum => {
                self.show_spectrum = !self.show_spectrum;
            }
            Action::StartFilter => {
                self.filter_active = true;
                self.filter.clear();
            }
            Action::ClearFilter => {
                self.filter_active = false;
                self.filter.clear();
            }
            Action::CycleSort => {
                self.sort_mode = self.sort_mode.next();
                self.add_toast(format!("Sort: {}", self.sort_mode.label()), ToastSeverity::Info);
            }
            Action::NextTheme => {
                self.theme_index = (self.theme_index + 1) % 4;
            }
            Action::Resize { width, height } => {
                self.term_width = width;
                self.term_height = height;
            }
            Action::Tick => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_tick).as_secs_f64();
                self.last_tick = now;

                if self.scanning {
                    self.scan_anim_progress = (self.scan_anim_progress + dt * 0.8) % 1.0;
                    self.animating = true;
                } else if self.scan_anim_progress > 0.0 {
                    self.scan_anim_progress = 0.0;
                }

                self.toasts.retain(super::ui::toast::Toast::alive);
                self.animating = self.scanning || !self.toasts.is_empty();
                if !self.animating {
                    self.dirty = false;
                }
            }
            Action::BackendEvent(event) => {
                self.handle_backend_event(event);
            }
            Action::CharInput(c) => match &mut self.popup {
                Some(Popup::Password { input, .. }) => {
                    input.push(c);
                }
                Some(Popup::HiddenSsid { input }) => {
                    input.push(c);
                }
                _ => {
                    if self.filter_active {
                        self.filter.push(c);
                        self.selected_index = 0;
                    }
                }
            },
            Action::Backspace => match &mut self.popup {
                Some(Popup::Password { input, .. }) => {
                    input.pop();
                }
                Some(Popup::HiddenSsid { input }) => {
                    input.pop();
                }
                _ => {
                    if self.filter_active {
                        self.filter.pop();
                        self.selected_index = 0;
                    }
                }
            },
            Action::ToggleWifi => {
                self.add_toast(
                    "WiFi toggle not yet supported by polkit".to_string(),
                    ToastSeverity::Warning,
                );
            }
            Action::ScrollUp => {
                if let Some(Popup::Help { ref mut scroll }) = self.popup {
                    *scroll = scroll.saturating_sub(1);
                }
            }
            Action::ScrollDown => {
                if let Some(Popup::Help { ref mut scroll }) = self.popup {
                    *scroll += 1;
                }
            }
            Action::ConnectHidden => {
                self.popup = Some(Popup::HiddenSsid { input: String::new() });
            }
        }

        None
    }

    /// Handle an event from the NM backend.
    fn handle_backend_event(&mut self, event: NmEvent) {
        match event {
            NmEvent::AccessPoints(aps) => {
                for ap in &aps {
                    if !ap.is_hidden() {
                        let history = self
                            .signal_history
                            .entry(ap.ssid.clone())
                            .or_insert_with(|| VecDeque::with_capacity(60));
                        history.push_back(ap.strength);
                        if history.len() > 60 {
                            history.pop_front();
                        }
                    }
                }

                self.access_points = dedupe_aps(aps);
                self.restore_selection();
            }
            NmEvent::State(state) => {
                let was_connecting = matches!(self.conn_state, ConnState::Connecting { .. });
                self.conn_state = state;

                if was_connecting {
                    if let ConnState::Connected { ref ssid, .. } = self.conn_state {
                        self.add_toast(
                            format!("Connected to {}", String::from_utf8_lossy(ssid)),
                            ToastSeverity::Success,
                        );
                    }
                }
            }
            NmEvent::ScanStarted => {
                self.scanning = true;
                self.animating = true;
            }
            NmEvent::ScanFinished => {
                self.scanning = false;
            }
            NmEvent::ConnectResult(result) => {
                if let Err(e) = result {
                    self.add_toast(e.user_message().to_string(), ToastSeverity::Error);

                    if matches!(e, NmError::WrongPassword) {
                        if let ConnState::Connecting { ref ssid, .. } = self.conn_state {
                            if let Some(ap) = self.access_points.iter().find(|a| a.ssid == *ssid) {
                                self.popup = Some(Popup::Password {
                                    ap_id: ap.id.clone(),
                                    ssid_display: ap.ssid_display(),
                                    security: ap.security,
                                    input: String::new(),
                                    revealed: false,
                                });
                            }
                        }
                    }
                }
            }
            NmEvent::Fatal(error) => {
                self.add_toast(error.user_message().to_string(), ToastSeverity::Error);
            }
        }
    }

    /// Pin the selection to the currently selected SSID.
    fn pin_selection(&mut self) {
        let visible = self.visible_aps();
        self.selected_ssid = visible.get(self.selected_index).map(|ap| ap.ssid.clone());
    }

    /// Restore selection to the pinned SSID after data changes.
    fn restore_selection(&mut self) {
        if let Some(ref ssid) = self.selected_ssid {
            let visible = self.visible_aps();
            if let Some(pos) = visible.iter().position(|ap| &ap.ssid == ssid) {
                self.selected_index = pos;
            } else {
                let count = visible.len();
                if count > 0 {
                    self.selected_index = self.selected_index.min(count - 1);
                } else {
                    self.selected_index = 0;
                }
            }
        }
    }

    /// Add a toast notification.
    pub fn add_toast(&mut self, message: String, severity: ToastSeverity) {
        self.toasts.push_back(Toast::new(message, severity));
        self.animating = true;
        while self.toasts.len() > 5 {
            self.toasts.pop_front();
        }
    }
}

/// Deduplicate access points by SSID, keeping the strongest signal
/// and merging BSSID lists.
pub fn dedupe_aps(aps: Vec<AccessPoint>) -> Vec<AccessPoint> {
    let mut map: HashMap<Vec<u8>, AccessPoint> = HashMap::new();

    for ap in aps {
        if ap.is_hidden() {
            continue;
        }

        match map.get_mut(&ap.ssid) {
            Some(existing) => {
                if ap.strength > existing.strength {
                    existing.strength = ap.strength;
                    existing.freq_mhz = ap.freq_mhz;
                    existing.id = ap.id;
                }
                for bssid in &ap.bssids {
                    if !existing.bssids.contains(bssid) {
                        existing.bssids.push(*bssid);
                    }
                }
                existing.known = existing.known || ap.known;
                existing.active = existing.active || ap.active;
            }
            None => {
                map.insert(ap.ssid.clone(), ap);
            }
        }
    }

    map.into_values().collect()
}
