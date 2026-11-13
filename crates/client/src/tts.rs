//! Optional spoken alerts via the platform's built-in text-to-speech.
//!
//! Uses the [`tts`] crate, which drives the OS-native speech engine — SAPI on Windows,
//! Speech Dispatcher on Linux, AVFoundation on macOS. That means no bundled binaries and
//! nothing architecture-specific to ship: if the platform has a speech engine, alerts can
//! be spoken; if not, [`TtsEngine`] is a silent no-op (logged once at startup).
//!
//! The engine lives on the alert service's voice thread (`alert_service`) and is only
//! touched from there. `speak` hands the phrase to the OS engine, which renders it
//! asynchronously, so that thread never blocks. An identical phrase repeated within a short window is suppressed so a flapping
//! alert edge can't stutter.

use std::time::{Duration, Instant};

use tracing::{info, warn};
use ::tts::Tts;

/// Suppress an identical phrase repeated within this window.
const DEDUP_WINDOW: Duration = Duration::from_secs(5);

/// Speaks alert phrases through the OS text-to-speech engine.
pub struct TtsEngine {
    /// `None` when no speech engine is available; all calls then no-op.
    inner: Option<Tts>,
    /// Last phrase spoken and when, for short-window de-duplication.
    last: Option<(String, Instant)>,
}

impl TtsEngine {
    /// Connects to the OS speech engine and sets the initial volume (0–100).
    /// Returns a no-op engine if no speech engine is available.
    pub fn new(volume_pct: u8) -> Self {
        match Tts::default() {
            Ok(mut tts) => {
                apply_volume(&mut tts, volume_pct);
                info!("Voice alerts enabled (system text-to-speech)");
                Self { inner: Some(tts), last: None }
            }
            Err(e) => {
                info!("Voice alerts unavailable: no system text-to-speech engine ({e})");
                Self { inner: None, last: None }
            }
        }
    }

    /// Update the playback volume (0–100); takes effect on the next spoken phrase.
    pub fn set_volume(&mut self, volume_pct: u8) {
        if let Some(tts) = self.inner.as_mut() {
            apply_volume(tts, volume_pct);
        }
    }

    /// Speak a phrase. Cheap and non-blocking; dropped if voice is unavailable, and
    /// suppressed if identical to the previous phrase within [`DEDUP_WINDOW`].
    pub fn speak(&mut self, text: impl Into<String>) {
        let Some(tts) = self.inner.as_mut() else {
            return;
        };
        let text = text.into();
        if let Some((last, at)) = &self.last {
            if *last == text && at.elapsed() < DEDUP_WINDOW {
                return;
            }
        }
        // interrupt=false so several alerts firing together queue rather than cut each other off.
        if let Err(e) = tts.speak(&text, false) {
            warn!("Voice synthesis failed: {e}");
        }
        self.last = Some((text, Instant::now()));
    }
}

impl Default for TtsEngine {
    fn default() -> Self {
        Self::new(50)
    }
}

/// Map a 0–100 percentage onto the engine's volume range and apply it. No-op on backends
/// that don't support volume control.
fn apply_volume(tts: &mut Tts, volume_pct: u8) {
    if !tts.supported_features().volume {
        return;
    }
    let min = tts.min_volume();
    let max = tts.max_volume();
    let vol = min + (max - min) * (volume_pct.min(100) as f32 / 100.0);
    if let Err(e) = tts.set_volume(vol) {
        warn!("Could not set voice volume: {e}");
    }
}
