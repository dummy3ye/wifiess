//! NM backend error types.
//!
//! Each variant maps to a short, user-presentable message.

use std::fmt;

/// Errors from the `NetworkManager` backend.
#[derive(Debug, Clone, thiserror::Error)]
pub enum NmError {
    /// Wrong password provided.
    #[error("Wrong password")]
    WrongPassword,

    /// Connection timed out.
    #[error("Connection timed out")]
    Timeout,

    /// Action denied by polkit.
    #[error("Permission denied by polkit")]
    PermissionDenied,

    /// Scan rate-limited by `NetworkManager`.
    #[error("Scan rate-limited, try again in a few seconds")]
    ScanRateLimited,

    /// No `WiFi` device found.
    #[error("No WiFi device found")]
    NoWifiDevice,

    /// `WiFi` is disabled (rfkill).
    #[error("WiFi is turned off")]
    WifiDisabled,

    /// `NetworkManager` is not running.
    #[error("NetworkManager is not running")]
    NmNotRunning,

    /// The network does not support connection (e.g. Enterprise).
    #[error("This network type is not supported")]
    UnsupportedNetwork,

    /// A D-Bus communication error.
    #[error("{message}")]
    Dbus {
        /// Human-readable description.
        message: String,
    },

    /// A generic backend error.
    #[error("{message}")]
    Other {
        /// Human-readable description.
        message: String,
    },
}

impl NmError {
    /// Create a D-Bus error with a human-friendly message.
    pub fn dbus(msg: impl fmt::Display) -> Self {
        Self::Dbus { message: msg.to_string() }
    }

    /// Create a generic error.
    pub fn other(msg: impl fmt::Display) -> Self {
        Self::Other { message: msg.to_string() }
    }

    /// A short user-facing summary of the error.
    pub fn user_message(&self) -> &str {
        match self {
            Self::WrongPassword => "Wrong password. Please try again.",
            Self::Timeout => "Connection timed out. The network may be out of range.",
            Self::PermissionDenied => {
                "Permission denied. You may need to run as root or configure polkit."
            }
            Self::ScanRateLimited => "Scan rate-limited. Try again in a few seconds.",
            Self::NoWifiDevice => "No WiFi device found. Check that your adapter is connected.",
            Self::WifiDisabled => {
                "WiFi is turned off. Enable it with rfkill or your system settings."
            }
            Self::NmNotRunning => {
                "NetworkManager is not running. Start it with: sudo systemctl start NetworkManager"
            }
            Self::UnsupportedNetwork => "This network type is not supported.",
            Self::Dbus { message } => message.as_str(),
            Self::Other { message } => message.as_str(),
        }
    }
}
