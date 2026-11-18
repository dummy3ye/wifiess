//! Comprehensive unit tests for all modules and components of wifiess.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use secrecy::SecretString;
use std::time::Duration;

use wifiess::action::Action;
use wifiess::app::{Popup, SortMode, State};
use wifiess::config::{AppConfig, ColorDepthOverride};
use wifiess::nm::error::NmError;
use wifiess::nm::model::{AccessPoint, ApId, Band, ConnId, ConnStage, ConnState, Security};
use wifiess::nm::settings::build_wifi_settings;
use wifiess::nm::{Command, NmEvent};
use wifiess::tui::{ColorDepth, UnicodeSupport};
use wifiess::ui::anim::{SCAN_FRAMES, lerp, scan_indicator};
use wifiess::ui::schart::render_signal_chart;
use wifiess::ui::spectrum::render_spectrum;
use wifiess::ui::theme::Theme;
use wifiess::ui::toast::{Toast, ToastSeverity, ToastStack};

// =========================================================================
// 1. NM MODEL & TYPES TESTS
// =========================================================================

#[test]
fn test_ap_id_and_conn_id_conversions_and_display() {
    let ap_id_str = ApId::from("ap/test/1");
    let ap_id_string = ApId::from(String::from("ap/test/2"));
    assert_eq!(format!("{ap_id_str}"), "ap/test/1");
    assert_eq!(format!("{ap_id_string}"), "ap/test/2");
    assert_eq!(ap_id_str, ApId("ap/test/1".to_string()));

    let conn_id_str = ConnId::from("conn/1");
    let conn_id_string = ConnId::from(String::from("conn/2"));
    assert_eq!(format!("{conn_id_str}"), "conn/1");
    assert_eq!(format!("{conn_id_string}"), "conn/2");
    assert_eq!(conn_id_str, ConnId("conn/1".to_string()));
}

#[test]
fn test_band_display_and_from_freq() {
    assert_eq!(format!("{}", Band::TwoGhz), "2.4G");
    assert_eq!(format!("{}", Band::FiveGhz), "5G");
    assert_eq!(format!("{}", Band::SixGhz), "6G");
    assert_eq!(format!("{}", Band::Unknown), "?");

    assert_eq!(Band::from_freq(2400), Band::TwoGhz);
    assert_eq!(Band::from_freq(2484), Band::TwoGhz);
    assert_eq!(Band::from_freq(5000), Band::FiveGhz);
    assert_eq!(Band::from_freq(5885), Band::FiveGhz);
    assert_eq!(Band::from_freq(5925), Band::SixGhz);
    assert_eq!(Band::from_freq(7125), Band::SixGhz);
    assert_eq!(Band::from_freq(8000), Band::Unknown);
    assert_eq!(Band::from_freq(100), Band::Unknown);
}

#[test]
fn test_channel_from_freq_all_bands() {
    // 2.4 GHz
    assert_eq!(Band::channel_from_freq(2412), Some(1));
    assert_eq!(Band::channel_from_freq(2437), Some(6));
    assert_eq!(Band::channel_from_freq(2462), Some(11));
    assert_eq!(Band::channel_from_freq(2472), Some(13));
    assert_eq!(Band::channel_from_freq(2484), Some(14));

    // 5 GHz
    assert_eq!(Band::channel_from_freq(5180), Some(36));
    assert_eq!(Band::channel_from_freq(5240), Some(48));
    assert_eq!(Band::channel_from_freq(5500), Some(100));
    assert_eq!(Band::channel_from_freq(5745), Some(149));
    assert_eq!(Band::channel_from_freq(5825), Some(165));

    // 6 GHz
    assert_eq!(Band::channel_from_freq(5955), Some(1));
    assert_eq!(Band::channel_from_freq(5975), Some(5));
    assert_eq!(Band::channel_from_freq(6015), Some(13));

    // Non-wifi frequency
    assert_eq!(Band::channel_from_freq(1234), None);
}

