//! zbus proxy definitions for `NetworkManager` D-Bus interfaces.

use std::collections::HashMap;
use zbus::proxy;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

/// Proxy for `org.freedesktop.NetworkManager`
#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager",
    interface = "org.freedesktop.NetworkManager"
)]
pub trait NetworkManager {
    /// Get all devices known to `NetworkManager`.
    fn get_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    /// Activate a connection.
    fn activate_connection(
        &self,
        connection: &OwnedObjectPath,
        device: &OwnedObjectPath,
        specific_object: &OwnedObjectPath,
    ) -> zbus::Result<OwnedObjectPath>;

    /// Add a new connection and immediately activate it.
    fn add_and_activate_connection(
        &self,
        connection: HashMap<String, HashMap<String, OwnedValue>>,
        device: &OwnedObjectPath,
        specific_object: &OwnedObjectPath,
    ) -> zbus::Result<(OwnedObjectPath, OwnedObjectPath)>;

    /// Deactivate an active connection.
    fn deactivate_connection(&self, active_connection: &OwnedObjectPath) -> zbus::Result<()>;

    /// Whether wireless is enabled.
    #[zbus(property)]
    fn wireless_enabled(&self) -> zbus::Result<bool>;

    /// Set whether wireless is enabled.
    #[zbus(property)]
    fn set_wireless_enabled(&self, value: bool) -> zbus::Result<()>;

    /// Current overall state of `NetworkManager`.
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
}

/// Proxy for `org.freedesktop.NetworkManager.Device`
#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Device"
)]
pub trait Device {
    /// Device type (e.g. 2 for `WiFi`).
    #[zbus(property)]
    fn device_type(&self) -> zbus::Result<u32>;

    /// Operating interface name (e.g. "wlan0").
    #[zbus(property)]
    fn interface(&self) -> zbus::Result<String>;

    /// Currently active connection on this device.
    #[zbus(property)]
    fn active_connection(&self) -> zbus::Result<OwnedObjectPath>;

    /// Current device state.
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    /// `IP4Config` object path if available.
    #[zbus(property)]
    fn ip4_config(&self) -> zbus::Result<OwnedObjectPath>;
}

/// Proxy for `org.freedesktop.NetworkManager.Device.Wireless`
#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Device.Wireless"
)]
pub trait Wireless {
    /// Request the wireless device to scan.
    fn request_scan(&self, options: HashMap<String, OwnedValue>) -> zbus::Result<()>;

    /// Get all access points visible to this device.
    fn get_all_access_points(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    /// Path to currently active `AccessPoint`.
    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<OwnedObjectPath>;

    /// Signal emitted when an `AccessPoint` is added.
    #[zbus(signal)]
    fn access_point_added(&self, access_point: OwnedObjectPath) -> zbus::Result<()>;

    /// Signal emitted when an `AccessPoint` is removed.
    #[zbus(signal)]
    fn access_point_removed(&self, access_point: OwnedObjectPath) -> zbus::Result<()>;
}

/// Proxy for `org.freedesktop.NetworkManager.AccessPoint`
#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.AccessPoint"
)]
pub trait AccessPoint {
    /// SSID as raw byte array.
    #[zbus(property)]
    fn ssid(&self) -> zbus::Result<Vec<u8>>;

    /// Signal strength (0-100).
    #[zbus(property)]
    fn strength(&self) -> zbus::Result<u8>;

    /// Frequency in MHz.
    #[zbus(property)]
    fn frequency(&self) -> zbus::Result<u32>;

    /// Hardware BSSID address string (e.g. "00:11:22:33:44:55").
    #[zbus(property)]
    fn hw_address(&self) -> zbus::Result<String>;

    /// General AP flags.
    #[zbus(property)]
    fn flags(&self) -> zbus::Result<u32>;

    /// WPA flags.
    #[zbus(property)]
    fn wpa_flags(&self) -> zbus::Result<u32>;

    /// RSN / WPA2 / WPA3 flags.
    #[zbus(property)]
    fn rsn_flags(&self) -> zbus::Result<u32>;
}

/// Proxy for `org.freedesktop.NetworkManager.Settings`
#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings",
    interface = "org.freedesktop.NetworkManager.Settings"
)]
pub trait Settings {
    /// List all saved connection paths.
    fn list_connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    /// Add a new connection without activating.
    fn add_connection(
        &self,
        connection: HashMap<String, HashMap<String, OwnedValue>>,
    ) -> zbus::Result<OwnedObjectPath>;
}

/// Proxy for `org.freedesktop.NetworkManager.Settings.Connection`
#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Settings.Connection"
)]
pub trait SettingsConnection {
    /// Get the connection settings dictionary.
    fn get_settings(&self) -> zbus::Result<HashMap<String, HashMap<String, OwnedValue>>>;

    /// Delete this saved connection.
    fn delete(&self) -> zbus::Result<()>;
}

/// Proxy for `org.freedesktop.NetworkManager.Connection.Active`
#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.Connection.Active"
)]
pub trait ActiveConnection {
    /// Connection identifier string.
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    /// Current connection state (2 = connected, 1 = connecting, etc.).
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;

    /// Specific object (usually access point path).
    #[zbus(property)]
    fn specific_object(&self) -> zbus::Result<OwnedObjectPath>;

    /// Path to `IP4Config`.
    #[zbus(property)]
    fn ip4_config(&self) -> zbus::Result<OwnedObjectPath>;
}

/// Proxy for `org.freedesktop.NetworkManager.IP4Config`
#[proxy(
    default_service = "org.freedesktop.NetworkManager",
    interface = "org.freedesktop.NetworkManager.IP4Config"
)]
pub trait IP4Config {
    /// Address data list.
    #[zbus(property)]
    fn address_data(&self) -> zbus::Result<Vec<HashMap<String, OwnedValue>>>;
}
