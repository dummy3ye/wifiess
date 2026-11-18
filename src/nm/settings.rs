//! `NetworkManager` connection settings builder.
//!
//! Builds `a{sa{sv}}` configuration dictionaries for `AddAndActivateConnection`.

use secrecy::{ExposeSecret, SecretString};
use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Value};

use super::model::Security;

/// Build settings map for creating a new connection in `NetworkManager`.
pub fn build_wifi_settings(
    ssid: &[u8],
    security: Security,
    psk: Option<&SecretString>,
    hidden: bool,
) -> HashMap<String, HashMap<String, OwnedValue>> {
    let mut root = HashMap::new();

    // 1. "connection" section
    let mut connection_sec = HashMap::new();
    let id_str = String::from_utf8_lossy(ssid).to_string();
    connection_sec.insert(
        "id".to_string(),
        OwnedValue::try_from(Value::from(id_str)).expect("valid string value"),
    );
    connection_sec.insert(
        "type".to_string(),
        OwnedValue::try_from(Value::from("802-11-wireless")).expect("valid type value"),
    );
    connection_sec.insert(
        "uuid".to_string(),
        OwnedValue::try_from(Value::from(generate_simple_uuid())).expect("valid uuid value"),
    );
    root.insert("connection".to_string(), connection_sec);

    // 2. "802-11-wireless" section
    let mut wifi_sec = HashMap::new();
    wifi_sec.insert(
        "ssid".to_string(),
        OwnedValue::try_from(Value::from(ssid.to_vec())).expect("valid ssid value"),
    );
    wifi_sec.insert(
        "mode".to_string(),
        OwnedValue::try_from(Value::from("infrastructure")).expect("valid mode value"),
    );
    if hidden {
        wifi_sec.insert(
            "hidden".to_string(),
            OwnedValue::try_from(Value::from(true)).expect("valid hidden value"),
        );
    }

    if security != Security::Open {
        wifi_sec.insert(
            "security".to_string(),
            OwnedValue::try_from(Value::from("802-11-wireless-security")).expect("valid sec value"),
        );
    }
    root.insert("802-11-wireless".to_string(), wifi_sec);

    // 3. "802-11-wireless-security" section (if secured)
    if security != Security::Open {
        let mut sec_sec = HashMap::new();
        let key_mgmt = match security {
            Security::Wpa3 => "sae",
            Security::Wpa2 => "wpa-psk",
            _ => "wpa-psk",
        };
        sec_sec.insert(
            "key-mgmt".to_string(),
            OwnedValue::try_from(Value::from(key_mgmt)).expect("valid key-mgmt"),
        );
        if let Some(pass) = psk {
            let secret_slice: &str = pass.expose_secret();
            sec_sec.insert(
                "psk".to_string(),
                OwnedValue::try_from(Value::from(secret_slice)).expect("valid psk"),
            );
        }
        root.insert("802-11-wireless-security".to_string(), sec_sec);
    }

    // 4. "ipv4" section
    let mut ipv4_sec = HashMap::new();
    ipv4_sec.insert(
        "method".to_string(),
        OwnedValue::try_from(Value::from("auto")).expect("valid ipv4"),
    );
    root.insert("ipv4".to_string(), ipv4_sec);

    // 5. "ipv6" section
    let mut ipv6_sec = HashMap::new();
    ipv6_sec.insert(
        "method".to_string(),
        OwnedValue::try_from(Value::from("auto")).expect("valid ipv6"),
    );
    root.insert("ipv6".to_string(), ipv6_sec);

    root
}

fn generate_simple_uuid() -> String {
    use std::time::SystemTime;
    let now =
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        (now & 0xFFFF_FFFF) as u32,
        ((now >> 32) & 0xFFFF) as u16,
        ((now >> 48) & 0x0FFF) as u16,
        ((now >> 60) & 0x0FFF) as u16,
        (now >> 64) as u64
    )
}