#[test]
fn test_security_from_flags() {
    // Open
    assert_eq!(Security::from_flags(0, 0, 0), Security::Open);

    // Enterprise (802.1X = 0x200)
    assert_eq!(Security::from_flags(0, 0x200, 0), Security::Enterprise);
    assert_eq!(Security::from_flags(0, 0, 0x200), Security::Enterprise);

    // WPA3 (SAE = 0x400 in rsn_flags)
    assert_eq!(Security::from_flags(0, 0, 0x400), Security::Wpa3);

    // WPA2 (PSK = 0x100 or non-zero rsn_flags)
    assert_eq!(Security::from_flags(0, 0x100, 0), Security::Wpa2);
    assert_eq!(Security::from_flags(0, 0, 0x100), Security::Wpa2);
    assert_eq!(Security::from_flags(0, 0, 0x08), Security::Wpa2);

    // Unsupported (e.g. WEP flags & 0x1)
    assert_eq!(Security::from_flags(0x1, 0, 0), Security::Unsupported);

    // Display
    assert_eq!(format!("{}", Security::Open), "Open");
    assert_eq!(format!("{}", Security::Wpa2), "WPA2");
    assert_eq!(format!("{}", Security::Wpa3), "WPA3");
    assert_eq!(format!("{}", Security::Enterprise), "Enterprise");
    assert_eq!(format!("{}", Security::Unsupported), "Unsupported");
}

#[test]
fn test_access_point_properties() {
    let ap = AccessPoint {
        id: ApId("ap-1".to_string()),
        ssid: b"TestSSID".to_vec(),
        bssids: vec![[1, 2, 3, 4, 5, 6]],
        strength: 82,
        freq_mhz: 2437,
        security: Security::Wpa2,
        known: true,
        active: false,
    };

    assert_eq!(ap.band(), Band::TwoGhz);
    assert_eq!(ap.channel(), Some(6));
    assert!(!ap.is_hidden());
    assert_eq!(ap.ssid_display(), "TestSSID");

    let hidden_ap = AccessPoint {
        id: ApId("hidden".to_string()),
        ssid: vec![],
        bssids: vec![],
        strength: 40,
        freq_mhz: 5180,
        security: Security::Open,
        known: false,
        active: false,
    };
    assert!(hidden_ap.is_hidden());
}

#[test]
fn test_conn_state_and_stages() {
    assert_eq!(ConnState::default(), ConnState::Disconnected);
    assert_eq!(format!("{}", ConnStage::Authenticating), "Authenticating");
    assert_eq!(format!("{}", ConnStage::ObtainingIp), "Obtaining IP");
    assert_eq!(format!("{}", ConnStage::Connected), "Connected");
}

// =========================================================================
// 2. ERROR MODULE TESTS
// =========================================================================

#[test]
fn test_nm_error_messages() {
    assert_eq!(NmError::WrongPassword.user_message(), "Wrong password. Please try again.");
    assert_eq!(
        NmError::Timeout.user_message(),
        "Connection timed out. The network may be out of range."
    );
    assert_eq!(
        NmError::PermissionDenied.user_message(),
        "Permission denied. You may need to run as root or configure polkit."
    );
    assert_eq!(
        NmError::ScanRateLimited.user_message(),
        "Scan rate-limited. Try again in a few seconds."
    );
    assert_eq!(
        NmError::NoWifiDevice.user_message(),
        "No WiFi device found. Check that your adapter is connected."
    );
    assert_eq!(
        NmError::WifiDisabled.user_message(),
        "WiFi is turned off. Enable it with rfkill or your system settings."
    );
    assert_eq!(
        NmError::NmNotRunning.user_message(),
        "NetworkManager is not running. Start it with: sudo systemctl start NetworkManager"
    );
    assert_eq!(NmError::UnsupportedNetwork.user_message(), "This network type is not supported.");

    let dbus_err = NmError::dbus("org.freedesktop.DBus.Error.Failed: connection reset");
    assert_eq!(dbus_err.user_message(), "org.freedesktop.DBus.Error.Failed: connection reset");

    let other_err = NmError::other("Unknown failure");
    assert_eq!(other_err.user_message(), "Unknown failure");
}

