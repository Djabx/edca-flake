//! In-app alerting.
//!
//! The [`AlertEngine`] is fed every journal snapshot the watcher emits (the journal reader
//! hands it each one on its own thread, `alert_service`, so no state transition is missed
//! and none waits for the UI) plus a periodic time tick. It turns
//! state changes into [`Alert`]s, which the app surfaces as transient on-screen toasts
//! and optional spoken alerts. No OS/desktop notifications, no extra dependencies.
//!
//! Three trigger kinds are represented:
//! - **edge**: fire on a rising transition, re-arm when it clears (low fuel, interdiction)
//! - **event**: a discrete thing appears in a snapshot (an undiscovered valuable body)
//! - **timer**: fires at a future instant with no triggering event (carrier jump ready)

use crate::journal_reader::JournalData;
use chrono::{DateTime, Utc};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AlertKind {
    LowFuel,
    Overheating,
    Interdiction,
    InDanger,
    LowOxygen,
    LowHealth,
    NonHumanSignal,
    GuardianSignal,
    HighBioSignals,
    HighGeoSignals,
    ValuableBody,
    CarrierJumpReady,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone)]
pub struct Alert {
    pub kind: AlertKind,
    pub severity: Severity,
    pub title: String,
    pub body: String,
}

/// Which alert kinds are enabled.
#[derive(Clone, Copy)]
pub struct AlertConfig {
    pub enabled: bool,
    pub low_fuel: bool,
    pub overheating: bool,
    pub interdiction: bool,
    pub in_danger: bool,
    pub low_oxygen: bool,
    pub low_health: bool,
    pub non_human_signal: bool,
    pub guardian_signal: bool,
    pub high_bio_signals: bool,
    pub high_geo_signals: bool,
    /// A body needs at least this many biological/geological signals to count as "high".
    pub high_bio_threshold: u8,
    pub high_geo_threshold: u8,
    /// An undiscovered Earth-like, water or ammonia world, or a terraformable body.
    pub valuable_body: bool,
    pub carrier_jump_ready: bool,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            low_fuel: true,
            overheating: true,
            interdiction: true,
            in_danger: true,
            low_oxygen: true,
            low_health: true,
            non_human_signal: true,
            guardian_signal: true,
            high_bio_signals: true,
            high_geo_signals: true,
            high_bio_threshold: 3,
            high_geo_threshold: 3,
            valuable_body: true,
            carrier_jump_ready: true,
        }
    }
}

/// True when the system carries a signal whose USS type or name contains `needle`.
fn has_signal_matching(j: &JournalData, needle: &str) -> bool {
    j.discovered_signals.iter().any(|s| {
        s.uss_type
            .as_deref()
            .map(|u| u.to_lowercase().contains(needle))
            .unwrap_or(false)
            || s.display_name.to_lowercase().contains(needle)
    })
}

/// Highest biological (or geological) signal count found on any body in the current system.
fn max_signal_count(j: &JournalData, biological: bool) -> i32 {
    let matches = |sig: &crate::journal_reader::BodySignal| {
        if biological { sig.is_biological() } else { sig.is_geological() }
    };
    let mut max = 0;
    for saa in j.saa_data.values() {
        for sig in saa.signals.iter().filter(|s| matches(s)) {
            max = max.max(sig.count);
        }
    }
    for sigs in j.fss_signals.values() {
        for sig in sigs.iter().filter(|s| matches(s)) {
            max = max.max(sig.count);
        }
    }
    max
}

/// What makes a body worth an alert — an Earth-like, water or ammonia world, or any
/// terraformable body — as a name to say, or `None` for any other body.
/// The kind of a body worth finding first (earth-like, water and ammonia worlds, and any
/// terraformable), in words; `None` for the rest. The valuable-body alert and the explorer's
/// "Valuable" mark use it.
pub fn valuable_kind(b: &crate::journal_reader::BodyScan) -> Option<String> {
    let class = b.planet_class.to_lowercase();
    let terraformable = b.terraform_state.to_lowercase().starts_with("terraform");
    let world = if class.contains("earthlike") {
        "Earth-like world"
    } else if class.contains("water world") {
        "Water world"
    } else if class.contains("ammonia world") {
        "Ammonia world"
    } else if terraformable {
        return Some(format!("Terraformable {class}"));
    } else {
        return None;
    };
    Some(if terraformable && world != "Earth-like world" { format!("Terraformable {}", world.to_lowercase()) } else { world.to_string() })
}

