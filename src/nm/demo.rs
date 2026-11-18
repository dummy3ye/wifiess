//! Demo (fake) backend for `--demo` mode and testing.
//!
//! Produces realistic fluctuating access points without requiring
//! a `WiFi` device or `NetworkManager`.

use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time;

use super::model::{AccessPoint, ApId, ConnStage, ConnState, Security};
use super::{Command, NmEvent};

/// Run the demo backend.
pub async fn run(mut cmd_rx: mpsc::Receiver<Command>, event_tx: mpsc::Sender<NmEvent>) {
    tracing::info!("Starting demo backend");

    // Send initial state
    let _ = event_tx.send(NmEvent::State(ConnState::Disconnected)).await;

    // Send initial APs
    let mut tick: u64 = 0;
    let _ = event_tx.send(NmEvent::AccessPoints(generate_aps(tick))).await;

    let mut update_interval = time::interval(Duration::from_secs(2));
    update_interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    let mut connected_ssid: Option<Vec<u8>> = None;

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    Command::Scan => {
                        tracing::debug!("Demo: scan requested");
                        let _ = event_tx.send(NmEvent::ScanStarted).await;
                        time::sleep(Duration::from_millis(1500)).await;
                        tick += 1;
                        let _ = event_tx.send(NmEvent::AccessPoints(generate_aps(tick))).await;
                        let _ = event_tx.send(NmEvent::ScanFinished).await;
                    }
                    Command::Connect { ap, psk } => {
                        tracing::debug!("Demo: connect to {}", ap);
                        let ssid = ap_id_to_ssid(&ap);

                        let _ = event_tx.send(NmEvent::State(ConnState::Connecting {
                            ssid: ssid.clone(),
                            stage: ConnStage::Authenticating,
                        })).await;

                        time::sleep(Duration::from_millis(800)).await;

                        if ap.0.contains("secure") && psk.is_none() {
                            let _ = event_tx.send(NmEvent::ConnectResult(
                                Err(super::NmError::WrongPassword)
                            )).await;
                            let _ = event_tx.send(NmEvent::State(ConnState::Disconnected)).await;
                            continue;
                        }

                        let _ = event_tx.send(NmEvent::State(ConnState::Connecting {
                            ssid: ssid.clone(),
                            stage: ConnStage::ObtainingIp,
                        })).await;

                        time::sleep(Duration::from_millis(600)).await;

                        connected_ssid = Some(ssid.clone());
                        let _ = event_tx.send(NmEvent::State(ConnState::Connected {
                            ssid,
                            ip: Some("192.168.1.42".to_string()),
                        })).await;
                        let _ = event_tx.send(NmEvent::ConnectResult(Ok(()))).await;
                    }
                    Command::ConnectHidden { ssid, .. } => {
                        tracing::debug!("Demo: connect to hidden network");
                        let _ = event_tx.send(NmEvent::State(ConnState::Connecting {
                            ssid: ssid.clone(),
                            stage: ConnStage::Authenticating,
                        })).await;
                        time::sleep(Duration::from_millis(800)).await;
                        let _ = event_tx.send(NmEvent::State(ConnState::Connecting {
                            ssid: ssid.clone(),
                            stage: ConnStage::ObtainingIp,
                        })).await;
                        time::sleep(Duration::from_millis(600)).await;
                        connected_ssid = Some(ssid.clone());
                        let _ = event_tx.send(NmEvent::State(ConnState::Connected {
                            ssid,
                            ip: Some("10.0.0.50".to_string()),
                        })).await;
                        let _ = event_tx.send(NmEvent::ConnectResult(Ok(()))).await;
                    }
                    Command::Disconnect => {
                        tracing::debug!("Demo: disconnect");
                        let _ = event_tx.send(NmEvent::State(ConnState::Disconnecting)).await;
                        time::sleep(Duration::from_millis(300)).await;
                        connected_ssid = None;
                        let _ = event_tx.send(NmEvent::State(ConnState::Disconnected)).await;
                    }
                    Command::Forget(conn_id) => {
                        tracing::debug!("Demo: forget {}", conn_id);
                        // In demo mode, mark the forgotten network as not known
                        let mut aps = generate_aps(tick);
                        for ap in &mut aps {
                            if ap.id.0 == conn_id.0 || ap.ssid_display() == conn_id.0 {
                                ap.known = false;
                            }
                        }
                        let _ = event_tx.send(NmEvent::AccessPoints(aps)).await;
                    }
                }
            }
            _ = update_interval.tick() => {
                tick += 1;
                let mut aps = generate_aps(tick);

                if let Some(ref ssid) = connected_ssid {
                    for ap in &mut aps {
                        if ap.ssid == *ssid {
                            ap.active = true;
                        }
                    }
                }

                let _ = event_tx.send(NmEvent::AccessPoints(aps)).await;
            }
        }
    }
}

fn ap_id_to_ssid(ap_id: &ApId) -> Vec<u8> {
    ap_id.0.strip_prefix("demo/").unwrap_or(&ap_id.0).as_bytes().to_vec()
}

fn generate_aps(tick: u64) -> Vec<AccessPoint> {
    let networks: Vec<(&str, u8, u32, Security, bool)> = vec![
        ("Home-5G", 85, 5180, Security::Wpa3, true),
        ("Home-2.4G", 78, 2437, Security::Wpa2, true),
        ("CoffeeShop-Guest", 65, 2412, Security::Wpa2, false),
        ("Neighbor_WiFi_2G", 42, 2462, Security::Wpa2, false),
        ("FreePublicWiFi", 58, 2422, Security::Open, false),
        ("Office-Corp-5G", 72, 5745, Security::Wpa3, true),
        ("CommunityGuest", 38, 2432, Security::Open, false),
        ("🏠 IoT-SmartDevices", 62, 5200, Security::Wpa2, false),
        ("NETGEAR-Setup-99", 30, 2447, Security::Wpa2, false),
        ("xfinitywifi_mesh", 36, 2417, Security::Open, false),
        ("TP-Link_Archer_AX50", 52, 5240, Security::Wpa2, false),
        ("CorpEnterprise_Secure", 75, 5500, Security::Enterprise, false),
        ("NextGen-6G-Fast", 90, 5955, Security::Wpa3, true),
        ("Airport_Free_Boingo", 48, 2452, Security::Open, false),
    ];

    networks
        .into_iter()
        .enumerate()
        .map(|(i, (ssid, base, freq, security, known))| {
            let phase = (i as u64).wrapping_mul(7);
            let wobble = ((tick.wrapping_add(phase)) % 15) as i16 - 7;
            let strength = (i16::from(base) + wobble).clamp(5, 100) as u8;

            let bssid = [0xAA, 0xBB, 0xCC, i as u8, ((tick % 256) as u8), 0x01];

            AccessPoint {
                id: ApId(format!("demo/{ssid}")),
                ssid: ssid.as_bytes().to_vec(),
                bssids: vec![bssid],
                strength,
                freq_mhz: freq,
                security,
                known,
                active: false,
            }
        })
        .collect()
}