// =========================================================================
// 3. SETTINGS BUILDER TESTS
// =========================================================================

#[test]
fn test_build_wifi_settings_open_network() {
    let settings = build_wifi_settings(b"FreePublicWiFi", Security::Open, None, false);

    assert!(settings.contains_key("connection"));
    assert!(settings.contains_key("802-11-wireless"));
    assert!(settings.contains_key("ipv4"));
    assert!(settings.contains_key("ipv6"));
    // Open network must not contain wireless-security
    assert!(!settings.contains_key("802-11-wireless-security"));

    let conn_sec = settings.get("connection").unwrap();
    assert_eq!(conn_sec.get("type").unwrap().to_string(), "\"802-11-wireless\"");
    assert_eq!(conn_sec.get("id").unwrap().to_string(), "\"FreePublicWiFi\"");

    let wifi_sec = settings.get("802-11-wireless").unwrap();
    assert_eq!(wifi_sec.get("mode").unwrap().to_string(), "\"infrastructure\"");
    assert!(!wifi_sec.contains_key("hidden"));
}

#[test]
fn test_build_wifi_settings_wpa2_with_psk() {
    let psk = SecretString::from("secretpass123");
    let settings = build_wifi_settings(b"HomeWiFi", Security::Wpa2, Some(&psk), false);

    assert!(settings.contains_key("802-11-wireless-security"));
    let sec_sec = settings.get("802-11-wireless-security").unwrap();
    assert_eq!(sec_sec.get("key-mgmt").unwrap().to_string(), "\"wpa-psk\"");
    assert_eq!(sec_sec.get("psk").unwrap().to_string(), "\"secretpass123\"");
}

#[test]
fn test_build_wifi_settings_wpa3_hidden() {
    let psk = SecretString::from("wpa3secretpass");
    let settings = build_wifi_settings(b"StealthSAE", Security::Wpa3, Some(&psk), true);

    let wifi_sec = settings.get("802-11-wireless").unwrap();
    assert_eq!(wifi_sec.get("hidden").unwrap().to_string(), "true");

    let sec_sec = settings.get("802-11-wireless-security").unwrap();
    assert_eq!(sec_sec.get("key-mgmt").unwrap().to_string(), "\"sae\"");
}

// =========================================================================
// 4. CONFIG MODULE TESTS
// =========================================================================

#[test]
fn test_default_config_and_serialization() {
    let default_cfg = AppConfig::default();
    assert_eq!(default_cfg.theme, "dark");
    assert_eq!(default_cfg.scan_interval_secs, 30);
    assert!(default_cfg.mouse);
    assert_eq!(default_cfg.keys.quit, "q");
    assert_eq!(default_cfg.keys.scan, "s");
    assert_eq!(default_cfg.keys.disconnect, "d");
    assert_eq!(default_cfg.keys.help, "?");

    let toml_str = toml::to_string(&default_cfg).expect("should serialize config to TOML");
    let parsed: AppConfig = toml::from_str(&toml_str).expect("should deserialize config from TOML");
    assert_eq!(parsed.theme, "dark");
    assert_eq!(parsed.scan_interval_secs, 30);
}

#[test]
fn test_color_depth_override_options() {
    let cfg_str = r#"
        theme = "nord"
        scan_interval_secs = 10
        color_override = "truecolor"
        unicode_override = false
    "#;
    let parsed: AppConfig = toml::from_str(cfg_str).expect("valid custom toml");
    assert_eq!(parsed.theme, "nord");
    assert_eq!(parsed.scan_interval_secs, 10);
    assert!(matches!(parsed.color_override, Some(ColorDepthOverride::Truecolor)));
    assert_eq!(parsed.unicode_override, Some(false));
}

// =========================================================================
// 5. TUI CAPABILITY DETECTION TESTS
// =========================================================================

