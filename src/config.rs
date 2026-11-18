//! Configuration loading and management.
//!
//! Loads settings from `~/.config/wifiess/config.toml` (or XDG equivalent).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Selected theme name.
    pub theme: String,
    /// Scan interval in seconds.
    pub scan_interval_secs: u64,
    /// Whether to enable mouse support.
    pub mouse: bool,
    /// Override color depth detection.
    pub color_override: Option<ColorDepthOverride>,
    /// Override Unicode detection.
    pub unicode_override: Option<bool>,
    /// Keybindings (currently using defaults).
    pub keys: KeyConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            scan_interval_secs: 30,
            mouse: true,
            color_override: None,
            unicode_override: None,
            keys: KeyConfig::default(),
        }
    }
}

/// Color depth override options.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorDepthOverride {
    /// True color (24-bit).
    Truecolor,
    /// 256 colors.
    Color256,
    /// 16 colors.
    Color16,
    /// No color.
    NoColor,
}

/// Keybinding configuration (placeholder for customization).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyConfig {
    /// Key to quit.
    pub quit: String,
    /// Key to scan.
    pub scan: String,
    /// Key to disconnect.
    pub disconnect: String,
    /// Key to forget network.
    pub forget: String,
    /// Key to toggle help.
    pub help: String,
    /// Key to toggle spectrum.
    pub spectrum: String,
    /// Key to start filter.
    pub filter: String,
    /// Key to cycle sort.
    pub sort: String,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            quit: "q".to_string(),
            scan: "s".to_string(),
            disconnect: "d".to_string(),
            forget: "f".to_string(),
            help: "?".to_string(),
            spectrum: "S".to_string(),
            filter: "/".to_string(),
            sort: "o".to_string(),
        }
    }
}

/// Return the path to the config file.
pub fn config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "wifiess")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Return the path to the log directory.
pub fn log_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "wifiess").map(|dirs| dirs.data_dir().join("logs"))
}

/// Load configuration from the default path, falling back to defaults.
pub fn load_config() -> AppConfig {
    let Some(path) = config_path() else {
        tracing::warn!("Could not determine config directory, using defaults");
        return AppConfig::default();
    };

    if !path.exists() {
        tracing::info!("No config file at {}, using defaults", path.display());
        return AppConfig::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => {
                tracing::info!("Loaded config from {}", path.display());
                config
            }
            Err(e) => {
                tracing::error!("Failed to parse config at {}: {e}", path.display());
                tracing::warn!("Using default configuration");
                AppConfig::default()
            }
        },
        Err(e) => {
            tracing::error!("Failed to read config at {}: {e}", path.display());
            AppConfig::default()
        }
    }
}
