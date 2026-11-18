//! Snapshot tests for UI layouts and themes using Ratatui `TestBackend`.

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use wifiess::app::{Action, State};
use wifiess::config::AppConfig;
use wifiess::nm::NmEvent;
use wifiess::nm::model::{AccessPoint, ApId, Security};
use wifiess::ui::view;

fn sample_state() -> State {
    let mut state = State::new(AppConfig::default());
    let ap1 = AccessPoint {
        id: ApId("demo/Home-5G".to_string()),
        ssid: b"Home-5G".to_vec(),
        bssids: vec![[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]],
        strength: 84,
        freq_mhz: 5180,
        security: Security::Wpa3,
        known: true,
        active: true,
    };
    let ap2 = AccessPoint {
        id: ApId("demo/CoffeeShop".to_string()),
        ssid: b"CoffeeShop-Free".to_vec(),
        bssids: vec![[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]],
        strength: 65,
        freq_mhz: 2412,
        security: Security::Open,
        known: false,
        active: false,
    };
    let ap3 = AccessPoint {
        id: ApId("demo/Neighbor".to_string()),
        ssid: b"Neighbor_2.4G".to_vec(),
        bssids: vec![[0x12, 0x34, 0x56, 0x78, 0x90, 0xAB]],
        strength: 38,
        freq_mhz: 2437,
        security: Security::Wpa2,
        known: false,
        active: false,
    };

    state.update(Action::BackendEvent(NmEvent::AccessPoints(vec![ap1, ap2, ap3])));
    state
}

fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let mut s = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            s.push_str(buffer[(x, y)].symbol());
        }
        s.push('\n');
    }
    s
}

#[test]
fn test_render_wide_layout() {
    let state = sample_state();
    let backend = TestBackend::new(120, 35);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            view(&state, f.area(), f.buffer_mut());
        })
        .unwrap();

    let text = buffer_to_string(terminal.backend().buffer());
    assert!(text.contains("Networks"));
    assert!(text.contains("Network Details"));
}

#[test]
fn test_render_medium_layout() {
    let state = sample_state();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            view(&state, f.area(), f.buffer_mut());
        })
        .unwrap();

    let text = buffer_to_string(terminal.backend().buffer());
    assert!(text.contains("Networks"));
}

#[test]
fn test_render_narrow_layout() {
    let state = sample_state();
    let backend = TestBackend::new(50, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            view(&state, f.area(), f.buffer_mut());
        })
        .unwrap();

    let text = buffer_to_string(terminal.backend().buffer());
    assert!(text.contains("Networks"));
}

#[test]
fn test_render_too_small_layout() {
    let state = sample_state();
    let backend = TestBackend::new(30, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            view(&state, f.area(), f.buffer_mut());
        })
        .unwrap();

    let text = buffer_to_string(terminal.backend().buffer());
    assert!(text.contains("Terminal too small"));
}

#[test]
fn test_render_spectrum_toggle() {
    let mut state = sample_state();
    state.update(Action::ToggleSpectrum);
    assert!(state.show_spectrum);

    let backend = TestBackend::new(120, 35);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            view(&state, f.area(), f.buffer_mut());
        })
        .unwrap();

    let text = buffer_to_string(terminal.backend().buffer());
    assert!(text.contains("2.4 GHz Spectrum"));
    assert!(text.contains("Channel:"));
}

#[test]
fn test_render_help_overlay() {
    let mut state = sample_state();
    state.update(Action::ToggleHelp);

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            view(&state, f.area(), f.buffer_mut());
        })
        .unwrap();

    let text = buffer_to_string(terminal.backend().buffer());
    assert!(text.contains("Keybindings & Navigation"));
    assert!(text.contains("Select next network"));
}