#[test]
fn test_color_depth_detect_variants() {
    // Standard invocation shouldn't panic
    let depth = ColorDepth::detect();
    assert!(matches!(
        depth,
        ColorDepth::TrueColor | ColorDepth::Color256 | ColorDepth::Color16 | ColorDepth::NoColor
    ));
}

#[test]
fn test_unicode_support_detect() {
    let uni = UnicodeSupport::detect();
    assert!(matches!(uni, UnicodeSupport::Full | UnicodeSupport::AsciiOnly));
}

// =========================================================================
// 6. ANIMATION & EASING TESTS
// =========================================================================

#[test]
fn test_anim_lerp() {
    assert_eq!(lerp(0.0, 100.0, 0.0), 0.0);
    assert_eq!(lerp(0.0, 100.0, 0.5), 50.0);
    assert_eq!(lerp(0.0, 100.0, 1.0), 100.0);
    // Clamping past boundaries
    assert_eq!(lerp(0.0, 100.0, -0.5), 0.0);
    assert_eq!(lerp(0.0, 100.0, 1.5), 100.0);
}

#[test]
fn test_anim_scan_indicator_frames() {
    for i in 0..SCAN_FRAMES.len() {
        let progress = (i as f64) / 10.0;
        let frame = scan_indicator(progress);
        assert!(SCAN_FRAMES.contains(&frame));
    }
}

// =========================================================================
// 7. TOAST NOTIFICATION TESTS
// =========================================================================

#[test]
fn test_toast_severity_properties() {
    assert_eq!(ToastSeverity::Info.icon(), "ℹ ");
    assert_eq!(ToastSeverity::Success.icon(), "✓ ");
    assert_eq!(ToastSeverity::Warning.icon(), "⚠ ");
    assert_eq!(ToastSeverity::Error.icon(), "✗ ");
}

#[test]
fn test_toast_lifecycle() {
    let toast = Toast::new("Scan complete".to_string(), ToastSeverity::Success);
    assert!(toast.alive());
    assert_eq!(toast.duration, Duration::from_secs(4));

    let expired_toast = Toast::with_duration(
        "Immediate expire".to_string(),
        ToastSeverity::Info,
        Duration::from_millis(0),
    );
    std::thread::sleep(Duration::from_millis(1));
    assert!(!expired_toast.alive());
}

#[test]
fn test_toast_stack_rendering() {
    let toasts = vec![
        Toast::new("Message 1".to_string(), ToastSeverity::Info),
        Toast::new("Message 2".to_string(), ToastSeverity::Error),
    ];
    let stack = ToastStack::new(&toasts);

    let area = Rect::new(0, 0, 50, 10);
    let mut buffer = Buffer::empty(area);
    stack.render(area, &mut buffer);
    // Rendering shouldn't panic and should write characters to buffer
    assert!(buffer.content().iter().any(|c| c.symbol() != " "));
}

// =========================================================================
// 8. THEMES & COLOR SYSTEM TESTS
// =========================================================================

#[test]
fn test_all_theme_definitions() {
    let dark = Theme::dark();
    assert_eq!(dark.name, "Dark (Default)");

    let light = Theme::light();
    assert_eq!(light.name, "Light");

    let pastel = Theme::pastel();
    assert_eq!(pastel.name, "Nordic Pastel");

    let hc = Theme::high_contrast();
    assert_eq!(hc.name, "High Contrast");

    // Signal ramp test
    assert_eq!(dark.signal_color(10), dark.bad);
    assert_eq!(dark.signal_color(40), dark.warn);
    assert_eq!(dark.signal_color(70), dark.accent);
    assert_eq!(dark.signal_color(90), dark.good);
}

// =========================================================================
// 9. SIGNAL CHART & SPECTRUM VIEW TESTS
// =========================================================================

#[test]
fn test_signal_chart_render() {
    let theme = Theme::dark();
    let area = Rect::new(0, 0, 30, 1);
    let mut buf = Buffer::empty(area);

    let samples = vec![10, 35, 50, 75, 95];
    render_signal_chart(&samples, &theme, area, &mut buf);

    let content: String = buf.content().iter().map(ratatui::buffer::Cell::symbol).collect();
    assert!(content.contains(' '));
    assert!(content.contains("▂"));
    assert!(content.contains("▄"));
    assert!(content.contains("▆"));
    assert!(content.contains("█"));
}

