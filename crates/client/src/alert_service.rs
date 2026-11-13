//! Runs the alerts outside the UI's frames.
//!
//! The UI only runs a frame while its window is shown: on Wayland a hidden or covered window
//! gets none, and while the commander flies, EDCAS is hidden. Alerts evaluated in the frame
//! waited for the window, so a valuable body was spoken 7 s after its scan, other events up
//! to six minutes late (2026-10-28). So the journal reader hands every snapshot to an
//! [`AlertSink`] on its own thread as it reads it, a voice thread speaks what fires and keeps
//! the clock-based alerts, and the UI only picks up the alerts to show as toasts.

use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use tracing::info;

use crate::alerts::{Alert, AlertConfig, AlertEngine};
use crate::journal_reader::JournalData;

/// How often the voice thread checks the clock-based alerts (carrier jump ready).
const TICK: Duration = Duration::from_secs(1);

enum VoiceInput {
    Alerts(Vec<Alert>),
    Voice { on: bool, volume: u8 },
    Say(String),
}

/// What the journal reader feeds: evaluates a snapshot on the caller's thread and passes
/// what fires to the voice thread.
#[derive(Clone)]
pub struct AlertSink {
    engine: Arc<Mutex<AlertEngine>>,
    voice: mpsc::Sender<VoiceInput>,
}

impl AlertSink {
    pub fn ingest(&self, data: &JournalData) {
        let fired = self.engine.lock().unwrap_or_else(PoisonError::into_inner).ingest(data);
        if !fired.is_empty() {
            let _ = self.voice.send(VoiceInput::Alerts(fired));
        }
    }
}

/// The app's end: settings in, alerts for the toasts out.
pub struct AlertService {
    sink: AlertSink,
    toasts: mpsc::Receiver<Alert>,
    /// Voice on and volume as last sent to the voice thread.
    voice: (bool, u8),
}

impl AlertService {
    pub fn start(config: AlertConfig, voice_on: bool, volume: u8) -> Self {
        let engine = Arc::new(Mutex::new(AlertEngine::new(config)));
        let (voice_tx, voice_rx) = mpsc::channel();
        let (toast_tx, toasts) = mpsc::channel();
        let clock = Arc::clone(&engine);
        thread::Builder::new()
            .name("alert-voice".into())
            .spawn(move || voice_thread(voice_rx, toast_tx, clock, voice_on, volume))
            .expect("failed to spawn the alert voice thread");
        Self { sink: AlertSink { engine, voice: voice_tx }, toasts, voice: (voice_on, volume) }
    }

    /// The sink to hand the journal reader.
    pub fn sink(&self) -> AlertSink {
        self.sink.clone()
    }

    /// Applies the current settings; cheap enough to call every frame.
    pub fn configure(&mut self, config: AlertConfig, voice_on: bool, volume: u8) {
        self.sink.engine.lock().unwrap_or_else(PoisonError::into_inner).set_config(config);
        if self.voice != (voice_on, volume) {
            self.voice = (voice_on, volume);
            let _ = self.sink.voice.send(VoiceInput::Voice { on: voice_on, volume });
        }
    }

    /// Speaks a phrase whether voice alerts are on or not (the settings' test button).
    pub fn say(&self, text: String) {
        let _ = self.sink.voice.send(VoiceInput::Say(text));
    }

    /// The alerts fired since the last call, for the toasts.
    pub fn fired(&self) -> Vec<Alert> {
        self.toasts.try_iter().collect()
    }
}

fn voice_thread(
    input: mpsc::Receiver<VoiceInput>,
    toasts: mpsc::Sender<Alert>,
    engine: Arc<Mutex<AlertEngine>>,
    mut on: bool,
    mut volume: u8,
) {
    // Connected on the first phrase, on this thread: nothing talks to the speech engine while
    // voice alerts are off.
    let mut tts: Option<crate::tts::TtsEngine> = None;
    let speak = |tts: &mut Option<crate::tts::TtsEngine>, volume: u8, text: String| {
        let engine = tts.get_or_insert_with(|| crate::tts::TtsEngine::new(volume));
        engine.set_volume(volume);
        engine.speak(text);
    };
    let mut next_tick = Instant::now() + TICK;
    loop {
        let mut alerts = Vec::new();
        match input.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
            Ok(VoiceInput::Alerts(fired)) => alerts = fired,
            Ok(VoiceInput::Voice { on: now_on, volume: now }) => (on, volume) = (now_on, now),
            Ok(VoiceInput::Say(text)) => speak(&mut tts, volume, text),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
        if Instant::now() >= next_tick {
            next_tick = Instant::now() + TICK;
            alerts.extend(engine.lock().unwrap_or_else(PoisonError::into_inner).tick(chrono::Utc::now()));
        }
        if alerts.is_empty() {
            continue;
        }
        for a in alerts {
            info!("Alert [{:?} {:?}] {} — {}", a.severity, a.kind, a.title, a.body);
            if on {
                speak(&mut tts, volume, format!("{}. {}", a.title, a.body));
            }
            if toasts.send(a).is_err() {
                return;
            }
        }
        crate::wake::wake();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scan's alert arrives while nothing runs the UI's frames.
    #[test]
    fn an_alert_fires_from_the_reader_thread_without_a_frame() {
        let service = AlertService::start(AlertConfig::default(), false, 50);
        let sink = service.sink();
        let mut j = JournalData::new();
        j.process_line(r#"{ "timestamp":"2026-10-28T12:24:22Z", "event":"FSDJump", "StarSystem":"Pro Thaa YA-Z c3", "SystemAddress":895265935626, "StarPos":[0.0,0.0,0.0], "Population":0 }"#);
        sink.ingest(&j);
        j.process_line(r#"{ "timestamp":"2026-10-28T12:25:06Z", "event":"Scan", "ScanType":"Detailed", "BodyName":"Pro Thaa YA-Z c3 1", "BodyID":1, "StarSystem":"Pro Thaa YA-Z c3", "SystemAddress":895265935626, "DistanceFromArrivalLS":482.0, "PlanetClass":"Water world", "TerraformState":"Terraformable", "Radius":6000000.0, "MassEM":1.0, "WasDiscovered":false, "WasMapped":false }"#);
        thread::spawn(move || sink.ingest(&j)).join().unwrap();

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut fired = Vec::new();
        while fired.is_empty() && Instant::now() < deadline {
            fired = service.fired();
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].title, "Undiscovered terraformable water world");
    }
}