/// A body's name without its system's in front, as the alert says it: "Body 5" for
/// "Hegoea XU-G d10-54 5". Voice alerts read the whole system designation otherwise.
fn short_body_name(body: &str, system: &str) -> String {
    match body.strip_prefix(system).map(str::trim) {
        Some(rest) if !rest.is_empty() => format!("Body {rest}"),
        _ => body.to_string(),
    }
}

pub struct AlertEngine {
    config: AlertConfig,
    /// Bodies (system address, body ID) already considered for the valuable-body alert,
    /// so each fires once however often its scans come round again.
    bodies_seen: std::collections::HashSet<(i64, i32)>,
    /// The first snapshot only initialises latches — never fires — so the startup journal
    /// replay doesn't produce a burst of stale alerts.
    seeded: bool,
    /// Edge latches keyed by kind: true ⇒ ready to fire on the next rising edge.
    armed: std::collections::HashMap<AlertKind, bool>,
    /// Scheduled carrier-jump departure and whether we've already fired for it.
    jump_at: Option<DateTime<Utc>>,
    jump_fired: bool,
    /// Raw (undebounced) `IsInDanger` reading from the previous *fresh* Status.json poll
    /// (tracked via `status_seq`, not call count — see below). Elite Dangerous can write a
    /// single stale `Status.json` poll with this bit spuriously set (observed when opening
    /// the on-foot ship-locker/backpack transfer UI, or during on-foot item/equipment
    /// interactions); only treat it as a real danger once it's held true across two
    /// consecutive fresh polls.
    in_danger_prev_raw: bool,
    /// `StatusData::status_seq` as of the last time the InDanger debounce advanced.
    /// `ingest()` is called once per `JournalData` broadcast, but a broadcast can be
    /// triggered by an unrelated file changing (Backpack.json, ShipLocker.json, journal
    /// lines, ...) while carrying forward the *same* unchanged Status.json snapshot —
    /// which would otherwise let one stale poll satisfy a two-consecutive-call debounce
    /// via a duplicate delivery. Gating on `status_seq` ensures the debounce only advances
    /// on snapshots where Status.json was actually re-read from disk.
    in_danger_last_status_seq: Option<u64>,
}