#[test]
fn test_spectrum_render_channel_overlap() {
    let mut state = State::new(AppConfig::default());
    let ap1 = AccessPoint {
        id: ApId("1".to_string()),
        ssid: b"Channel1AP".to_vec(),
        bssids: vec![],
        strength: 80,
        freq_mhz: 2412, // Channel 1
        security: Security::Wpa2,
        known: false,
        active: false,
    };
    let ap2 = AccessPoint {
        id: ApId("2".to_string()),
        ssid: b"Channel6AP".to_vec(),
        bssids: vec![],
        strength: 75,
        freq_mhz: 2437, // Channel 6
        security: Security::Wpa2,
        known: false,
        active: false,
    };

    state.update(Action::BackendEvent(NmEvent::AccessPoints(vec![ap1, ap2])));

    let theme = Theme::dark();
    let area = Rect::new(0, 0, 80, 8);
    let mut buf = Buffer::empty(area);

    render_spectrum(&state, &theme, area, &mut buf);
    let rendered_text: String = buf.content().iter().map(ratatui::buffer::Cell::symbol).collect();
    assert!(rendered_text.contains("2.4 GHz Spectrum"));
    assert!(rendered_text.contains("Channel:"));
    assert!(rendered_text.contains("Networks:"));
}

// =========================================================================
// 10. APP STATE REDUCER & ACTIONS TESTS
// =========================================================================

#[test]
fn test_app_state_sort_cycles() {
    let mut state = State::new(AppConfig::default());
    assert_eq!(state.sort_mode, SortMode::Signal);

    state.update(Action::CycleSort);
    assert_eq!(state.sort_mode, SortMode::Name);

    state.update(Action::CycleSort);
    assert_eq!(state.sort_mode, SortMode::KnownFirst);

    state.update(Action::CycleSort);
    assert_eq!(state.sort_mode, SortMode::Signal);
}

#[test]
fn test_app_state_theme_cycling() {
    let mut state = State::new(AppConfig::default());
    assert_eq!(state.theme_index, 0);

    state.update(Action::NextTheme);
    assert_eq!(state.theme_index, 1);

    state.update(Action::NextTheme);
    state.update(Action::NextTheme);
    state.update(Action::NextTheme);
    assert_eq!(state.theme_index, 0);
}

#[test]
fn test_app_state_help_and_spectrum_toggles() {
    let mut state = State::new(AppConfig::default());
    assert!(!state.show_spectrum);
    assert!(state.popup.is_none());

    // Toggle help
    state.update(Action::ToggleHelp);
    assert!(matches!(state.popup, Some(Popup::Help { scroll: 0 })));

    state.update(Action::ScrollDown);
    if let Some(Popup::Help { scroll }) = state.popup {
        assert_eq!(scroll, 1);
    } else {
        panic!("Help popup missing");
    }

    state.update(Action::ScrollUp);
    if let Some(Popup::Help { scroll }) = state.popup {
        assert_eq!(scroll, 0);
    } else {
        panic!("Help popup missing");
    }

    state.update(Action::ToggleHelp);
    assert!(state.popup.is_none());

    // Toggle spectrum
    state.update(Action::ToggleSpectrum);
    assert!(state.show_spectrum);
    state.update(Action::ToggleSpectrum);
    assert!(!state.show_spectrum);
}

#[test]
fn test_render_confirm_popup_modal() {
    let popup = Popup::Confirm {
        message: "Forget network \"MySecretWiFi\"? You will need to re-enter the password."
            .to_string(),
        on_confirm: wifiess::app::ConfirmAction::ForgetNetwork(ConnId("MySecretWiFi".to_string())),
    };
    let theme = Theme::dark();
    let area = Rect::new(0, 0, 70, 15);
    let mut buf = Buffer::empty(area);

    wifiess::ui::confirm::render_confirm_popup(&popup, &theme, area, &mut buf);

    let content: String = buf.content().iter().map(ratatui::buffer::Cell::symbol).collect();
    assert!(content.contains("Forget Network"));
    assert!(content.contains("MySecretWiFi"));
    assert!(content.contains("Confirm"));
    assert!(content.contains("Cancel"));
}

