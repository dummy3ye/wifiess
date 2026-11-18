//! Animation helpers, radar sweep, and transitions.

/// Radar scan spinner frames.
pub const SCAN_FRAMES: &[&str] = &["◜", "◠", "◝", "◞", "◡", "◟"];

/// Get animated scan indicator frame.
pub fn scan_indicator(progress: f64) -> &'static str {
    let index = ((progress * 10.0) as usize) % SCAN_FRAMES.len();
    SCAN_FRAMES[index]
}

/// Simple linear interpolation.
pub fn lerp(start: f64, end: f64, t: f64) -> f64 {
    start + (end - start) * t.clamp(0.0, 1.0)
}
