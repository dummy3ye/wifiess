//! Real `NetworkManager` service task.
//!
//! Owns the `zbus::Connection` and translates [`Command`]s to D-Bus calls,
//! emitting [`NmEvent`]s back to the UI.

use std::collections::HashMap;
use std::time::Duration;

use secrecy::SecretString;
use tokio::sync::mpsc;
use tokio::time;
use zbus::zvariant::OwnedObjectPath;

use super::model::{AccessPoint, ApId, ConnStage, ConnState, Security};
use super::proxies::{
    AccessPointProxy, ActiveConnectionProxy, DeviceProxy, NetworkManagerProxy,
    SettingsConnectionProxy, SettingsProxy, WirelessProxy,
};
use super::settings::build_wifi_settings;
use super::{Command, NmError, NmEvent};

/// Run the real NM backend service loop.
pub async fn run(mut cmd_rx: mpsc::Receiver<Command>, event_tx: mpsc::Sender<NmEvent>) {
    tracing::info!("Starting NetworkManager backend service");

    let conn = match zbus::Connection::system().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to connect to system D-Bus: {e}");
            let _ = event_tx.send(NmEvent::Fatal(NmError::NmNotRunning)).await;
            return;
        }
    };

    let nm = match NetworkManagerProxy::new(&conn).await {
        Ok(nm) => nm,
        Err(e) => {
            tracing::error!("Failed to initialize NetworkManagerProxy: {e}");
            let _ = event_tx.send(NmEvent::Fatal(NmError::NmNotRunning)).await;
            return;
        }
    };

    // Find the primary WiFi device
    let wifi_device_path = match find_wifi_device(&nm, &conn).await {
        Ok(Some(dev)) => dev,
        Ok(None) => {
            let _ = event_tx.send(NmEvent::Fatal(NmError::NoWifiDevice)).await;
            return;
        }
        Err(e) => {
            let _ = event_tx.send(NmEvent::Fatal(e)).await;
            return;
        }
    };

    let wireless = match WirelessProxy::builder(&conn).path(&wifi_device_path) {
        Ok(builder) => match builder.build().await {
            Ok(w) => w,
            Err(e) => {
                let _ = event_tx.send(NmEvent::Fatal(NmError::dbus(e))).await;
                return;
            }
        },
        Err(e) => {
            let _ = event_tx.send(NmEvent::Fatal(NmError::dbus(e))).await;
            return;
        }
    };

    let dev_proxy = match DeviceProxy::builder(&conn).path(&wifi_device_path) {
        Ok(builder) => match builder.build().await {
            Ok(d) => d,
            Err(e) => {
                let _ = event_tx.send(NmEvent::Fatal(NmError::dbus(e))).await;
                return;
            }
        },
        Err(e) => {
            let _ = event_tx.send(NmEvent::Fatal(NmError::dbus(e))).await;
            return;
        }
    };

    let settings = match SettingsProxy::new(&conn).await {
        Ok(s) => s,
        Err(e) => {
            let _ = event_tx.send(NmEvent::Fatal(NmError::dbus(e))).await;
            return;
        }
    };

    // Send initial snapshot
    if let Ok(aps) = snapshot_access_points(&conn, &wireless, &settings, &dev_proxy).await {
        let _ = event_tx.send(NmEvent::AccessPoints(aps)).await;
    }

    // Send initial connection state
    if let Ok(state) = query_connection_state(&conn, &dev_proxy).await {
        let _ = event_tx.send(NmEvent::State(state)).await;
    }

    let mut poll_interval = time::interval(Duration::from_secs(3));
    poll_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    Command::Scan => {
                        let _ = event_tx.send(NmEvent::ScanStarted).await;
                        let opts = HashMap::new();
                        match wireless.request_scan(opts).await {
                            Ok(()) => {
                                time::sleep(Duration::from_millis(1500)).await;
                                if let Ok(aps) = snapshot_access_points(&conn, &wireless, &settings, &dev_proxy).await {
                                    let _ = event_tx.send(NmEvent::AccessPoints(aps)).await;
                                }
                                let _ = event_tx.send(NmEvent::ScanFinished).await;
                            }
                            Err(e) => {
                                tracing::warn!("RequestScan returned: {e}");
                                let _ = event_tx.send(NmEvent::ScanFinished).await;
                            }
                        }
                    }
                    Command::Connect { ap, psk } => {
                        handle_connect(
                            &conn,
                            &nm,
                            &settings,
                            &wifi_device_path,
                            &ap,
                            psk.as_ref(),
                            &event_tx,
                        ).await;
                    }
                    Command::ConnectHidden { ssid, psk } => {
                        handle_connect_hidden(
                            &nm,
                            &wifi_device_path,
                            &ssid,
                            psk.as_ref(),
                            &event_tx,
                        ).await;
                    }
                    Command::Disconnect => {
                        if let Ok(active_conn_path) = dev_proxy.active_connection().await {
                            if active_conn_path.as_str() != "/" {
                                let _ = nm.deactivate_connection(&active_conn_path).await;
                            }
                        }
                        let _ = event_tx.send(NmEvent::State(ConnState::Disconnected)).await;
                    }
                    Command::Forget(conn_id) => {
                        let mut deleted = false;
                        // 1. Try treating conn_id as a Settings.Connection path directly
                        if let Ok(path) = OwnedObjectPath::try_from(conn_id.0.as_str()) {
                            if let Ok(builder) = SettingsConnectionProxy::builder(&conn).path(path) {
                                if let Ok(conn_proxy) = builder.build().await {
                                    if conn_proxy.delete().await.is_ok() {
                                        deleted = true;
                                    }
                                }
                            }
                        }

                        // 2. If conn_id was an AP path or SSID instead of Settings connection path,
                        // find the saved connection with matching SSID and delete it.
                        if !deleted {
                            if let Ok(conns) = settings.list_connections().await {
                                for cpath in conns {
                                    if let Ok(builder) = SettingsConnectionProxy::builder(&conn).path(&cpath) {
                                        if let Ok(cproxy) = builder.build().await {
                                            if let Ok(dict) = cproxy.get_settings().await {
                                                if let Some(wifi) = dict.get("802-11-wireless") {
                                                    if let Some(sval) = wifi.get("ssid") {
                                                        if let Ok(bytes) = <Vec<u8>>::try_from(sval.clone()) {
                                                            let ssid_str = String::from_utf8_lossy(&bytes);
                                                            if conn_id.0.contains(ssid_str.as_ref()) {
                                                                let _ = cproxy.delete().await;
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Immediately trigger an AP refresh so the 'known' badge updates
                        if let Ok(aps) = snapshot_access_points(&conn, &wireless, &settings, &dev_proxy).await {
                            let _ = event_tx.send(NmEvent::AccessPoints(aps)).await;
                        }
                    }
                }
            }
            _ = poll_interval.tick() => {
                if let Ok(aps) = snapshot_access_points(&conn, &wireless, &settings, &dev_proxy).await {
                    let _ = event_tx.send(NmEvent::AccessPoints(aps)).await;
                }
                if let Ok(state) = query_connection_state(&conn, &dev_proxy).await {
                    let _ = event_tx.send(NmEvent::State(state)).await;
                }
            }
        }
    }
}

async fn find_wifi_device(
    nm: &NetworkManagerProxy<'_>,
    conn: &zbus::Connection,
) -> Result<Option<OwnedObjectPath>, NmError> {
    let devices = nm.get_devices().await.map_err(NmError::dbus)?;

    for dev_path in devices {
        if let Ok(builder) = DeviceProxy::builder(conn).path(&dev_path) {
            if let Ok(dev) = builder.build().await {
                // NM_DEVICE_TYPE_WIFI = 2
                if let Ok(dtype) = dev.device_type().await {
                    if dtype == 2 {
                        return Ok(Some(dev_path));
                    }
                }
            }
        }
    }
    Ok(None)
}

async fn query_connection_state(
    conn: &zbus::Connection,
    dev: &DeviceProxy<'_>,
) -> Result<ConnState, NmError> {
    let active_path = dev.active_connection().await.map_err(NmError::dbus)?;
    if active_path.as_str() == "/" || active_path.as_str().is_empty() {
        return Ok(ConnState::Disconnected);
    }

    let active_builder =
        ActiveConnectionProxy::builder(conn).path(active_path).map_err(NmError::dbus)?;
    let active_proxy = active_builder.build().await.map_err(NmError::dbus)?;

    let id = active_proxy.id().await.unwrap_or_default();
    let state = active_proxy.state().await.unwrap_or(0);

    // NM_ACTIVE_CONNECTION_STATE_ACTIVATED = 2
    if state == 2 {
        let mut ip_str = None;
        if let Ok(ip_path) = active_proxy.ip4_config().await {
            if ip_path.as_str() != "/" {
                if let Ok(builder) = super::proxies::IP4ConfigProxy::builder(conn).path(ip_path) {
                    if let Ok(ip_proxy) = builder.build().await {
                        if let Ok(data) = ip_proxy.address_data().await {
                            if let Some(first) = data.first() {
                                if let Some(val) = first.get("address") {
                                    if let Ok(s) = <&str>::try_from(val) {
                                        ip_str = Some(s.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ConnState::Connected { ssid: id.into_bytes(), ip: ip_str })
    } else if state == 1 {
        // NM_ACTIVE_CONNECTION_STATE_ACTIVATING = 1
        Ok(ConnState::Connecting { ssid: id.into_bytes(), stage: ConnStage::Authenticating })
    } else {
        Ok(ConnState::Disconnected)
    }
}

async fn snapshot_access_points(
    conn: &zbus::Connection,
    wireless: &WirelessProxy<'_>,
    settings: &SettingsProxy<'_>,
    _dev: &DeviceProxy<'_>,
) -> Result<Vec<AccessPoint>, NmError> {
    let ap_paths = wireless.get_all_access_points().await.map_err(NmError::dbus)?;

    let saved_ssids = load_saved_ssids(conn, settings).await;
    let active_ap_path =
        wireless.active_access_point().await.map(|p| p.as_str().to_string()).unwrap_or_default();

    let mut result = Vec::new();

    for ap_path in ap_paths {
        if let Ok(builder) = AccessPointProxy::builder(conn).path(&ap_path) {
            if let Ok(ap) = builder.build().await {
                let ssid = ap.ssid().await.unwrap_or_default();
                let strength = ap.strength().await.unwrap_or(0);
                let freq = ap.frequency().await.unwrap_or(0);
                let flags = ap.flags().await.unwrap_or(0);
                let wpa_flags = ap.wpa_flags().await.unwrap_or(0);
                let rsn_flags = ap.rsn_flags().await.unwrap_or(0);
                let hw_address = ap.hw_address().await.unwrap_or_default();

                let security = Security::from_flags(flags, wpa_flags, rsn_flags);
                let known = saved_ssids.contains(&ssid);
                let active = ap_path.as_str() == active_ap_path && !active_ap_path.is_empty();

                let bssid_bytes = parse_mac_address(&hw_address);

                result.push(AccessPoint {
                    id: ApId(ap_path.as_str().to_string()),
                    ssid,
                    bssids: vec![bssid_bytes],
                    strength,
                    freq_mhz: freq,
                    security,
                    known,
                    active,
                });
            }
        }
    }

    Ok(result)
}

async fn load_saved_ssids(conn: &zbus::Connection, settings: &SettingsProxy<'_>) -> Vec<Vec<u8>> {
    let mut ssids = Vec::new();
    if let Ok(conns) = settings.list_connections().await {
        for cpath in conns {
            if let Ok(builder) = SettingsConnectionProxy::builder(conn).path(cpath) {
                if let Ok(cproxy) = builder.build().await {
                    if let Ok(dict) = cproxy.get_settings().await {
                        if let Some(wifi) = dict.get("802-11-wireless") {
                            if let Some(sval) = wifi.get("ssid") {
                                if let Ok(bytes) = <Vec<u8>>::try_from(sval.clone()) {
                                    ssids.push(bytes);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ssids
}

async fn handle_connect(
    conn: &zbus::Connection,
    nm: &NetworkManagerProxy<'_>,
    settings: &SettingsProxy<'_>,
    device_path: &OwnedObjectPath,
    ap_id: &ApId,
    psk: Option<&SecretString>,
    event_tx: &mpsc::Sender<NmEvent>,
) {
    let Ok(target_ap_path) = OwnedObjectPath::try_from(ap_id.0.as_str()) else {
        let _ = event_tx
            .send(NmEvent::ConnectResult(Err(NmError::other("Invalid access point path"))))
            .await;
        return;
    };

    let Ok(builder) = AccessPointProxy::builder(conn).path(&target_ap_path) else {
        let _ = event_tx
            .send(NmEvent::ConnectResult(Err(NmError::other("Access point no longer visible"))))
            .await;
        return;
    };

    let Ok(ap_proxy) = builder.build().await else {
        let _ = event_tx
            .send(NmEvent::ConnectResult(Err(NmError::other("Access point no longer visible"))))
            .await;
        return;
    };

    let ssid = ap_proxy.ssid().await.unwrap_or_default();
    let flags = ap_proxy.flags().await.unwrap_or(0);
    let wpa = ap_proxy.wpa_flags().await.unwrap_or(0);
    let rsn = ap_proxy.rsn_flags().await.unwrap_or(0);
    let sec = Security::from_flags(flags, wpa, rsn);

    // Check if already known
    let mut existing_conn_path = None;
    if let Ok(conn_paths) = settings.list_connections().await {
        for cp in conn_paths {
            if let Ok(b) = SettingsConnectionProxy::builder(conn).path(&cp) {
                if let Ok(cproxy) = b.build().await {
                    if let Ok(dict) = cproxy.get_settings().await {
                        if let Some(wifi) = dict.get("802-11-wireless") {
                            if let Some(sval) = wifi.get("ssid") {
                                if let Ok(bytes) = <Vec<u8>>::try_from(sval.clone()) {
                                    if bytes == ssid {
                                        existing_conn_path = Some(cp);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(saved_path) = existing_conn_path {
        // Activate existing saved profile
        match nm.activate_connection(&saved_path, device_path, &target_ap_path).await {
            Ok(_) => {
                let _ = event_tx.send(NmEvent::ConnectResult(Ok(()))).await;
            }
            Err(e) => {
                let _ = event_tx.send(NmEvent::ConnectResult(Err(translate_dbus_error(&e)))).await;
            }
        }
    } else {
        // Create new connection settings and activate
        let settings_dict = build_wifi_settings(&ssid, sec, psk, false);
        match nm.add_and_activate_connection(settings_dict, device_path, &target_ap_path).await {
            Ok(_) => {
                let _ = event_tx.send(NmEvent::ConnectResult(Ok(()))).await;
            }
            Err(e) => {
                let _ = event_tx.send(NmEvent::ConnectResult(Err(translate_dbus_error(&e)))).await;
            }
        }
    }
}

async fn handle_connect_hidden(
    nm: &NetworkManagerProxy<'_>,
    device_path: &OwnedObjectPath,
    ssid: &[u8],
    psk: Option<&SecretString>,
    event_tx: &mpsc::Sender<NmEvent>,
) {
    let specific_object = OwnedObjectPath::try_from("/").unwrap_or_default();
    let sec = if psk.is_some() { Security::Wpa2 } else { Security::Open };
    let settings_dict = build_wifi_settings(ssid, sec, psk, true);

    match nm.add_and_activate_connection(settings_dict, device_path, &specific_object).await {
        Ok(_) => {
            let _ = event_tx.send(NmEvent::ConnectResult(Ok(()))).await;
        }
        Err(e) => {
            let _ = event_tx.send(NmEvent::ConnectResult(Err(translate_dbus_error(&e)))).await;
        }
    }
}

fn translate_dbus_error(e: &zbus::Error) -> NmError {
    let err_str = e.to_string();
    if err_str.contains("polkit") || err_str.contains("NotAuthorized") {
        NmError::PermissionDenied
    } else if err_str.contains("Secret") || err_str.contains("password") {
        NmError::WrongPassword
    } else if err_str.contains("Timeout") {
        NmError::Timeout
    } else {
        NmError::dbus(err_str)
    }
}

fn parse_mac_address(s: &str) -> [u8; 6] {
    let mut mac = [0u8; 6];
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 6 {
        for (i, p) in parts.iter().enumerate() {
            if let Ok(b) = u8::from_str_radix(p, 16) {
                mac[i] = b;
            }
        }
    }
    mac
}
