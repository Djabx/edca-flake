//! Records rebuilt from the whole journal folder: a background thread replays the events a
//! [`JournalLog`] reads from every journal, oldest first, then follows the newest files.
//!
//! The live journal reader only sees the current session; this is for what reaches back
//! further, like a carrier's history or exploration data not yet sold.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::{info, warn};

/// How often the newest journals are checked for new lines.
const POLL: Duration = Duration::from_secs(2);

/// A record built by replaying journal events in order.
pub trait JournalLog: Clone + Default + Send + Sync + 'static {
    /// The background thread's name, also used in the start-up log line.
    const NAME: &'static str;
    /// What the lines it reads contain near their start, e.g. `"event":"Carrier`: a cheap
    /// filter before parsing, as the folder holds gigabytes of other events.
    const EVENTS: &'static [&'static [u8]];

    fn apply(&mut self, v: &Value);
    /// Runs after every poll; returns whether that changed anything.
    fn settle(&mut self, _now: DateTime<Utc>) -> bool {
        false
    }
    /// Called once the first pass over every journal is done.
    fn set_loaded(&mut self);
    /// One line for the log once the first pass is done.
    fn summary(&self) -> String;
}

/// Follows the journal folder for one [`JournalLog`] in a background thread.
pub struct JournalFollower<T: JournalLog> {
    shared: Arc<Mutex<Arc<T>>>,
    stop: Arc<AtomicBool>,
    pub dir: PathBuf,
}

impl<T: JournalLog> JournalFollower<T> {
    pub fn start(dir: PathBuf) -> Self {
        let shared = Arc::new(Mutex::new(Arc::new(T::default())));
        let stop = Arc::new(AtomicBool::new(false));
        let (s2, st2, d2) = (Arc::clone(&shared), Arc::clone(&stop), dir.clone());
        let spawned = std::thread::Builder::new().name(T::NAME.into()).spawn(move || follow(&d2, &s2, &st2));
        if let Err(e) = spawned {
            warn!("Could not start the {} journal scan: {e}", T::NAME);
        }
        Self { shared, stop, dir }
    }

    /// The latest snapshot (cheap: an `Arc` clone).
    pub fn snapshot(&self) -> Arc<T> {
        Arc::clone(&self.shared.lock().unwrap())
    }
}

impl<T: JournalLog> Drop for JournalFollower<T> {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Feeds the complete lines in `buf` to `log`; returns the bytes consumed (up to the last
/// newline, so a line the game is still writing waits for the next read) and whether any
/// line was applied.
pub fn feed<T: JournalLog>(log: &mut T, buf: &[u8]) -> (usize, bool) {
    let Some(end) = buf.iter().rposition(|&b| b == b'\n') else { return (0, false) };
    let mut changed = false;
    for line in buf[..end].split(|&b| b == b'\n') {
        // The event name always follows the fixed-width timestamp near the start of the line.
        let head = &line[..line.len().min(80)];
        if T::EVENTS.iter().any(|ev| head.windows(ev.len()).any(|w| w == *ev)) {
            if let Ok(v) = serde_json::from_slice::<Value>(line) {
                log.apply(&v);
                changed = true;
            }
        }
    }
    (end + 1, changed)
}

fn read_from(path: &Path, offset: u64) -> Option<Vec<u8>> {
    let mut f = File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    if len <= offset {
        return None;
    }
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = Vec::with_capacity((len - offset) as usize);
    f.read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn follow<T: JournalLog>(dir: &Path, shared: &Mutex<Arc<T>>, stop: &AtomicBool) {
    let started = std::time::Instant::now();
    let mut log = T::default();
    let mut offsets: HashMap<PathBuf, u64> = HashMap::new();
    let mut first = true;
    while !stop.load(Ordering::Relaxed) {
        // Oldest first, so events replay in order. After the first pass only new files and
        // the two newest (the one the game writes, and the one before at a session change)
        // can have grown.
        let mut files = crate::journal_reader::find_all_journal_files(dir);
        files.reverse();
        let n = files.len();
        let mut changed = false;
        for (i, path) in files.into_iter().enumerate() {
            let known = offsets.get(&path).copied();
            if known.is_some() && i + 2 < n {
                continue;
            }
            let offset = known.unwrap_or(0);
            if let Some(buf) = read_from(&path, offset) {
                let (used, c) = feed(&mut log, &buf);
                changed |= c;
                offsets.insert(path, offset + used as u64);
            } else {
                offsets.entry(path).or_insert(offset);
            }
        }
        changed |= log.settle(Utc::now());
        if first {
            log.set_loaded();
            info!(
                "{} journal scan: {} from {} files in {:.1}s",
                T::NAME,
                log.summary(),
                offsets.len(),
                started.elapsed().as_secs_f32()
            );
            changed = true;
            first = false;
        }
        if changed {
            *shared.lock().unwrap() = Arc::new(log.clone());
        }
        std::thread::sleep(POLL);
    }
}
