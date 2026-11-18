//! Core data model types for the NM backend.
//!
//! These types are plain Rust structs/enums shared between the NM and UI layers.
//! They contain no D-Bus or Ratatui dependencies.

use std::fmt;

/// Unique identifier for an access point (D-Bus object path or synthetic ID).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApId(pub String);

impl fmt::Display for ApId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ApId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ApId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Unique identifier for a saved connection (D-Bus object path or synthetic ID).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnId(pub String);

impl fmt::Display for ConnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ConnId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ConnId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// `WiFi` frequency band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Band {
    /// 2.4 GHz band.
    TwoGhz,
    /// 5 GHz band.
    FiveGhz,
    /// 6 GHz band.
    SixGhz,
    /// Unknown band.
    Unknown,
}

impl Band {
    /// Derive the band from a frequency in MHz.
    pub fn from_freq(freq_mhz: u32) -> Self {
        match freq_mhz {
            2400..=2500 => Self::TwoGhz,
            5000..=5900 => Self::FiveGhz,
            5925..=7125 => Self::SixGhz,
            _ => Self::Unknown,
        }
    }

    /// Return the `WiFi` channel number for a given frequency.
    pub fn channel_from_freq(freq_mhz: u32) -> Option<u32> {
        match freq_mhz {
            // 2.4 GHz: channels 1-14
            2412 => Some(1),
            2417 => Some(2),
            2422 => Some(3),
            2427 => Some(4),
            2432 => Some(5),
            2437 => Some(6),
            2442 => Some(7),
            2447 => Some(8),
            2452 => Some(9),
            2457 => Some(10),
            2462 => Some(11),
            2467 => Some(12),
            2472 => Some(13),
            2484 => Some(14),
            // 5 GHz: common channels
            5180 => Some(36),
            5200 => Some(40),
            5220 => Some(44),
            5240 => Some(48),
            5260 => Some(52),
            5280 => Some(56),
            5300 => Some(60),
            5320 => Some(64),
            5500 => Some(100),
            5520 => Some(104),
            5540 => Some(108),
            5560 => Some(112),
            5580 => Some(116),
            5600 => Some(120),
            5620 => Some(124),
            5640 => Some(128),
            5660 => Some(132),
            5680 => Some(136),
            5700 => Some(140),
            5720 => Some(144),
            5745 => Some(149),
            5765 => Some(153),
            5785 => Some(157),
            5805 => Some(161),
            5825 => Some(165),
            // 6 GHz: selected channels
            5955 => Some(1),
            5975 => Some(5),
            5995 => Some(9),
            6015 => Some(13),
            6035 => Some(17),
            6055 => Some(21),
            6075 => Some(25),
            6095 => Some(29),
            6115 => Some(33),
            // Generic formula fallback
            f if (2412..=2484).contains(&f) => Some((f - 2407) / 5),
            f if (5180..=5825).contains(&f) => Some((f - 5000) / 5),
            f if (5955..=7115).contains(&f) => Some((f - 5950) / 5),
            _ => None,
        }
    }
}

impl fmt::Display for Band {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TwoGhz => write!(f, "2.4G"),
            Self::FiveGhz => write!(f, "5G"),
            Self::SixGhz => write!(f, "6G"),
            Self::Unknown => write!(f, "?"),
        }
    }
}

/// Security classification of an access point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Security {
    /// No security.
    Open,
    /// WPA2-PSK.
    Wpa2,
    /// WPA3-SAE.
    Wpa3,
    /// WPA Enterprise (802.1X) — not supported in v1.
    Enterprise,
    /// Unknown or unsupported security type.
    Unsupported,
}

impl Security {
    /// Derive security classification from NM flags.
    pub fn from_flags(flags: u32, wpa_flags: u32, rsn_flags: u32) -> Self {
        // NM_802_11_AP_FLAGS_PRIVACY = 0x1
        if flags == 0 && wpa_flags == 0 && rsn_flags == 0 {
            return Self::Open;
        }

        // NM_802_11_AP_SEC_KEY_MGMT_802_1X = 0x200
        if (wpa_flags & 0x200 != 0) || (rsn_flags & 0x200 != 0) {
            return Self::Enterprise;
        }

        // NM_802_11_AP_SEC_KEY_MGMT_SAE = 0x400
        if rsn_flags & 0x400 != 0 {
            return Self::Wpa3;
        }

        // NM_802_11_AP_SEC_KEY_MGMT_PSK = 0x100
        if (rsn_flags & 0x100 != 0) || (wpa_flags & 0x100 != 0) || (rsn_flags != 0) {
            return Self::Wpa2;
        }

        if flags & 0x1 != 0 {
            // WEP or other legacy encrypted
            return Self::Unsupported;
        }

        Self::Open
    }

