//! What the client adds to journal lines it uploads to the edcas API.
//!
//! The journal writes a system's position (`StarPos`) only on the events that enter it:
//! `FSDJump`, `Location`, `CarrierJump`. A `Scan` names its system by address alone. The
//! server checks every position against the cell its system address names
//! (`edcas_common::system_address`), so a scan without one is a scan it cannot check.
//! EDDN asks its uploaders to add the position for the same reason; this does it for
//! the direct upload.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

/// Positions of the systems the journal has entered, by system address.
///
/// A map and not just "the current system": the reader also sends older journal files
/// through the same channel (the scans of a system visited before), so lines of several
/// systems interleave. A scan gets the position of its own address or none.
#[derive(Default)]
pub struct SystemPositions {
    seen: HashMap<i64, Value>,
}

impl SystemPositions {
    /// The positions the newest journal file knows, so that a scan made right after the
    /// client starts (the commander is already in the system) gets one too.
    pub fn from_latest_journal(dir: &Path) -> Self {
        let mut positions = Self::default();
        let Some(path) = crate::journal_reader::find_latest_journal_file(dir) else {
            return positions;
        };
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                // Only the three event types matter; skip parsing everything else.
                if !line.contains("\"StarPos\"") {
                    continue;
                }
                if let Ok(message) = serde_json::from_str::<Value>(line.trim()) {
                    positions.note(&message);
                }
            }
        }
        positions
    }

    /// Remembers the position of a system the commander enters.
    fn note(&mut self, message: &Value) {
        if !matches!(event_name(message), "FSDJump" | "Location" | "CarrierJump") {
            return;
        }
        let address = message.get("SystemAddress").and_then(Value::as_i64);
        let pos = message.get("StarPos").filter(|p| is_position(p));
        if let (Some(address), Some(pos)) = (address, pos) {
            self.seen.insert(address, pos.clone());
        }
    }

    /// Prepares one journal line for upload: notes a system's position, and gives a scan
    /// the position of its system when the journal has told us it.
    pub fn apply(&mut self, message: &mut Value) {
        self.note(message);
        if event_name(message) != "Scan" || message.get("StarPos").is_some() {
            return;
        }
        let Some(address) = message.get("SystemAddress").and_then(Value::as_i64) else {
            return;
        };
        if let (Some(pos), Some(fields)) = (self.seen.get(&address), message.as_object_mut()) {
            fields.insert("StarPos".to_string(), pos.clone());
        }
    }
}

fn event_name(message: &Value) -> &str {
    message.get("event").and_then(Value::as_str).unwrap_or("")
}

fn is_position(value: &Value) -> bool {
    value.as_array().is_some_and(|a| a.len() == 3 && a.iter().all(Value::is_number))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn jump(system: &str, address: i64, pos: [f64; 3]) -> Value {
        json!({ "timestamp": "2026-11-20T18:00:00Z", "event": "FSDJump", "StarSystem": system, "SystemAddress": address, "StarPos": pos })
    }

    fn scan(body: &str, address: i64) -> Value {
        json!({ "timestamp": "2026-11-20T18:01:00Z", "event": "Scan", "BodyName": body, "BodyID": 1, "SystemAddress": address })
    }

    #[test]
    fn a_scan_gets_the_position_of_the_system_it_was_made_in() {
        let mut positions = SystemPositions::default();
        positions.apply(&mut jump("Plooe Thoi RB-O c6-7", 1992026034234, [-9506.9375, 87.375, -3322.9375]));
        let mut s = scan("Plooe Thoi RB-O c6-7 A", 1992026034234);
        positions.apply(&mut s);
        assert_eq!(s["StarPos"], json!([-9506.9375, 87.375, -3322.9375]));
    }

    #[test]
    fn the_server_accepts_what_this_sends() {
        let mut positions = SystemPositions::default();
        positions.apply(&mut jump("Sol", 10477373803, [0.0, 0.0, 0.0]));
        let mut s = json!({ "timestamp": "2026-11-20T18:01:00Z", "event": "Scan", "ScanType": "Detailed",
            "BodyName": "Earth", "BodyID": 3, "StarSystem": "Sol", "SystemAddress": 10477373803i64 });
        positions.apply(&mut s);
        let event = edcas_common::journal::JournalEvent::from_eddn_message(s).expect("a scan the server parses");
        assert_eq!(event.validate(), Ok(()));
    }

    /// The reader replays an older journal file between two live lines: the live scan
    /// must not take the old file's system.
    #[test]
    fn interleaved_systems_each_keep_their_own_position() {
        let mut positions = SystemPositions::default();
        positions.apply(&mut jump("Sol", 10477373803, [0.0, 0.0, 0.0]));
        positions.apply(&mut jump("Achenar", 164098653, [67.5, -119.46875, 24.84375]));
        let mut sol = scan("Earth", 10477373803);
        positions.apply(&mut sol);
        let mut achenar = scan("Achenar 1", 164098653);
        positions.apply(&mut achenar);
        assert_eq!(sol["StarPos"], json!([0.0, 0.0, 0.0]));
        assert_eq!(achenar["StarPos"], json!([67.5, -119.46875, 24.84375]));
    }

    #[test]
    fn a_scan_of_a_system_the_journal_gave_no_position_for_goes_out_as_it_is() {
        let mut positions = SystemPositions::default();
        positions.apply(&mut jump("Sol", 10477373803, [0.0, 0.0, 0.0]));
        let mut s = scan("Achenar 1", 164098653);
        let before = s.clone();
        positions.apply(&mut s);
        assert_eq!(s, before);
    }

    #[test]
    fn other_lines_and_a_position_already_there_are_left_alone() {
        let mut positions = SystemPositions::default();
        positions.apply(&mut jump("Sol", 10477373803, [0.0, 0.0, 0.0]));
        let mut docked = json!({ "event": "Docked", "StationName": "Abraham Lincoln", "SystemAddress": 10477373803i64 });
        let before = docked.clone();
        positions.apply(&mut docked);
        assert_eq!(docked, before);
        let mut with_pos = scan("Earth", 10477373803);
        with_pos["StarPos"] = json!([1.0, 2.0, 3.0]);
        positions.apply(&mut with_pos);
        assert_eq!(with_pos["StarPos"], json!([1.0, 2.0, 3.0]));
        // A market file has no "event" at all.
        let mut market = json!({ "MarketID": 128016640, "Items": [] });
        positions.apply(&mut market);
        assert_eq!(market, json!({ "MarketID": 128016640, "Items": [] }));
    }

    #[test]
    fn a_jump_with_a_broken_position_is_not_remembered() {
        let mut positions = SystemPositions::default();
        positions.apply(&mut json!({ "event": "FSDJump", "SystemAddress": 10477373803i64, "StarPos": [0.0, 0.0] }));
        let mut s = scan("Earth", 10477373803);
        positions.apply(&mut s);
        assert!(s.get("StarPos").is_none());
    }
}