#[test]
fn test_app_state_confirm_forget_network() {
    let mut state = State::new(AppConfig::default());
    let ap = AccessPoint {
        id: ApId("conn_saved".to_string()),
        ssid: b"SavedMesh".to_vec(),
        bssids: vec![],
        strength: 75,
        freq_mhz: 5180,
        security: Security::Wpa2,
        known: true,
        active: false,
    };
    state.update(Action::BackendEvent(NmEvent::AccessPoints(vec![ap])));

    // Request forget
    state.update(Action::ForgetSelected);
    assert!(matches!(state.popup, Some(Popup::Confirm { .. })));

    // Confirm forget
    let cmd = state.update(Action::Confirm);
    assert!(matches!(cmd, Some(Command::Forget(ConnId(id))) if id == "SavedMesh"));
    assert!(state.popup.is_none());
}

#[test]
fn test_app_state_connect_hidden_network() {
    let mut state = State::new(AppConfig::default());

    state.update(Action::ConnectHidden);
    assert!(matches!(state.popup, Some(Popup::HiddenSsid { .. })));

    state.update(Action::CharInput('S'));
    state.update(Action::CharInput('e'));
    state.update(Action::CharInput('c'));
    state.update(Action::Backspace);

    if let Some(Popup::HiddenSsid { input }) = &state.popup {
        assert_eq!(input, "Se");
    } else {
        panic!("HiddenSsid popup missing");
    }

    state.update(Action::Cancel);
    assert!(state.popup.is_none());
}

#[test]
fn test_app_state_scan_and_disconnect_commands() {
    let mut state = State::new(AppConfig::default());

    let scan_cmd = state.update(Action::Scan);
    assert!(matches!(scan_cmd, Some(Command::Scan)));

    let disc_cmd = state.update(Action::Disconnect);
    assert!(matches!(disc_cmd, Some(Command::Disconnect)));
}

#[test]
fn test_app_state_tick_animating_updates() {
    let mut state = State::new(AppConfig::default());
    state.scanning = true;

    state.update(Action::Tick);
    assert!(state.animating);
    assert!(state.scan_anim_progress >= 0.0);

    state.scanning = false;
    state.update(Action::Tick);
    assert_eq!(state.scan_anim_progress, 0.0);
}

#[test]
fn test_app_state_restore_selection_by_ssid() {
    let mut state = State::new(AppConfig::default());
    let ap1 = AccessPoint {
        id: ApId("1".to_string()),
        ssid: b"NetworkA".to_vec(),
        bssids: vec![],
        strength: 50,
        freq_mhz: 2412,
        security: Security::Open,
        known: false,
        active: false,
    };
    let ap2 = AccessPoint {
        id: ApId("2".to_string()),
        ssid: b"NetworkB".to_vec(),
        bssids: vec![],
        strength: 40,
        freq_mhz: 2412,
        security: Security::Open,
        known: false,
        active: false,
    };

    state.update(Action::BackendEvent(NmEvent::AccessPoints(vec![ap1.clone(), ap2.clone()])));
    state.update(Action::SelectNext); // select NetworkB
    assert_eq!(state.selected_ap().unwrap().ssid, b"NetworkB");

    // Signal fluctuation: NetworkB becomes stronger than NetworkA
    let ap2_stronger = AccessPoint { strength: 95, ..ap2 };
    state.update(Action::BackendEvent(NmEvent::AccessPoints(vec![ap1, ap2_stronger])));

    // Selection must follow NetworkB despite index shifting to 0
    assert_eq!(state.selected_ap().unwrap().ssid, b"NetworkB");
    assert_eq!(state.selected_index, 0);
}