    /// Whether this security type requires a password to connect.
    pub fn needs_password(&self) -> bool {
        matches!(self, Self::Wpa2 | Self::Wpa3)
    }

    /// Whether this security type is supported for connection.
    pub fn is_connectable(&self) -> bool {
        matches!(self, Self::Open | Self::Wpa2 | Self::Wpa3)
    }
}

impl fmt::Display for Security {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "Open"),
            Self::Wpa2 => write!(f, "WPA2"),
            Self::Wpa3 => write!(f, "WPA3"),
            Self::Enterprise => write!(f, "Enterprise"),
            Self::Unsupported => write!(f, "Unsupported"),
        }
    }
}

/// A visible `WiFi` access point (possibly merged from multiple BSSIDs).
#[derive(Debug, Clone)]
pub struct AccessPoint {
    /// Unique identifier (D-Bus path or synthetic).
    pub id: ApId,
    /// SSID as raw bytes — NOT guaranteed UTF-8.
    pub ssid: Vec<u8>,
    /// MAC addresses of BSSIDs advertising this SSID.
    pub bssids: Vec<[u8; 6]>,
    /// Signal strength 0–100 (from NM).
    pub strength: u8,
    /// Primary frequency in MHz.
    pub freq_mhz: u32,
    /// Security classification.
    pub security: Security,
    /// Whether this network has a saved connection profile.
    pub known: bool,
    /// Whether this is the currently connected network.
    pub active: bool,
}

impl AccessPoint {
    /// The SSID as a lossy UTF-8 string, suitable for display.
    pub fn ssid_display(&self) -> String {
        let raw = String::from_utf8_lossy(&self.ssid);
        // Strip control characters that could corrupt the terminal
        raw.chars().map(|c| if c.is_control() { '\u{FFFD}' } else { c }).collect()
    }

    /// The `WiFi` band this AP operates on.
    pub fn band(&self) -> Band {
        Band::from_freq(self.freq_mhz)
    }

    /// The `WiFi` channel number, if known.
    pub fn channel(&self) -> Option<u32> {
        Band::channel_from_freq(self.freq_mhz)
    }

    /// Whether the SSID is empty (hidden network).
    pub fn is_hidden(&self) -> bool {
        self.ssid.is_empty()
    }
}

/// Overall connection state of the `WiFi` device.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnState {
    /// Not connected to any network.
    #[default]
    Disconnected,
    /// Attempting to connect.
    Connecting {
        /// SSID being connected to.
        ssid: Vec<u8>,
        /// Current stage of the connection.
        stage: ConnStage,
    },
    /// Connected to a network.
    Connected {
        /// SSID of the connected network.
        ssid: Vec<u8>,
        /// IP address, if available.
        ip: Option<String>,
    },
    /// Disconnecting from the current network.
    Disconnecting,
}

/// Stage within a connection attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnStage {
    /// Authenticating with the AP.
    Authenticating,
    /// Obtaining an IP address via DHCP.
    ObtainingIp,
    /// Fully connected.
    Connected,
}

impl fmt::Display for ConnStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authenticating => write!(f, "Authenticating"),
            Self::ObtainingIp => write!(f, "Obtaining IP"),
            Self::Connected => write!(f, "Connected"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_from_freq() {
        assert_eq!(Band::from_freq(2412), Band::TwoGhz);
        assert_eq!(Band::from_freq(5180), Band::FiveGhz);
        assert_eq!(Band::from_freq(5955), Band::SixGhz);
        assert_eq!(Band::from_freq(900), Band::Unknown);
    }

    #[test]
    fn channel_from_freq() {
        assert_eq!(Band::channel_from_freq(2412), Some(1));
        assert_eq!(Band::channel_from_freq(2437), Some(6));
        assert_eq!(Band::channel_from_freq(2462), Some(11));
        assert_eq!(Band::channel_from_freq(5180), Some(36));
        assert_eq!(Band::channel_from_freq(5745), Some(149));
    }

    #[test]
    fn security_needs_password() {
        assert!(!Security::Open.needs_password());
        assert!(Security::Wpa2.needs_password());
        assert!(Security::Wpa3.needs_password());
        assert!(!Security::Enterprise.needs_password());
    }

    #[test]
    fn ssid_display_strips_control_chars() {
        let ap = AccessPoint {
            id: ApId("test".to_string()),
            ssid: b"hello\x1b[31mworld".to_vec(),
            bssids: vec![],
            strength: 50,
            freq_mhz: 2412,
            security: Security::Open,
            known: false,
            active: false,
        };
        let display = ap.ssid_display();
        assert!(!display.contains('\x1b'));
        assert!(display.contains("hello"));
        assert!(display.contains("world"));
    }
}