impl AlertEngine {
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config,
            seeded: false,
            bodies_seen: std::collections::HashSet::new(),
            armed: std::collections::HashMap::new(),
            jump_at: None,
            jump_fired: false,
            in_danger_prev_raw: false,
            in_danger_last_status_seq: None,
        }
    }

    pub fn set_config(&mut self, config: AlertConfig) {
        self.config = config;
    }

    /// Edge detector: fires `make()` once when `cond` rises, then disarms until `cond`
    /// clears again. The first (seeding) snapshot only sets the latch, never fires.
    fn edge(
        &mut self,
        out: &mut Vec<Alert>,
        seeding: bool,
        kind: AlertKind,
        cond: bool,
        kind_enabled: bool,
        make: impl FnOnce() -> Alert,
    ) {
        let armed = *self.armed.get(&kind).unwrap_or(&true);
        if cond && armed && !seeding && self.config.enabled && kind_enabled {
            out.push(make());
        }
        self.armed.insert(kind, !cond);
    }

    /// Feed a fresh journal snapshot. Returns any alerts triggered by state changes since
    /// the previous snapshot. The first call only seeds internal latches.
    pub fn ingest(&mut self, j: &JournalData) -> Vec<Alert> {
        let mut out = Vec::new();
        let seeding = !self.seeded;
        self.seeded = true;
        let s = &j.status;
        let valid = s.valid;
        let cfg = self.config;

        // ── Ship state edges ─────────────────────────────────────
        self.edge(&mut out, seeding, AlertKind::LowFuel, valid && s.low_fuel(), cfg.low_fuel, || {
            let pct = if j.pilot.fuel_capacity > 0.0 {
                (s.fuel_main / j.pilot.fuel_capacity * 100.0).round()
            } else {
                0.0
            };
            Alert {
                kind: AlertKind::LowFuel,
                severity: Severity::Warning,
                title: "Low fuel".into(),
                body: format!("{pct:.0}% remaining ({:.1} t)", s.fuel_main),
            }
        });
        self.edge(&mut out, seeding, AlertKind::Overheating, valid && s.overheating(), cfg.overheating, || Alert {
            kind: AlertKind::Overheating,
            severity: Severity::Critical,
            title: "Overheating".into(),
            body: "Ship temperature critical — vent heat now.".into(),
        });
        self.edge(&mut out, seeding, AlertKind::Interdiction, valid && s.being_interdicted(), cfg.interdiction, || Alert {
            kind: AlertKind::Interdiction,
            severity: Severity::Critical,
            title: "Being interdicted!".into(),
            body: "Submit or fight — throttle to zero to drop quickly.".into(),
        });
        // Debounced: only treat IsInDanger as real once it's held true across two
        // consecutive *fresh* Status.json polls (see field docs). Gated on `status_seq`
        // rather than call count — `ingest()` can be invoked with a carried-over stale
        // status snapshot whenever some other watched file changes (Backpack.json/
        // ShipLocker.json churn during on-foot activity like locker access or item/
        // equipment interactions), which would otherwise let a single stale poll satisfy
        // a naive two-call debounce via duplicate delivery.
        if Some(s.status_seq) != self.in_danger_last_status_seq {
            self.in_danger_last_status_seq = Some(s.status_seq);
            let in_danger_raw = valid && s.in_danger();
            let in_danger_confirmed = in_danger_raw && self.in_danger_prev_raw;
            self.in_danger_prev_raw = in_danger_raw;
            self.edge(&mut out, seeding, AlertKind::InDanger, in_danger_confirmed, cfg.in_danger, || Alert {
                kind: AlertKind::InDanger,
                severity: Severity::Warning,
                title: "In danger".into(),
                body: "Hostiles or a hazard have been detected nearby.".into(),
            });
        }

        // ── On-foot life support edges ───────────────────────────
        self.edge(&mut out, seeding, AlertKind::LowOxygen, valid && s.low_oxygen(), cfg.low_oxygen, || Alert {
            kind: AlertKind::LowOxygen,
            severity: Severity::Critical,
            title: "Low oxygen".into(),
            body: "Suit oxygen running out — find a breathable atmosphere.".into(),
        });
        self.edge(&mut out, seeding, AlertKind::LowHealth, valid && s.low_health(), cfg.low_health, || Alert {
            kind: AlertKind::LowHealth,
            severity: Severity::Critical,
            title: "Low health".into(),
            body: "Suit health critical.".into(),
        });

        // ── System-event edges (reset when signals clear on a jump) ──
        let non_human = has_signal_matching(j, "nonhuman")
            || j.discovered_signals.iter().any(|s| s.display_name.to_lowercase().contains("thargoid"));
        self.edge(&mut out, seeding, AlertKind::NonHumanSignal, non_human, cfg.non_human_signal, || Alert {
            kind: AlertKind::NonHumanSignal,
            severity: Severity::Warning,
            title: "Non-human signal".into(),
            body: "A non-human signal source is present in this system.".into(),
        });
        self.edge(&mut out, seeding, AlertKind::GuardianSignal, has_signal_matching(j, "guardian"), cfg.guardian_signal, || Alert {
            kind: AlertKind::GuardianSignal,
            severity: Severity::Info,
            title: "Guardian signal".into(),
            body: "A Guardian signal source is present in this system.".into(),
        });

        // High biological / geological signal counts (informational, threshold-based).
        let bio = max_signal_count(j, true);
        let bio_high = cfg.high_bio_threshold > 0 && bio >= cfg.high_bio_threshold as i32;
        self.edge(&mut out, seeding, AlertKind::HighBioSignals, bio_high, cfg.high_bio_signals, || Alert {
            kind: AlertKind::HighBioSignals,
            severity: Severity::Info,
            title: "High bio signals".into(),
            body: format!("A body here has {bio} biological signals."),
        });
        let geo = max_signal_count(j, false);
        let geo_high = cfg.high_geo_threshold > 0 && geo >= cfg.high_geo_threshold as i32;
        self.edge(&mut out, seeding, AlertKind::HighGeoSignals, geo_high, cfg.high_geo_signals, || Alert {
            kind: AlertKind::HighGeoSignals,
            severity: Severity::Info,
            title: "High geo signals".into(),
            body: format!("A body here has {geo} geological signals."),
        });

        // ── Undiscovered valuable bodies (event: once per body) ──
        // Only the commander's own scans: one that EDDN reported isn't theirs to find.
        if let Some((addr, system)) = j.current_system.as_ref().map(|s| (s.system_address, s.name.as_str())) {
            for b in &j.bodies {
                let undiscovered = b.discovery.is_some_and(|d| !d.discovered && !d.reported);
                let Some(kind) = valuable_kind(b).filter(|_| undiscovered) else { continue };
                if !self.bodies_seen.insert((addr, b.body_id)) || seeding || !cfg.enabled || !cfg.valuable_body {
                    continue;
                }
                let worth = b.credits_if_mapped().or_else(|| b.credits()).filter(|&c| c > 0);
                let worth = worth.map(|c| format!(", worth {} mapped", edcas_common::format::credits(c))).unwrap_or_default();
                out.push(Alert {
                    kind: AlertKind::ValuableBody,
                    severity: Severity::Info,
                    title: format!("Undiscovered {}", kind.to_lowercase()),
                    body: format!("{} at {:.0} Ls{worth}.", short_body_name(&b.body_name, system), b.distance_from_arrival_ls),
                });
            }
        }

        // ── Carrier jump schedule (timer setup) ──────────────────
        // A changed departure means a new request (or a cancellation ⇒ None); reset the latch.
        if j.carrier_jump_departure != self.jump_at {
            self.jump_at = j.carrier_jump_departure;
            self.jump_fired = false;
        }

        out
    }

    /// Evaluate time-based alerts. Call regularly (the alert service does so every second).
    pub fn tick(&mut self, now: DateTime<Utc>) -> Vec<Alert> {
        let mut out = Vec::new();
        if let Some(at) = self.jump_at {
            if !self.jump_fired && now >= at && self.config.enabled && self.config.carrier_jump_ready {
                self.jump_fired = true;
                out.push(Alert {
                    kind: AlertKind::CarrierJumpReady,
                    severity: Severity::Info,
                    title: "Carrier jump ready".into(),
                    body: "Your fleet carrier has reached its destination.".into(),
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal_reader::JournalData;

    const LOW_FUEL_BIT: u32 = 1 << 19;
    const INTERDICTED_BIT: u32 = 1 << 23;

    fn engine() -> AlertEngine {
        AlertEngine::new(AlertConfig::default())
    }

    /// Each call simulates a genuinely fresh Status.json poll (distinct `status_seq`),
    /// matching how the real watcher only bumps it when the file's mtime actually changed.
    fn journal_with_flags(flags: u32) -> JournalData {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(1);
        let mut j = JournalData::new();
        j.status.valid = true;
        j.status.flags = flags;
        j.status.status_seq = SEQ.fetch_add(1, Ordering::Relaxed);
        j
    }

    #[test]
    fn first_snapshot_seeds_without_firing() {
        let mut e = engine();
        // Starting already low on fuel must not fire on the seed snapshot.
        let fired = e.ingest(&journal_with_flags(LOW_FUEL_BIT));
        assert!(fired.is_empty());
    }

    #[test]
    fn low_fuel_edge_fires_once_then_rearms() {
        let mut e = engine();
        e.ingest(&journal_with_flags(0)); // seed: healthy
        // Crossing into low fuel fires exactly once.
        assert_eq!(e.ingest(&journal_with_flags(LOW_FUEL_BIT)).len(), 1);
        // Staying low does not re-fire.
        assert!(e.ingest(&journal_with_flags(LOW_FUEL_BIT)).is_empty());
        // Recovering does not fire.
        assert!(e.ingest(&journal_with_flags(0)).is_empty());
        // Going low again re-fires.
        assert_eq!(e.ingest(&journal_with_flags(LOW_FUEL_BIT)).len(), 1);
    }

    #[test]
    fn in_danger_single_poll_blip_does_not_fire() {
        const IN_DANGER_BIT: u32 = 1 << 22;
        let mut e = engine();
        e.ingest(&journal_with_flags(0)); // seed: safe
        // A single stale poll with the bit set (e.g. the on-foot locker UI quirk) must not fire.
        assert!(e.ingest(&journal_with_flags(IN_DANGER_BIT)).is_empty());
        // Clearing again on the next poll confirms it never armed.
        assert!(e.ingest(&journal_with_flags(0)).is_empty());
    }

    #[test]
    fn in_danger_sustained_across_two_polls_fires_once() {
        const IN_DANGER_BIT: u32 = 1 << 22;
        let mut e = engine();
        e.ingest(&journal_with_flags(0)); // seed: safe
        assert!(e.ingest(&journal_with_flags(IN_DANGER_BIT)).is_empty()); // 1st poll: pending
        let fired = e.ingest(&journal_with_flags(IN_DANGER_BIT)); // 2nd consecutive poll: confirmed
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].kind, AlertKind::InDanger);
        // Staying in danger does not re-fire.
        assert!(e.ingest(&journal_with_flags(IN_DANGER_BIT)).is_empty());
        // Recovering, then going low again, re-fires after a fresh two-poll confirmation.
        assert!(e.ingest(&journal_with_flags(0)).is_empty());
        assert!(e.ingest(&journal_with_flags(IN_DANGER_BIT)).is_empty());
        assert_eq!(e.ingest(&journal_with_flags(IN_DANGER_BIT)).len(), 1);
    }

    #[test]
    fn in_danger_stale_status_redelivered_by_unrelated_file_churn_does_not_fire() {
        // Regression test: opening the ship locker OR performing on-foot item/equipment
        // interactions (e.g. installing a component, "welding") causes Backpack.json/
        // ShipLocker.json to churn every few seconds. The watcher broadcasts a fresh
        // JournalData on *any* watched file change, carrying forward the same unchanged
        // Status.json snapshot (same status_seq) when only those files changed. A naive
        // "fires on two consecutive ingest() calls" debounce would treat that duplicate
        // delivery as a second confirming poll and fire on a single spurious blip.
        const IN_DANGER_BIT: u32 = 1 << 22;
        let mut e = engine();
        let mut j = journal_with_flags(0);
        e.ingest(&j); // seed: safe

        // One genuine fresh poll flips the bit (the spurious blip itself).
        j.status.flags = IN_DANGER_BIT;
        j.status.status_seq += 1;
        assert!(e.ingest(&j).is_empty());

        // Unrelated file churn re-broadcasts the identical snapshot several times —
        // status_seq does NOT change, so none of these may confirm/fire the alert.
        for _ in 0..5 {
            assert!(e.ingest(&j).is_empty());
        }
    }

    #[test]
    fn interdiction_is_critical() {
        let mut e = engine();
        e.ingest(&journal_with_flags(0));
        let fired = e.ingest(&journal_with_flags(INTERDICTED_BIT));
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].kind, AlertKind::Interdiction);
        assert_eq!(fired[0].severity, Severity::Critical);
    }

    #[test]
    fn disabled_kind_does_not_fire() {
        let mut cfg = AlertConfig::default();
        cfg.low_fuel = false;
        let mut e = AlertEngine::new(cfg);
        e.ingest(&journal_with_flags(0));
        assert!(e.ingest(&journal_with_flags(LOW_FUEL_BIT)).is_empty());
    }

    #[test]
    fn high_bio_signals_respect_threshold() {
        use crate::journal_reader::BodySignal;
        let bio = |count: i32| {
            let mut j = JournalData::new();
            j.fss_signals.insert(1, vec![BodySignal {
                signal_type: "$SAA_SignalType_Biological;".into(),
                signal_type_localised: None,
                count,
            }]);
            j
        };
        // Threshold is 3 by default: 2 signals must not fire, 3 must.
        let mut e = engine();
        e.ingest(&JournalData::new()); // seed: no signals
        assert!(e.ingest(&bio(2)).is_empty());
        // Returning to a signal-free system re-arms (signals clear on jump).
        e.ingest(&JournalData::new());
        let fired = e.ingest(&bio(3));
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].kind, AlertKind::HighBioSignals);
    }

    #[test]
    fn high_bio_threshold_zero_never_fires() {
        use crate::journal_reader::BodySignal;
        let mut cfg = AlertConfig::default();
        cfg.high_bio_threshold = 0;
        let mut e = AlertEngine::new(cfg);
        let mut j = JournalData::new();
        j.fss_signals.insert(1, vec![BodySignal {
            signal_type: "$SAA_SignalType_Biological;".into(),
            signal_type_localised: None,
            count: 9,
        }]);
        e.ingest(&JournalData::new());
        assert!(e.ingest(&j).is_empty());
    }

    #[test]
    fn undiscovered_valuable_body_fires_once_per_body() {
        let scan = |id: i32, name: &str, class: &str, terraform: &str, discovered: bool| {
            format!(
                r#"{{ "timestamp":"2026-10-14T11:00:00Z", "event":"Scan", "ScanType":"Detailed", "BodyName":"{name}", "BodyID":{id}, "StarSystem":"Oochont", "SystemAddress":42, "DistanceFromArrivalLS":120.4, "PlanetClass":"{class}", "TerraformState":"{terraform}", "Radius":6000000.0, "MassEM":1.0, "WasDiscovered":{discovered}, "WasMapped":false }}"#
            )
        };
        let mut j = JournalData::new();
        j.process_line(r#"{ "timestamp":"2026-10-14T11:00:00Z", "event":"FSDJump", "StarSystem":"Oochont", "SystemAddress":42, "StarPos":[0.0,0.0,0.0], "Population":0 }"#);
        // A water world already in the system when the app starts doesn't fire.
        j.process_line(&scan(1, "Oochont 1", "Water world", "", false));
        let mut e = engine();
        assert!(e.ingest(&j).is_empty());

        j.process_line(&scan(2, "Oochont 2", "Earthlike body", "", false));
        let fired = e.ingest(&j);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].kind, AlertKind::ValuableBody);
        assert_eq!(fired[0].title, "Undiscovered earth-like world");
        // The voice reads the body, not the system's designation again.
        assert!(fired[0].body.starts_with("Body 2 at 120 Ls"), "{}", fired[0].body);
        // Scanning it again, or another snapshot, doesn't repeat it.
        j.process_line(&scan(2, "Oochont 2", "Earthlike body", "", false));
        assert!(e.ingest(&j).is_empty());

        // Someone found it first; a plain rocky body is no find.
        j.process_line(&scan(3, "Oochont 3", "Ammonia world", "", true));
        j.process_line(&scan(4, "Oochont 4", "Rocky body", "", false));
        assert!(e.ingest(&j).is_empty());

        j.process_line(&scan(5, "Oochont 5", "High metal content body", "Terraformable", false));
        let fired = e.ingest(&j);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].title, "Undiscovered terraformable high metal content body");
    }

    #[test]
    fn a_body_is_named_without_its_system() {
        assert_eq!(short_body_name("Hegoea XU-G d10-54 5", "Hegoea XU-G d10-54"), "Body 5");
        assert_eq!(short_body_name("Hegoea EN-F b39-0 B 6 a", "Hegoea EN-F b39-0"), "Body B 6 a");
        // A body with a name of its own keeps it.
        assert_eq!(short_body_name("Earth", "Sol"), "Earth");
    }

    #[test]
    fn carrier_jump_timer_fires_once_when_due() {
        let mut e = engine();
        let mut j = JournalData::new();
        j.carrier_jump_departure = Some(Utc::now() - chrono::Duration::seconds(1));
        e.ingest(&j); // seeds the schedule
        // Tick after the departure time fires exactly once.
        assert_eq!(e.tick(Utc::now()).len(), 1);
        assert!(e.tick(Utc::now()).is_empty());
    }
}
