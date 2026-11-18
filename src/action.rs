//! Action types representing UI intents.
//!
//! Actions are produced from events and consumed by [`crate::app::State::update`].

/// An action the user or system wants to perform.
#[derive(Debug, Clone)]
pub enum Action {
    /// Quit the application.
    Quit,
    /// Move selection down in the list.
    SelectNext,
    /// Move selection up in the list.
    SelectPrev,
    /// Jump to the first item.
    SelectFirst,
    /// Jump to the last item.
    SelectLast,
    /// Activate the selected item (connect or show detail).
    Activate,
    /// Start a `WiFi` scan.
    Scan,
    /// Disconnect from the current network.
    Disconnect,
    /// Forget the selected saved network.
    ForgetSelected,
    /// Confirm a pending destructive action.
    Confirm,
    /// Cancel / dismiss the current popup or action.
    Cancel,
    /// Toggle the password visibility.
    TogglePasswordReveal,
    /// Submit the password input.
    SubmitPassword,
    /// Open the help overlay.
    ToggleHelp,
    /// Toggle the spectrum view.
    ToggleSpectrum,
    /// Open the search/filter.
    StartFilter,
    /// Clear the search/filter.
    ClearFilter,
    /// Cycle the sort mode.
    CycleSort,
    /// Switch to the next theme.
    NextTheme,
    /// Resize the terminal.
    Resize { width: u16, height: u16 },
    /// A tick for animations and periodic updates.
    Tick,
    /// An NM backend event arrived.
    BackendEvent(crate::nm::NmEvent),
    /// A character input for text fields (password, filter).
    CharInput(char),
    /// Delete a character in text input.
    Backspace,
    /// Toggle `WiFi` on/off.
    ToggleWifi,
    /// Scroll up in detail/help view.
    ScrollUp,
    /// Scroll down in detail/help view.
    ScrollDown,
    /// Connect to a hidden network.
    ConnectHidden,
}
