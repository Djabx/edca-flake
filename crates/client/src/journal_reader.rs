use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use edcas_common::exploration::ScanValue;
use edcas_common::api::{ConstructionDepotSubmission, ConstructionResourceSubmission};
use edcas_common::journal::{
    CarrierJump, FsdJump, FssSignalDiscovered, JournalEvent, Location, Scan,
};
use edcas_common::journal::types::Conflict;
use tracing::{debug, error, info, warn};
use serde_json;

#[derive(Clone)]
pub struct ConflictData {
    pub war_type: String,
    pub status: String,
    pub opponent: String,
    pub our_won_days: i32,
    pub opponent_won_days: i32,
}

#[derive(Clone)]
pub struct FactionInfo {
    pub name: String,
    pub influence: f32,
    pub active_states: Vec<String>,
    pub pending_states: Vec<String>,
    pub recovering_states: Vec<String>,
    pub conflict: Option<ConflictData>,
}

#[derive(Clone)]
pub struct SystemData {
    pub name: String,
    pub system_address: i64,
    pub economy: String,
    pub second_economy: String,
    pub government: String,
    pub allegiance: String,
    pub security: String,
    pub population: i64,
    pub factions: Vec<FactionInfo>,
    pub system_faction: String,
    pub controlling_power: Option<String>,
    pub powers: Vec<String>,
    /// Galactic coordinates in ly (`StarPos`), when the event carried them.
    pub star_pos: Option<[f32; 3]>,
}

#[derive(Clone, Default)]
pub struct BodyScan {
    pub body_id: i32,
    pub body_name: String,
    pub planet_class: String,
    pub landable: bool,
    pub scan_type: String,
    pub distance_from_arrival_ls: f32,
    pub radius: f32,
    pub mass_em: f32,
    pub surface_temperature: f32,
    pub surface_gravity: f32,
    pub tidal_lock: bool,
    pub volcanism: String,
    pub atmosphere: String,
    pub terraform_state: String,
    pub star_type: String,
    pub parents: Vec<BodyParent>,
    pub rings: Vec<BodyRing>,
    pub materials: Vec<BodyMaterial>,
    /// What the scan sells for, with [`BodyScan::mapped`]; `None` for barycentres and
    /// bodies only known from the server.
    pub value: Option<ScanValue>,
    /// The commander's own surface map (`SAAScanComplete`): whether it met the probe
    /// efficiency target.
    pub mapped: Option<bool>,
    /// The commander set foot on it (`Disembark` on the planet).
    pub footfall: bool,
    pub composition: Option<BodyComposition>,
    /// What the galaxy knew of the body when it was scanned; `None` for barycentres and
    /// scans from before 3.3.
    pub discovery: Option<Discovery>,
    /// What the exobiology prediction needs besides the fields above; `None` for bodies
    /// not scanned by the commander (barycentres, the server's bodies).
    pub exo: Option<ExoScan>,
}

/// Scan details only the exobiology prediction reads.
#[derive(Clone, Default)]
pub struct ExoScan {
    /// `AtmosphereType`, e.g. "CarbonDioxide"; "None" or empty when airless.
    pub atmosphere_type: String,
    /// Sulphur dioxide share of the atmosphere, in percent.
    pub so2_percent: f32,
    /// Pa.
    pub surface_pressure: f32,
    /// `AtmosphereComposition`: every gas and its share in percent; empty when airless or
    /// when the scan didn't carry it.
    pub composition: Vec<AtmosphereGas>,
}

/// One gas of a scanned atmosphere.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AtmosphereGas {
    /// The game's name for it, e.g. "CarbonDioxide".
    pub name: String,
    pub percent: f32,
}

impl BodyScan {
    /// What its data sells for now, the commander's surface map included.
    pub fn credits(&self) -> Option<i64> {
        self.value.map(|v| v.credits(self.mapped))
    }

    /// What it would sell for after a surface map within the probe target, while that's
    /// still to do.
    pub fn credits_if_mapped(&self) -> Option<i64> {
        self.value.filter(|v| v.mappable() && self.mapped.is_none()).map(|v| v.credits(Some(true)))
    }
}

/// A scan's `WasDiscovered` / `WasMapped` / `WasFootfalled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Discovery {
    pub discovered: bool,
    pub mapped: bool,
    /// Planets only, and only in journals since Odyssey started tracking it.
    pub footfalled: Option<bool>,
    /// Another commander's scan, reported through EDDN, not the commander's own.
    pub reported: bool,
}

#[derive(Clone)]
pub struct BodyComposition {
    pub ice: f32,
    pub rock: f32,
    pub metal: f32,
}

#[derive(Clone)]
pub struct BodyParent {
    pub body_id: i32,
}

#[derive(Clone)]
pub struct BodyRing {
    pub name: String,
    pub ring_class: String,
}

#[derive(Clone)]
pub struct BodyMaterial {
    pub name: String,
    pub percent: f64,
}

#[derive(Clone)]
pub struct BodySignal {
    pub signal_type: String,
    pub signal_type_localised: Option<String>,
    pub count: i32,
}

#[derive(Clone)]
pub struct DiscoveredSignal {
    pub display_name: String,
    /// Raw signal type string from the journal (e.g. "FleetCarrier", "StationAsteroid", USS types).
    pub signal_type: Option<String>,
    pub uss_type: Option<String>,
    pub spawning_state: Option<String>,
    pub spawning_faction: Option<String>,
    pub threat_level: Option<i32>,
    pub time_remaining: Option<f32>,
    pub is_station: bool,
    /// Set when the journal includes `BodyID` — lets us attach this signal to a body in the tree.
    pub body_id: Option<i32>,
}

impl DiscoveredSignal {
    pub fn from_event(e: &FssSignalDiscovered) -> Self {
        let display_name = e.signal_name_localised
            .as_deref()
            .unwrap_or_else(|| clean_signal_name(&e.signal_name))
            .to_string();
        let uss_type = e.uss_type_localised.clone().or_else(|| e.uss_type.clone());
        let spawning_state = e.spawning_state_localised.clone().or_else(|| e.spawning_state.clone());
        Self {
            display_name,
            signal_type: e.signal_type.clone(),
            uss_type,
            spawning_state,
            spawning_faction: e.spawning_faction.clone(),
            threat_level: e.threat_level,
            time_remaining: e.time_remaining,
            is_station: e.is_station,
            body_id: e.body_id,
        }
    }
}

fn clean_signal_name(raw: &str) -> &str {
    raw.trim_start_matches('$').trim_end_matches(';')
}

/// Return `localised` if non-empty, otherwise strip `$..._name;` / `$...;` wrappers from `raw`.
fn localised_or_clean(localised: &str, raw: &str) -> String {
    if !localised.is_empty() {
        return localised.to_owned();
    }
    raw.trim_start_matches('$')
        .trim_end_matches("_name;")
        .trim_end_matches(';')
        .to_owned()
}

impl BodySignal {
    pub fn is_biological(&self) -> bool {
        self.signal_type.contains("Biological")
    }
    pub fn is_geological(&self) -> bool {
        self.signal_type.contains("Geological")
    }
    pub fn display_type(&self) -> &str {
        self.signal_type_localised.as_deref().unwrap_or(&self.signal_type)
    }
}

#[derive(Clone)]
pub struct SaaBodyData {
    pub signals: Vec<BodySignal>,
    pub genuses: Vec<GenusSignal>,
}

/// A genus a surface scan found on a body.
#[derive(Clone)]
pub struct GenusSignal {
    /// The codex symbol (`$Codex_Ent_Bacterial_Genus_Name;`), the same in every language.
    pub symbol: String,
    /// The name in the game's language.
    pub name: String,
}

#[derive(Clone)]
pub struct StationData {
    pub name: String,
    pub station_type: String,
    pub system_name: String,
    pub dist_from_star_ls: f32,
    pub services: Vec<String>,
    pub economy: String,
    pub secondary_economies: Vec<(String, f32)>,
    pub faction: String,
    pub government: String,
    pub allegiance: String,
    pub landing_pads: Option<(i32, i32, i32)>,
    pub market_id: i64,
    /// Body the station orbits, taken from the SupercruiseExit or Location BodyID.
    pub host_body_id: Option<i32>,
    /// Locally-parsed commodities from Market.json at the time of docking.
    /// Empty until Market.json is detected for this station's market_id.
    pub commodities: Vec<edcas_common::api::CommodityResponse>,
}

#[derive(Clone, Default)]
pub struct InventoryItem {
    pub name: String,
    pub localised: String,
    pub count: i32,
}

#[derive(Clone, Default)]
pub struct CargoItem {
    pub name: String,
    pub localised: String,
    pub count: i32,
    pub stolen: i32,
}

#[derive(Clone, Default)]
pub struct OnFootInventory {
    pub items: Vec<InventoryItem>,
    pub components: Vec<InventoryItem>,
    pub consumables: Vec<InventoryItem>,
    pub data: Vec<InventoryItem>,
}

#[derive(Clone)]
pub struct EngineeringModifier {
    pub label: String,
    pub value: f32,
    pub original_value: f32,
    pub less_is_good: bool,
}

#[derive(Clone)]
pub struct EngineeringData {
    pub blueprint: String,
    pub level: u8,
    pub engineer: String,
    pub experimental: String,
    pub modifiers: Vec<EngineeringModifier>,
}

#[derive(Clone)]
pub struct ShipModule {
    pub slot: String,
    pub item: String,
    pub power: f32,
    pub priority: u8,
    pub health: Option<f32>,
    pub value: Option<i64>,
    pub engineering: Option<EngineeringData>,
}

#[derive(Clone)]
pub struct SuitWeapon {
    pub slot: String,
    pub name: String,
    pub class: u8,
    pub mods: Vec<String>,
}

#[derive(Clone)]
pub struct SuitData {
    pub suit_type: String,
    pub grade: u8,
    pub loadout_name: String,
    pub mods: Vec<String>,
    pub weapons: Vec<SuitWeapon>,
}

#[derive(Clone)]
pub struct PilotData {
    pub name: String,
    pub credits: i64,
    pub ship_type: String,
    pub ship_name: String,
    pub ship_ident: String,
    /// Minimum landing pad size required by the current ship: 'S', 'M', or 'L'.
    /// Defaults to 'S' (no restriction) until a Loadout event is processed.
    pub ship_pad_size: char,
    pub fuel_level: f32,
    pub fuel_capacity: f32,
    pub reserve_fuel_capacity: f32,
    pub hull_health: f32,
    pub max_jump_range: f32,
    pub unladen_mass: f32,
    pub cargo_capacity: i32,
    pub modules_value: i64,
    pub rebuy: i64,
    pub game_mode: String,
    pub horizons: bool,
    pub odyssey: bool,
    pub rank_combat: u8,
    pub rank_trade: u8,
    pub rank_explore: u8,
    pub rank_soldier: u8,
    pub rank_exobiologist: u8,
    pub rank_empire: u8,
    pub rank_federation: u8,
    pub rank_cqc: u8,
    pub progress_combat: u8,
    pub progress_trade: u8,
    pub progress_explore: u8,
    pub progress_soldier: u8,
    pub progress_exobiologist: u8,
    pub progress_empire: u8,
    pub progress_federation: u8,
    pub progress_cqc: u8,
    pub reputation_empire: f32,
    pub reputation_federation: f32,
    pub reputation_alliance: f32,
    pub power: String,
    pub power_merits: i64,
    pub suit: Option<SuitData>,
}

impl Default for PilotData {
    fn default() -> Self {
        Self {
            name: String::new(),
            credits: 0,
            ship_type: String::new(),
            ship_name: String::new(),
            ship_ident: String::new(),
            ship_pad_size: 'S',
            fuel_level: 0.0,
            fuel_capacity: 0.0,
            reserve_fuel_capacity: 0.0,
            hull_health: 0.0,
            max_jump_range: 0.0,
            unladen_mass: 0.0,
            cargo_capacity: 0,
            modules_value: 0,
            rebuy: 0,
            game_mode: String::new(),
            horizons: false,
            odyssey: false,
            rank_combat: 0,
            rank_trade: 0,
            rank_explore: 0,
            rank_soldier: 0,
            rank_exobiologist: 0,
            rank_empire: 0,
            rank_federation: 0,
            rank_cqc: 0,
            progress_combat: 0,
            progress_trade: 0,
            progress_explore: 0,
            progress_soldier: 0,
            progress_exobiologist: 0,
            progress_empire: 0,
            progress_federation: 0,
            progress_cqc: 0,
            reputation_empire: 0.0,
            reputation_federation: 0.0,
            reputation_alliance: 0.0,
            power: String::new(),
            power_merits: 0,
            suit: None,
        }
    }
}

/// Live telemetry parsed from `Status.json`, the file the game rewrites continuously
/// while playing. Holds the most recent snapshot; `valid` is false until the first read.
#[derive(Clone, Default)]
pub struct StatusData {
    pub valid: bool,
    pub flags: u32,
    pub flags2: u32,
    /// System / Engine / Weapon pips, in half-pip units (0..=8 ⇒ 0.0..=4.0 when halved).
    pub pips_sys: u8,
    pub pips_eng: u8,
    pub pips_wep: u8,
    pub firegroup: u8,
    /// Current main-tank fuel in tons.
    pub fuel_main: f32,
    /// Cargo currently carried, in tons.
    pub cargo_tons: f32,
    pub legal_state: String,
    pub body_name: String,
    pub altitude: Option<f64>,
    pub destination: String,
    /// The targeted destination's system address and body ID (Status.json `Destination`);
    /// the ID names a body only when `destination` is that body's name.
    pub destination_system: i64,
    pub destination_body: i32,
    // On-foot (Odyssey) telemetry; None when in a ship/SRV.
    pub health: Option<f32>,
    pub oxygen: Option<f32>,
    pub temperature_k: Option<f32>,
    pub gravity: Option<f32>,
    /// Incremented every time this snapshot was produced by an actual fresh read of
    /// Status.json (i.e. its mtime changed), as opposed to being carried over unchanged
    /// into a `JournalData` broadcast triggered by some other file's change. Lets
    /// consumers (e.g. the alert debounce) tell a genuinely new poll of Status.json
    /// apart from a stale copy re-delivered alongside unrelated activity (Backpack.json/
    /// ShipLocker.json churn, journal-line updates, ...).
    pub status_seq: u64,
}

impl StatusData {
    #[inline]
    fn flag(&self, bit: u32) -> bool { self.flags & (1 << bit) != 0 }
    #[inline]
    fn flag2(&self, bit: u32) -> bool { self.flags2 & (1 << bit) != 0 }

    pub fn docked(&self) -> bool { self.flag(0) }
    pub fn landed(&self) -> bool { self.flag(1) }
    pub fn landing_gear(&self) -> bool { self.flag(2) }
    pub fn shields_up(&self) -> bool { self.flag(3) }
    pub fn supercruise(&self) -> bool { self.flag(4) }
    pub fn hardpoints(&self) -> bool { self.flag(6) }
    pub fn lights_on(&self) -> bool { self.flag(8) }
    pub fn cargo_scoop(&self) -> bool { self.flag(9) }
    pub fn silent_running(&self) -> bool { self.flag(10) }
    pub fn scooping_fuel(&self) -> bool { self.flag(11) }
    pub fn mass_locked(&self) -> bool { self.flag(16) }
    pub fn fsd_charging(&self) -> bool { self.flag(17) }
    pub fn fsd_cooldown(&self) -> bool { self.flag(18) }
    pub fn low_fuel(&self) -> bool { self.flag(19) }
    pub fn overheating(&self) -> bool { self.flag(20) }
    pub fn in_danger(&self) -> bool { self.flag(22) }
    pub fn being_interdicted(&self) -> bool { self.flag(23) }
    pub fn in_fighter(&self) -> bool { self.flag(25) }
    pub fn in_srv(&self) -> bool { self.flag(26) }
    pub fn analysis_mode(&self) -> bool { self.flag(27) }
    pub fn night_vision(&self) -> bool { self.flag(28) }

    pub fn on_foot(&self) -> bool { self.flag2(0) }
    pub fn in_taxi(&self) -> bool { self.flag2(1) }

    pub fn low_oxygen(&self) -> bool { self.flag2(6) }
    pub fn low_health(&self) -> bool { self.flag2(7) }
}

/// Counters accumulated over the current game session (reset when a new journal file —
/// i.e. a fresh session — is opened).
#[derive(Clone, Default)]
pub struct SessionStats {
    pub jumps: u32,
    pub light_years: f64,
    pub systems_visited: u32,
    pub bodies_scanned: u32,
    pub bodies_mapped: u32,
    pub bio_samples: u32,
    pub bio_species_complete: u32,
    pub dockings: u32,
    /// Credits earned from selling exploration data (Universal Cartographics) this session.
    pub exploration_earned: i64,
    /// Credits earned from selling exobiology data (Vista Genomics) this session.
    pub exobio_earned: i64,
}

/// A construction depot the player has visited, ready to submit to the server.
#[derive(Clone)]
pub struct ConstructionDepotData {
    pub submission: ConstructionDepotSubmission,
    pub system_name: String,
}

/// Upper bound on the most-recent-first session history lists (`visited_systems`,
/// `visited_stations`). These are never cleared, only ever prepended to, and are read purely
/// as recent-activity logs, so on a long exploration session they would otherwise grow without
/// limit (and the whole struct is cloned per event). Cap to the most recent N — dropping the
/// oldest entries does not change what the UI shows.
const MAX_VISITED_HISTORY: usize = 4096;

#[derive(Clone)]
pub struct JournalData {
    pub current_system: Option<SystemData>,
    pub bodies: Vec<BodyScan>,
    pub fss_signals: HashMap<i32, Vec<BodySignal>>,
    pub saa_data: HashMap<i32, SaaBodyData>,
    pub stations: Vec<StationData>,
    pub discovered_signals: Vec<DiscoveredSignal>,
    pub materials_raw: Vec<InventoryItem>,
    pub materials_manufactured: Vec<InventoryItem>,
    pub materials_encoded: Vec<InventoryItem>,
    pub cargo: Vec<CargoItem>,
    pub backpack: OnFootInventory,
    pub shiplocker: OnFootInventory,
    pub modules: Vec<ShipModule>,
    pub loadout_health: HashMap<String, f32>,
    pub loadout_engineering: HashMap<String, Option<EngineeringData>>,
    pub pilot: PilotData,
    /// Live telemetry from Status.json (fuel, hull, pips, flags). File-based.
    pub status: StatusData,
    /// In witch space: a hyperspace `StartJump` was followed by the music stopping (`Music`
    /// "NoTrack", 5 s later, when the ship leaves the system; `Status.json`'s jump flag is set
    /// during the countdown already), and no `FSDJump` has come yet.
    pub witch_space: bool,
    /// A hyperspace `StartJump` without its `FSDJump` so far (the countdown or the jump).
    jump_pending: bool,
    /// The `StarClass` of the star the pending hyperspace jump goes to (its light at arrival).
    jump_star_class: String,
    /// Scheduled fleet-carrier jump arrival time, from CarrierJumpRequest; cleared on
    /// CarrierJumpCancelled or once the jump completes.
    pub carrier_jump_departure: Option<chrono::DateTime<chrono::Utc>>,
    /// Counters accumulated over the current game session.
    pub session: SessionStats,
    /// Most recently visited construction depots, keyed by market_id.
    pub construction_depots: HashMap<i64, ConstructionDepotData>,
    /// Systems claimed by the player via ColonisationSystemClaim, keyed by system_address.
    pub claimed_systems: HashMap<i64, String>,
    /// (market_id, station_name, system_name, system_address) of the last Docked event.
    pub last_docked: Option<(i64, String, String, i64)>,
    /// Running tally of cargo transferred to each fleet carrier this session,
    /// keyed by carrier market_id → commodity name (lowercase) → count.
    /// Populated live from CargoTransfer events; no snapshot file exists so it
    /// starts at zero each session.
    pub carrier_cargo: HashMap<i64, HashMap<String, i32>>,
    /// Best-known localised display name for a normalised commodity key, built up from any
    /// event that carries a `Type_Localised` field alongside `Type`.
    pub commodity_names: HashMap<String, String>,
    /// On-foot materials stored on each fleet carrier, loaded from FCMaterials.json.
    /// Keyed by carrier market_id → (localised_name, stock).
    pub carrier_fc_materials: HashMap<i64, Vec<(String, i32)>>,
    /// BodyID from the most recent SupercruiseExit where BodyType != "Station".
    /// Cleared after consumption by the next Docked event or system jump.
    pending_host_body_id: Option<i32>,
    /// All visited systems this session, most recent first.
    pub visited_systems: Vec<SystemData>,
    /// All docked non-carrier stations this session, most recent first.
    pub visited_stations: Vec<StationData>,
    /// All docked fleet carriers this session, most recent first.
    pub visited_carriers: Vec<StationData>,
    pub fss_body_count: Option<u32>,
    pub fss_non_body_count: Option<u32>,
    pub fss_all_bodies_found: bool,
    pub nav_beacon_bodies: Option<u32>,
    /// The body the commander last turned to in the current system — targeted it, approached
    /// it, dropped out of supercruise at it, or scanned it in detail (FSS or surface map) —
    /// with a count of such moments so far, so the explorer can tell a new one from the
    /// last it followed.
    pub body_in_focus: Option<(u64, i32)>,
    /// Timestamp of the most recently processed journal event (ISO 8601 string).
    pub latest_event_timestamp: String,
    /// `event` name of the most recently processed journal event.
    pub latest_event: String,
    /// When set, CargoTransfer events for seeded carriers with timestamp ≤ this value
    /// are skipped to avoid double-counting after an in-session restart.
    /// Cleared after the initial file load so live events are never skipped.
    pub carrier_cargo_skip_before: Option<String>,
    /// Market IDs that were seeded from my_carriers.json on this startup.
    pub carrier_cargo_seeded: std::collections::HashSet<i64>,
    /// Locally-parsed commodity data from Market.json, as (market_id, commodities).
    /// Set by the watcher whenever Market.json changes; used to show market data
    /// immediately without waiting for the API round-trip.
    pub local_market: Option<(i64, Vec<edcas_common::api::CommodityResponse>)>,
    /// Set by a `Location` that arrived in another system than the one held, or found no
    /// scans of this one: earlier journals may hold the scans. The watcher answers it with
    /// [`backfill_current_system`], which clears it.
    pub needs_backfill: bool,
}

impl JournalData {
    pub fn new() -> Self {
        Self {
            witch_space: false,
            jump_pending: false,
            jump_star_class: String::new(),
            current_system: None,
            bodies: Vec::new(),
            fss_signals: HashMap::new(),
            saa_data: HashMap::new(),
            stations: Vec::new(),
            discovered_signals: Vec::new(),
            materials_raw: Vec::new(),
            materials_manufactured: Vec::new(),
            materials_encoded: Vec::new(),
            cargo: Vec::new(),
            backpack: OnFootInventory::default(),
            shiplocker: OnFootInventory::default(),
            modules: Vec::new(),
            loadout_health: HashMap::new(),
            loadout_engineering: HashMap::new(),
            pilot: PilotData::default(),
            status: StatusData::default(),
            carrier_jump_departure: None,
            session: SessionStats::default(),
            construction_depots: HashMap::new(),
            claimed_systems: HashMap::new(),
            last_docked: None,
            carrier_cargo: HashMap::new(),
            commodity_names: HashMap::new(),
            carrier_fc_materials: HashMap::new(),
            pending_host_body_id: None,
            visited_systems: Vec::new(),
            visited_stations: Vec::new(),
            visited_carriers: Vec::new(),
            fss_body_count: None,
            fss_non_body_count: None,
            fss_all_bodies_found: false,
            nav_beacon_bodies: None,
            body_in_focus: None,
            latest_event_timestamp: String::new(),
            latest_event: String::new(),
            carrier_cargo_skip_before: None,
            carrier_cargo_seeded: std::collections::HashSet::new(),
            local_market: None,
            needs_backfill: false,
        }
    }

    /// Add (positive) or remove (negative) `count` of a material in a specific
    /// Raw/Manufactured/Encoded list, keyed by its internal short name. The
    /// "Materials" event only gives a full snapshot at session start; this
    /// keeps counts live afterward as MaterialCollected/Discarded/Trade fire.
    fn adjust_material_in_category(&mut self, category: &str, name: &str, name_localised: &str, delta: i32) {
        let list = match category.to_lowercase().as_str() {
            "raw" => &mut self.materials_raw,
            "manufactured" => &mut self.materials_manufactured,
            "encoded" => &mut self.materials_encoded,
            _ => return,
        };
        adjust_material_count(list, name, name_localised, delta);
    }

    /// Same as `adjust_material_in_category`, but for events that don't state
    /// which category the material belongs to (EngineerCraft's Ingredients,
    /// Synthesis's Materials, EngineerContribution) — finds whichever of the
    /// three lists already holds it.
    fn adjust_material_any_category(&mut self, name: &str, name_localised: &str, delta: i32) {
        let lower = name.to_lowercase();
        for list in [&mut self.materials_raw, &mut self.materials_manufactured, &mut self.materials_encoded] {
            if list.iter().any(|i| i.name.to_lowercase() == lower) {
                adjust_material_count(list, name, name_localised, delta);
                return;
            }
        }
    }

    /// The body the commander is at, for the planet tint and clouds: `Status.json`'s body
    /// when it names one, else the targeted destination when that is a scanned body of the
    /// current system; empty otherwise. The one place this is decided.
    ///
    /// 2026-10-26: Status.json names the body only from orbital cruise on, so the clouds
    /// switched on only then; the target is already known in supercruise.
    pub fn body_name_here(&self) -> &str {
        let status = &self.status;
        if !status.body_name.is_empty() {
            return &status.body_name;
        }
        let here = self.current_system.as_ref().is_some_and(|s| s.system_address == status.destination_system);
        let a_body = self.bodies.iter().any(|b| b.body_id == status.destination_body && b.body_name == status.destination);
        if here && a_body {
            &status.destination
        } else {
            ""
        }
    }

    /// Marks `body_id` as the body the commander just turned to (see [`Self::body_in_focus`]).
    fn focus_body(&mut self, body_id: i32) {
        let n = self.body_in_focus.map_or(0, |(n, _)| n);
        self.body_in_focus = Some((n + 1, body_id));
    }

    /// Takes a fresh Status.json snapshot. Targeting a body of the current system in the
    /// game focuses it; a station or another system's target doesn't.
    pub fn take_status(&mut self, status: StatusData) {
        let retargeted = (status.destination_system, status.destination_body, &status.destination)
            != (self.status.destination_system, self.status.destination_body, &self.status.destination);
        let here = self.current_system.as_ref().is_some_and(|s| s.system_address == status.destination_system);
        let a_body = self.bodies.iter().any(|b| b.body_id == status.destination_body && b.body_name == status.destination);
        if retargeted && here && a_body {
            self.focus_body(status.destination_body);
        }
        let before = self.body_name_here().to_string();
        self.status = status;
        // Debug only, once per change: which body the tint and clouds follow is what a
        // "wrong planet" report turns on (2026-11-08).
        let after = self.body_name_here();
        if before != after {
            tracing::debug!("Body here changed: '{before}' -> '{after}'");
        }
    }

    pub fn process_line(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }

        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return,
        };

        if let Some(ts) = value.get("timestamp").and_then(|v| v.as_str()) {
            self.latest_event_timestamp = ts.to_string();
        }
        if let Some(event) = value.get("event").and_then(|v| v.as_str()) {
            self.latest_event = event.to_string();
        }
        // Witch space, for the star light's rainbow: between the music stopping after a hyperspace
        // countdown and the arrival.
        match value.get("event").and_then(|e| e.as_str()) {
            Some("StartJump") => {
                self.jump_pending = value["JumpType"].as_str() == Some("Hyperspace");
                self.jump_star_class = value["StarClass"].as_str().unwrap_or("").to_string();
                self.witch_space = false;
            }
            Some("Music") if self.jump_pending && value["MusicTrack"].as_str() == Some("NoTrack") => self.witch_space = true,
            Some("FSDJump" | "CarrierJump" | "Location" | "SupercruiseEntry" | "SupercruiseExit" | "Docked" | "Shutdown") => {
                self.jump_pending = false;
                self.witch_space = false;
            }
            _ => {}
        }

        // Loadout carries per-module health and detailed ship stats not in the typed event system.
        // Extract it before from_json consumes value.
        if value.get("event").and_then(|e| e.as_str()) == Some("Loadout") {
            let ship_internal = value["Ship"].as_str().unwrap_or("");
            self.pilot.ship_type = value["Ship_Localised"].as_str()
                .or_else(|| value["Ship"].as_str())
                .unwrap_or("").to_string();
            self.pilot.ship_pad_size = ship_pad_size(ship_internal);
            self.pilot.ship_name  = value["ShipName"].as_str().unwrap_or("").to_string();
            self.pilot.ship_ident = value["ShipIdent"].as_str().unwrap_or("").to_string();
            self.pilot.hull_health    = value["HullHealth"].as_f64().unwrap_or(0.0) as f32;
            self.pilot.max_jump_range = value["MaxJumpRange"].as_f64().unwrap_or(0.0) as f32;
            self.pilot.unladen_mass   = value["UnladenMass"].as_f64().unwrap_or(0.0) as f32;
            self.pilot.cargo_capacity = value["CargoCapacity"].as_i64().unwrap_or(0) as i32;
            self.pilot.modules_value  = value["ModulesValue"].as_i64().unwrap_or(0);
            self.pilot.rebuy          = value["Rebuy"].as_i64().unwrap_or(0);
            self.pilot.fuel_capacity  = value["FuelCapacity"]["Main"].as_f64().unwrap_or(0.0) as f32;
            self.pilot.reserve_fuel_capacity = value["FuelCapacity"]["Reserve"].as_f64().unwrap_or(0.0) as f32;

            if let Some(mods) = value["Modules"].as_array() {
                self.loadout_health.clear();
                self.loadout_engineering.clear();
                for m in mods {
                    let slot = m["Slot"].as_str().unwrap_or("").to_string();
                    if let Some(h) = m["Health"].as_f64() {
                        self.loadout_health.insert(slot.clone(), h as f32);
                    }
                    let eng = parse_engineering(&m["Engineering"]);
                    self.loadout_engineering.insert(slot, eng);
                }
                for i in 0..self.modules.len() {
                    let slot = self.modules[i].slot.clone();
                    self.modules[i].health = self.loadout_health.get(&slot).copied();
                    self.modules[i].engineering = self.loadout_engineering.get(&slot).cloned().flatten();
                }
            }
        }

        match JournalEvent::from_json(value) {
            Some(JournalEvent::FsdJump(e)) => {
                debug!("FSDJump to {}", e.star_system);
                self.pending_host_body_id = None;
                self.session.jumps += 1;
                self.session.systems_visited += 1;
                self.session.light_years += e.jump_dist.unwrap_or(0.0) as f64;
                let sys = system_from_fsdjump(&e);
                let addr = sys.system_address;
                self.visited_systems.retain(|s| s.system_address != addr);
                self.visited_systems.insert(0, sys.clone());
                self.visited_systems.truncate(MAX_VISITED_HISTORY);
                self.current_system = Some(sys);
                self.clear_scan_data();
                self.fss_body_count = None;
                self.fss_non_body_count = None;
                self.fss_all_bodies_found = false;
                self.nav_beacon_bodies = None;
            }
            Some(JournalEvent::Location(e)) => {
                debug!("Location: {}", e.star_system);
                let sys = system_from_location(&e);
                let addr = sys.system_address;
                // Every game start writes a Location, also when the commander never left
                // the system. 2026-10-26: after a game restart (EDCAS kept running) this
                // wiped the scans of Hegoea CX-B d13-3, so only a system change clears.
                let same_system = self.current_system.as_ref().map(|s| s.system_address) == Some(addr);
                self.visited_systems.retain(|s| s.system_address != addr);
                self.visited_systems.insert(0, sys.clone());
                self.visited_systems.truncate(MAX_VISITED_HISTORY);
                self.current_system = Some(sys);
                if !same_system {
                    self.clear_scan_data();
                    self.fss_body_count = None;
                    self.fss_non_body_count = None;
                    self.fss_all_bodies_found = false;
                    self.nav_beacon_bodies = None;
                }
                // Another system, or nothing held of this one: the scans are in earlier journals.
                self.needs_backfill = !same_system || self.bodies.is_empty();
                // When the game starts while docked, Location carries the station data.
                if e.docked {
                    if let (Some(name), Some(market_id)) = (e.station_name, e.market_id) {
                        let station_type = e.station_type.unwrap_or_default();
                        self.last_docked = Some((market_id, name.clone(), e.star_system.clone(), e.system_address));
                        let raw_economy = e.station_economy.unwrap_or_default();
                        let raw_government = e.station_government.unwrap_or_default();
                        let secondary_economies = e.station_economies.as_ref()
                            .map(|v| v.iter().skip(1)
                                .map(|se| (localised_or_clean("", &se.name), se.proportion))
                                .collect())
                            .unwrap_or_default();
                        // Location body_id is the body the station sits on (when BodyType != "Star").
                        let host_body_id = if e.body_type != "Star" && e.body_id != 0 {
                            Some(e.body_id)
                        } else {
                            None
                        };
                        let station = StationData {
                            name: name.clone(),
                            station_type: station_type.clone(),
                            system_name: e.star_system.clone(),
                            dist_from_star_ls: e.dist_from_star_ls.unwrap_or(0.0),
                            services: e.station_services.unwrap_or_default(),
                            economy: localised_or_clean(&e.station_economy_localised, &raw_economy),
                            secondary_economies,
                            faction: e.station_faction.as_ref().map(|f| f.name.clone()).unwrap_or_default(),
                            government: localised_or_clean(&e.station_government_localised, &raw_government),
                            allegiance: e.station_allegiance.unwrap_or_default(),
                            landing_pads: e.landing_pads.as_ref().map(|lp| (lp.small, lp.medium, lp.large)),
                            market_id,
                            host_body_id,
                            commodities: Vec::new(),
                        };
                        if !self.stations.iter().any(|s| s.market_id == market_id) {
                            self.stations.push(station.clone());
                        }
                        if station_type == "FleetCarrier" {
                            self.visited_carriers.retain(|s| s.market_id != market_id);
                            self.visited_carriers.insert(0, station);
                        } else {
                            let prior_commodities = self.visited_stations.iter()
                                .find(|s| s.market_id == market_id)
                                .map(|s| s.commodities.clone())
                                .unwrap_or_default();
                            self.visited_stations.retain(|s| s.market_id != market_id);
                            let mut station = station;
                            if station.commodities.is_empty() {
                                station.commodities = prior_commodities;
                            }
                            self.visited_stations.insert(0, station);
                            self.visited_stations.truncate(MAX_VISITED_HISTORY);
                        }
                    }
                }
            }
            Some(JournalEvent::CarrierJump(e)) => {
                debug!("CarrierJump to {}", e.star_system);
                // The jump has completed; clear any pending arrival timer.
                self.carrier_jump_departure = None;
                let sys = system_from_carrier_jump(&e);
                let addr = sys.system_address;
                self.visited_systems.retain(|s| s.system_address != addr);
                self.visited_systems.insert(0, sys.clone());
                self.visited_systems.truncate(MAX_VISITED_HISTORY);
                self.current_system = Some(sys);
                self.clear_scan_data();
                self.fss_body_count = None;
                self.fss_non_body_count = None;
                self.fss_all_bodies_found = false;
                self.nav_beacon_bodies = None;
            }
            Some(JournalEvent::Scan(e)) => {
                debug!("Scan: {}", e.body_name);
                // Count each body once per system visit (auto + detailed scans share a body_id).
                if !self.bodies.iter().any(|b| b.body_id == e.body_id) {
                    self.session.bodies_scanned += 1;
                }
                upsert_body(&mut self.bodies, body_from_scan(&e, self.pilot.odyssey));
                // Not the AutoScans of the bodies near the arrival point.
                if e.scan_type.as_deref() == Some("Detailed") {
                    self.focus_body(e.body_id);
                }
            }
            Some(JournalEvent::ScanBaryCentre(e)) => {
                let parents = e.parents.as_ref()
                    .map(|pv| pv.iter()
                        .filter_map(|p| p.parent_id().map(|pid| BodyParent {
                            body_id: pid,
                        }))
                        .collect())
                    .unwrap_or_default();
                upsert_body(&mut self.bodies, BodyScan {
                    body_id: e.body_id,
                    body_name: format!("{} Barycentre", e.star_system),
                    planet_class: String::new(),
                    landable: false,
                    scan_type: "AutoScan".into(),
                    distance_from_arrival_ls: e.distance_from_arrival_ls,
                    radius: 0.0,
                    mass_em: 0.0,
                    surface_temperature: 0.0,
                    surface_gravity: 0.0,
                    tidal_lock: false,
                    volcanism: String::new(),
                    atmosphere: String::new(),
                    terraform_state: String::new(),
                    star_type: String::new(),
                    parents,
                    rings: vec![],
                    materials: vec![],
                    value: None,
                    mapped: None,
                    footfall: false,
                    composition: None,
                    discovery: None,
                    exo: None,
                });
            }
            Some(JournalEvent::FssBodySignals(e)) => {
                debug!("FSSBodySignals for body {}", e.body_id);
                let signals = e.signals.iter().map(|s| BodySignal {
                    signal_type: s.signal_type.clone(),
                    signal_type_localised: s.signal_type_localised.clone(),
                    count: s.count,
                }).collect();
                self.fss_signals.insert(e.body_id, signals);
            }
            Some(JournalEvent::SaaSignalsFound(e)) => {
                debug!("SAASignalsFound for body {}", e.body_id);
                let signals = e.signals.iter().map(|s| BodySignal {
                    signal_type: s.signal_type.clone(),
                    signal_type_localised: s.signal_type_localised.clone(),
                    count: s.count,
                }).collect();
                let genuses = e.genuses.as_ref()
                    .map(|gv| gv.iter()
                        .map(|g| GenusSignal {
                            symbol: g.genus.clone(),
                            name: g.genus_localised.clone().unwrap_or_else(|| g.genus.trim_end_matches(';').replace('_', " ")),
                        })
                        .collect())
                    .unwrap_or_default();
                self.saa_data.insert(e.body_id, SaaBodyData { signals, genuses });
            }
            Some(JournalEvent::FssSignalDiscovered(e)) => {
                debug!("FSSSignalDiscovered: {}", e.signal_name);
                self.discovered_signals.push(DiscoveredSignal::from_event(&e));
            }
            Some(JournalEvent::FssDiscoveryScan(e)) => {
                debug!("FSSDiscoveryScan: {} bodies", e.body_count);
                self.fss_body_count = Some(e.body_count);
                self.fss_non_body_count = Some(e.non_body_count);
                self.fss_all_bodies_found = false;
            }
            Some(JournalEvent::FssAllBodiesFound(_)) => {
                debug!("FSSAllBodiesFound");
                self.fss_all_bodies_found = true;
            }
            Some(JournalEvent::NavBeaconScan(e)) => {
                debug!("NavBeaconScan: {} bodies", e.num_bodies);
                self.nav_beacon_bodies = Some(e.num_bodies);
            }
            Some(JournalEvent::ScanOrganic(e)) => {
                // A body's organics are followed across journals by crate::body_finds.
                debug!("ScanOrganic: {} {}", e.genus, e.scan_type);
                self.session.bio_samples += 1;
                // "Analyse" is the third and final sample — the species is now complete and sellable.
                if e.scan_type.eq_ignore_ascii_case("Analyse") {
                    self.session.bio_species_complete += 1;
                }
            }
            None => {
                // Handle events not in the typed enum
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    match v.get("event").and_then(|e| e.as_str()) {
                        Some("Materials") => {
                            self.materials_raw = parse_inventory_array(&v["Raw"]);
                            self.materials_manufactured = parse_inventory_array(&v["Manufactured"]);
                            self.materials_encoded = parse_inventory_array(&v["Encoded"]);
                            debug!(
                                "Materials: {} raw, {} manufactured, {} encoded",
                                self.materials_raw.len(),
                                self.materials_manufactured.len(),
                                self.materials_encoded.len()
                            );
                        }
                        // The events below keep materials_raw/manufactured/encoded live for
                        // the rest of the session — "Materials" above is only a snapshot at
                        // session start, so without these the have-counts shown in the Todo
                        // and Engineers tabs go stale the moment you collect or spend anything.
                        Some("MaterialCollected") => {
                            let category = v["Category"].as_str().unwrap_or("");
                            let name = v["Name"].as_str().unwrap_or("");
                            let name_localised = v["Name_Localised"].as_str().unwrap_or("");
                            let count = v["Count"].as_i64().unwrap_or(0) as i32;
                            if !name.is_empty() {
                                self.adjust_material_in_category(category, name, name_localised, count);
                            }
                        }
                        Some("MaterialDiscarded") => {
                            let category = v["Category"].as_str().unwrap_or("");
                            let name = v["Name"].as_str().unwrap_or("");
                            let name_localised = v["Name_Localised"].as_str().unwrap_or("");
                            let count = v["Count"].as_i64().unwrap_or(0) as i32;
                            if !name.is_empty() {
                                self.adjust_material_in_category(category, name, name_localised, -count);
                            }
                        }
                        Some("MaterialTrade") => {
                            let paid = &v["Paid"];
                            let paid_name = paid["Material"].as_str().unwrap_or("");
                            if !paid_name.is_empty() {
                                self.adjust_material_in_category(
                                    paid["Category"].as_str().unwrap_or(""),
                                    paid_name,
                                    paid["Material_Localised"].as_str().unwrap_or(""),
                                    -(paid["Quantity"].as_i64().unwrap_or(0) as i32),
                                );
                            }
                            let received = &v["Received"];
                            let received_name = received["Material"].as_str().unwrap_or("");
                            if !received_name.is_empty() {
                                self.adjust_material_in_category(
                                    received["Category"].as_str().unwrap_or(""),
                                    received_name,
                                    received["Material_Localised"].as_str().unwrap_or(""),
                                    received["Quantity"].as_i64().unwrap_or(0) as i32,
                                );
                            }
                        }
                        Some("EngineerCraft") => {
                            if let Some(ingredients) = v["Ingredients"].as_array() {
                                for ing in ingredients {
                                    let name = ing["Name"].as_str().unwrap_or("");
                                    if !name.is_empty() {
                                        self.adjust_material_any_category(
                                            name,
                                            ing["Name_Localised"].as_str().unwrap_or(""),
                                            -(ing["Count"].as_i64().unwrap_or(0) as i32),
                                        );
                                    }
                                }
                            }
                        }
                        Some("Synthesis") => {
                            if let Some(materials) = v["Materials"].as_array() {
                                for mat in materials {
                                    let name = mat["Name"].as_str().unwrap_or("");
                                    if !name.is_empty() {
                                        self.adjust_material_any_category(
                                            name,
                                            mat["Name_Localised"].as_str().unwrap_or(""),
                                            -(mat["Count"].as_i64().unwrap_or(0) as i32),
                                        );
                                    }
                                }
                            }
                        }
                        Some("EngineerContribution") if v["Type"].as_str() == Some("Materials") => {
                            let name = v["Material"].as_str().unwrap_or("");
                            if !name.is_empty() {
                                self.adjust_material_any_category(
                                    name,
                                    v["Material_Localised"].as_str().unwrap_or(""),
                                    -(v["Quantity"].as_i64().unwrap_or(0) as i32),
                                );
                            }
                        }
                        Some("Commander") => {
                            self.pilot.name = v["Name"].as_str().unwrap_or("").to_string();
                            debug!("Commander: {}", self.pilot.name);
                        }
                        Some("LoadGame") => {
                            if let Some(name) = v["Commander"].as_str() {
                                if !name.is_empty() { self.pilot.name = name.to_string(); }
                            }
                            self.pilot.credits = v["Credits"].as_i64().unwrap_or(0);
                            let ship_internal = v["Ship"].as_str().unwrap_or("");
                            self.pilot.ship_type = v["Ship_Localised"].as_str()
                                .or_else(|| v["Ship"].as_str())
                                .unwrap_or("").to_string();
                            self.pilot.ship_pad_size = ship_pad_size(ship_internal);
                            self.pilot.ship_name = v["ShipName"].as_str().unwrap_or("").to_string();
                            self.pilot.ship_ident = v["ShipIdent"].as_str().unwrap_or("").to_string();
                            self.pilot.fuel_level = v["FuelLevel"].as_f64().unwrap_or(0.0) as f32;
                            self.pilot.game_mode = v["GameMode"].as_str().unwrap_or("").to_string();
                            self.pilot.horizons = v["Horizons"].as_bool().unwrap_or(false);
                            self.pilot.odyssey = v["Odyssey"].as_bool().unwrap_or(false);
                            debug!(
                                "LoadGame: commander={}, credits={}, ship={}",
                                self.pilot.name, self.pilot.credits, self.pilot.ship_type
                            );
                        }
                        Some("Rank") => {
                            self.pilot.rank_combat = v["Combat"].as_u64().unwrap_or(0) as u8;
                            self.pilot.rank_trade = v["Trade"].as_u64().unwrap_or(0) as u8;
                            self.pilot.rank_explore = v["Explore"].as_u64().unwrap_or(0) as u8;
                            self.pilot.rank_soldier = v["Soldier"].as_u64().unwrap_or(0) as u8;
                            self.pilot.rank_exobiologist = v["Exobiologist"].as_u64().unwrap_or(0) as u8;
                            self.pilot.rank_empire = v["Empire"].as_u64().unwrap_or(0) as u8;
                            self.pilot.rank_federation = v["Federation"].as_u64().unwrap_or(0) as u8;
                            self.pilot.rank_cqc = v["CQC"].as_u64().unwrap_or(0) as u8;
                        }
                        Some("Progress") => {
                            self.pilot.progress_combat = v["Combat"].as_u64().unwrap_or(0) as u8;
                            self.pilot.progress_trade = v["Trade"].as_u64().unwrap_or(0) as u8;
                            self.pilot.progress_explore = v["Explore"].as_u64().unwrap_or(0) as u8;
                            self.pilot.progress_soldier = v["Soldier"].as_u64().unwrap_or(0) as u8;
                            self.pilot.progress_exobiologist = v["Exobiologist"].as_u64().unwrap_or(0) as u8;
                            self.pilot.progress_empire = v["Empire"].as_u64().unwrap_or(0) as u8;
                            self.pilot.progress_federation = v["Federation"].as_u64().unwrap_or(0) as u8;
                            self.pilot.progress_cqc = v["CQC"].as_u64().unwrap_or(0) as u8;
                        }
                        Some("Reputation") => {
                            self.pilot.reputation_empire = v["Empire"].as_f64().unwrap_or(0.0) as f32;
                            self.pilot.reputation_federation = v["Federation"].as_f64().unwrap_or(0.0) as f32;
                            self.pilot.reputation_alliance = v["Alliance"].as_f64().unwrap_or(0.0) as f32;
                        }
                        Some("Powerplay") => {
                            self.pilot.power = v["Power"].as_str().unwrap_or("").to_string();
                            self.pilot.power_merits = v["Merits"].as_i64().unwrap_or(0);
                            debug!(
                                "Powerplay: {} ({} merits)",
                                self.pilot.power, self.pilot.power_merits
                            );
                        }
                        Some("PowerplayMerits") => {
                            // Incremental merit award between full `Powerplay` snapshots — keep
                            // the running total current and note the gain.
                            if let Some(power) = v["Power"].as_str().filter(|s| !s.is_empty()) {
                                self.pilot.power = power.to_string();
                            }
                            let total = v["TotalMerits"].as_i64().unwrap_or(self.pilot.power_merits);
                            self.pilot.power_merits = total;
                            debug!(
                                "PowerplayMerits: +{} merits ({} total) for {}",
                                v["MeritsGained"].as_i64().unwrap_or(0),
                                total,
                                self.pilot.power
                            );
                        }
                        Some("Undocked") => {
                            // Left the station: drop the dock-scoped state so a later
                            // CargoTransfer isn't mis-attributed to the old carrier and a
                            // stale market snapshot doesn't resurface for other stations.
                            self.last_docked = None;
                            self.local_market = None;
                        }
                        Some("CargoTransfer") => {
                            // Track cargo moved to/from the carrier we're currently docked at.
                            if let Some((market_id, _, _, _)) = self.last_docked {
                                let is_carrier = self.visited_carriers.iter().any(|c| c.market_id == market_id);
                                if is_carrier {
                                    // Skip events already captured in the snapshot to avoid
                                    // double-counting when EDCAS restarts mid-session.
                                    let skip = self.carrier_cargo_skip_before.as_deref()
                                        .map(|skip_ts| {
                                            self.carrier_cargo_seeded.contains(&market_id)
                                                && v["timestamp"].as_str()
                                                    .map_or(false, |ts| ts <= skip_ts)
                                        })
                                        .unwrap_or(false);
                                    if !skip {
                                        if let Some(transfers) = v["Transfers"].as_array() {
                                            let cargo = self.carrier_cargo.entry(market_id).or_default();
                                            for t in transfers {
                                                let name = t["Type"].as_str().unwrap_or("").to_lowercase();
                                                if name.is_empty() { continue; }
                                                if let Some(loc) = t["Type_Localised"].as_str().filter(|s| !s.is_empty()) {
                                                    self.commodity_names.entry(name.clone()).or_insert_with(|| loc.to_string());
                                                }
                                                let count = t["Count"].as_i64().unwrap_or(0) as i32;
                                                match t["Direction"].as_str().unwrap_or("") {
                                                    "tocarrier" => *cargo.entry(name).or_insert(0) += count,
                                                    "toship" => {
                                                        let e = cargo.entry(name).or_insert(0);
                                                        *e = (*e - count).max(0);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Some("ColonisationSystemClaim") => {
                            if let (Some(addr), Some(name)) = (
                                v["SystemAddress"].as_i64(),
                                v["StarSystem"].as_str(),
                            ) {
                                self.claimed_systems.insert(addr, name.to_string());
                                debug!("ColonisationSystemClaim: {}", name);
                            }
                        }
                        Some("SuitLoadout") => {
                            let raw_name = v["SuitName"].as_str().unwrap_or("");
                            let (suit_type, grade) = parse_suit_name(raw_name);
                            let mods = v["SuitMods"].as_array()
                                .map(|arr| arr.iter()
                                    .filter_map(|m| m.as_str())
                                    .map(format_suit_mod)
                                    .collect())
                                .unwrap_or_default();
                            let weapons = v["Modules"].as_array()
                                .map(|arr| arr.iter().map(|m| SuitWeapon {
                                    slot: m["SlotName"].as_str().unwrap_or("").to_string(),
                                    name: m["ModuleName_Localised"].as_str()
                                        .or_else(|| m["ModuleName"].as_str())
                                        .unwrap_or("").to_string(),
                                    class: m["Class"].as_u64().unwrap_or(0) as u8,
                                    mods: m["WeaponMods"].as_array()
                                        .map(|wm| wm.iter()
                                            .filter_map(|w| w.as_str())
                                            .map(format_suit_mod)
                                            .collect())
                                        .unwrap_or_default(),
                                }).collect())
                                .unwrap_or_default();
                            debug!(
                                "SuitLoadout: {} (grade {}), loadout '{}'",
                                suit_type,
                                grade,
                                v["LoadoutName"].as_str().unwrap_or("")
                            );
                            self.pilot.suit = Some(SuitData {
                                suit_type,
                                grade,
                                loadout_name: v["LoadoutName"].as_str().unwrap_or("").to_string(),
                                mods,
                                weapons,
                            });
                        }
                        Some("ApproachBody") => {
                            if let Some(id) = v["BodyID"].as_i64() {
                                self.focus_body(id as i32);
                            }
                        }
                        Some("SAAScanComplete") => {
                            // Detailed Surface Scan (mapping) of a body completed.
                            self.session.bodies_mapped += 1;
                            mark_mapped(&mut self.bodies, &v);
                        }
                        Some("Disembark") => mark_footfall(&mut self.bodies, &v),
                        Some("CarrierJumpRequest") => {
                            // Schedule the arrival time so an alert can fire when the jump completes.
                            self.carrier_jump_departure = v["DepartureTime"].as_str()
                                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                                .map(|d| d.with_timezone(&chrono::Utc));
                        }
                        Some("CarrierJumpCancelled") => {
                            self.carrier_jump_departure = None;
                        }
                        Some("SellExplorationData") | Some("MultiSellExplorationData") => {
                            let earned = v["BaseValue"].as_i64().unwrap_or(0)
                                + v["Bonus"].as_i64().unwrap_or(0);
                            // TotalEarnings (when present) already includes the bonus.
                            let earned = v["TotalEarnings"].as_i64().unwrap_or(earned);
                            self.session.exploration_earned += earned;
                        }
                        Some("SellOrganicData") => {
                            // Sum the per-species payouts (Value + Bonus) from the sold list.
                            let total: i64 = v["BioData"].as_array().map(|arr| {
                                arr.iter().map(|d| {
                                    d["Value"].as_i64().unwrap_or(0) + d["Bonus"].as_i64().unwrap_or(0)
                                }).sum()
                            }).unwrap_or(0);
                            self.session.exobio_earned += total;
                        }
                        _ => {}
                    }
                }
            }
            Some(JournalEvent::SupercruiseExit(e)) => {
                // Track the body approached; used as host_body_id for the next Docked event.
                if e.body_type != "Station" && e.body_id != 0 {
                    self.pending_host_body_id = Some(e.body_id);
                } else {
                    self.pending_host_body_id = None;
                }
                if e.body_type != "Station" {
                    self.focus_body(e.body_id);
                }
            }
            Some(JournalEvent::Docked(e)) => {
                debug!("Docked at {} (market_id={})", e.station_name, e.market_id);
                self.session.dockings += 1;
                self.last_docked = Some((
                    e.market_id,
                    e.station_name.clone(),
                    e.star_system.clone(),
                    e.system_address,
                ));
                let secondary_economies = e.station_economies.as_ref()
                    .map(|v| v.iter().skip(1)
                        .map(|se| (localised_or_clean("", &se.name), se.proportion))
                        .collect())
                    .unwrap_or_default();
                let host_body_id = self.pending_host_body_id.take();
                let station = StationData {
                    name: e.station_name.clone(),
                    station_type: e.station_type.clone(),
                    system_name: e.star_system.clone(),
                    dist_from_star_ls: e.dist_from_star_ls.unwrap_or(0.0),
                    services: e.station_services.clone().unwrap_or_default(),
                    economy: localised_or_clean(&e.station_economy_localised, &e.station_economy),
                    secondary_economies,
                    faction: e.station_faction.as_ref().map(|f| f.name.clone()).unwrap_or_default(),
                    government: localised_or_clean(&e.station_government_localised, &e.station_government),
                    allegiance: e.station_allegiance.clone(),
                    landing_pads: e.landing_pads.as_ref().map(|lp| (lp.small, lp.medium, lp.large)),
                    market_id: e.market_id,
                    host_body_id,
                    commodities: Vec::new(),
                };
                if !self.stations.iter().any(|s| s.market_id == station.market_id) {
                    self.stations.push(station.clone());
                }
                if e.station_type == "FleetCarrier" {
                    self.visited_carriers.retain(|s| s.market_id != station.market_id);
                    self.visited_carriers.insert(0, station);
                } else {
                    // Preserve commodities from a prior visit so they survive re-dock
                    // until Market.json is re-parsed (which may take up to 500 ms).
                    let prior_commodities = self.visited_stations.iter()
                        .find(|s| s.market_id == station.market_id)
                        .map(|s| s.commodities.clone())
                        .unwrap_or_default();
                    self.visited_stations.retain(|s| s.market_id != station.market_id);
                    let mut station = station;
                    if station.commodities.is_empty() {
                        station.commodities = prior_commodities;
                    }
                    self.visited_stations.insert(0, station);
                    self.visited_stations.truncate(MAX_VISITED_HISTORY);
                }
            }
            Some(JournalEvent::ColonisationConstructionDepot(e)) => {
                debug!("ColonisationConstructionDepot for market_id={}", e.market_id);
                let (station_name, system_address, system_name) = self
                    .last_docked
                    .as_ref()
                    .filter(|(mid, _, _, _)| *mid == e.market_id)
                    .map(|(_, name, sys, addr)| (name.clone(), *addr, sys.clone()))
                    .unwrap_or_else(|| (format!("Depot {}", e.market_id), e.system_address, String::new()));
                let submission = ConstructionDepotSubmission {
                    market_id: e.market_id,
                    system_address,
                    station_name,
                    progress: e.construction_progress,
                    construction_complete: e.construction_complete,
                    construction_failed: e.construction_failed,
                    resources: e.resources.iter().map(|r| ConstructionResourceSubmission {
                        name: r.name.clone(),
                        display_name: r.display_name().to_string(),
                        required_amount: r.required_amount,
                        provided_amount: r.provided_amount,
                        payment: r.payment,
                    }).collect(),
                };
                self.construction_depots.insert(e.market_id, ConstructionDepotData { submission, system_name });
            }
            _ => {}
        }
    }

    fn clear_scan_data(&mut self) {
        self.bodies.clear();
        self.fss_signals.clear();
        self.saa_data.clear();
        self.stations.clear();
        self.discovered_signals.clear();
    }

    pub fn clear(&mut self) {
        // The current system and its scans stay: a game restart in the same system must
        // not lose them (2026-10-26, Hegoea CX-B d13-3). The new journal's Location
        // clears them itself when it is in another system.
        self.carrier_cargo.clear();
        self.commodity_names.clear();
        self.carrier_cargo_seeded.clear();
        self.carrier_cargo_skip_before = None;
        // Dock-scoped state doesn't carry across a new game session.
        self.last_docked = None;
        self.local_market = None;
        // A new journal file means a fresh game session — reset the session counters.
        self.session = SessionStats::default();
        // carrier_fc_materials is file-based, not event-based — cleared by the watcher when FCMaterials.json changes
    }
}

/// Map the internal `Ship` field from the journal Loadout event to the minimum
/// landing pad size the ship requires: 'L', 'M', or 'S'.
fn ship_pad_size(internal_name: &str) -> char {
    match internal_name.to_lowercase().as_str() {
        // Large-pad-only ships
        "anaconda" | "corvette" | "cutter" | "belugaliner" | "orca"
        | "type7" | "type7_multipurpose" | "type9" | "type9_military"
        | "type10" | "panthermkii" => 'L',
        // Small-pad ships
        "sidewinder" | "hauler" | "eagle" | "eagle_mkii" | "adder"
        | "viper" | "viper_mkiii" | "viper_mkiv"
        | "imperialeargle" | "imperial_eagle" => 'S',
        // Everything else defaults to medium (M or L pad accepted)
        _ => 'M',
    }
}

fn faction_info_from_journal(
    f: &edcas_common::journal::types::Faction,
    conflicts: Option<&Vec<Conflict>>,
) -> FactionInfo {
    let conflict = conflicts.and_then(|cs| {
        cs.iter()
            .find(|c| {
                c.status.to_lowercase() != "completed"
                    && c.faction1.won_days < 4
                    && c.faction2.won_days < 4
                    && (c.faction1.name == f.name || c.faction2.name == f.name)
            })
            .map(|c| {
                let (ours, theirs) = if c.faction1.name == f.name {
                    (&c.faction1, &c.faction2)
                } else {
                    (&c.faction2, &c.faction1)
                };
                ConflictData {
                    war_type: c.war_type.clone(),
                    status: c.status.clone(),
                    opponent: theirs.name.clone(),
                    our_won_days: ours.won_days,
                    opponent_won_days: theirs.won_days,
                }
            })
    });
    FactionInfo {
        name: f.name.clone(),
        influence: f.influence,
        active_states: f
            .active_states
            .as_ref()
            .map(|s| s.iter().map(|x| x.state.clone()).collect())
            .unwrap_or_default(),
        pending_states: f
            .pending_states
            .as_ref()
            .map(|s| s.iter().map(|x| x.state.clone()).collect())
            .unwrap_or_default(),
        recovering_states: f
            .recovering_states
            .as_ref()
            .map(|s| s.iter().map(|x| x.state.clone()).collect())
            .unwrap_or_default(),
        conflict,
    }
}

fn star_pos(v: &[f32]) -> Option<[f32; 3]> {
    match v {
        [x, y, z] => Some([*x, *y, *z]),
        _ => None,
    }
}

fn system_from_fsdjump(e: &FsdJump) -> SystemData {
    SystemData {
        name: e.star_system.clone(),
        system_address: e.system_address,
        economy: e.system_economy.clone(),
        second_economy: e.system_second_economy.clone(),
        government: e.system_government.clone(),
        allegiance: e.system_allegiance.clone(),
        security: e.system_security.clone(),
        population: e.population,
        factions: e
            .factions
            .as_ref()
            .map(|f| f.iter().map(|faction| faction_info_from_journal(faction, e.conflicts.as_ref())).collect())
            .unwrap_or_default(),
        system_faction: e
            .system_faction
            .as_ref()
            .map(|f| f.name.clone())
            .unwrap_or_default(),
        controlling_power: e.controlling_power.clone(),
        powers: e.powers.clone().unwrap_or_default(),
        star_pos: star_pos(&e.star_pos),
    }
}

fn system_from_location(e: &Location) -> SystemData {
    SystemData {
        name: e.star_system.clone(),
        system_address: e.system_address,
        economy: e.system_economy.clone(),
        second_economy: e.system_second_economy.clone(),
        government: e.system_government.clone(),
        allegiance: e.system_allegiance.clone(),
        security: e.system_security.clone(),
        population: e.population,
        factions: e
            .factions
            .as_ref()
            .map(|f| f.iter().map(|faction| faction_info_from_journal(faction, e.conflicts.as_ref())).collect())
            .unwrap_or_default(),
        system_faction: e
            .system_faction
            .as_ref()
            .map(|f| f.name.clone())
            .unwrap_or_default(),
        controlling_power: e.controlling_power.clone(),
        powers: e.powers.clone().unwrap_or_default(),
        star_pos: star_pos(&e.star_pos),
    }
}

fn system_from_carrier_jump(e: &CarrierJump) -> SystemData {
    SystemData {
        name: e.star_system.clone(),
        system_address: e.system_address,
        economy: e.system_economy.clone(),
        second_economy: e.system_second_economy.clone(),
        government: e.system_government.clone(),
        allegiance: e.system_allegiance.clone(),
        security: e.system_security.clone(),
        population: e.population,
        factions: e
            .factions
            .as_ref()
            .map(|f| f.iter().map(|faction| faction_info_from_journal(faction, e.conflicts.as_ref())).collect())
            .unwrap_or_default(),
        system_faction: e
            .system_faction
            .as_ref()
            .map(|f| f.name.clone())
            .unwrap_or_default(),
        controlling_power: e.controlling_power.clone(),
        powers: e.powers.clone().unwrap_or_default(),
        star_pos: star_pos(&e.star_pos),
    }
}

/// Adds a scanned body, or replaces an earlier scan of it: a body is often scanned twice
/// (FSS, then again on finishing the surface map), and the newer scan is the fuller one.
/// A rescan keeps the surface map and footfall made before it, and once the commander has
/// set foot on the body, the footfall the galaxy knew before (a rescan may count theirs).
fn upsert_body(bodies: &mut Vec<BodyScan>, mut body: BodyScan) {
    match bodies.iter_mut().find(|b| b.body_id == body.body_id) {
        Some(old) => {
            body.mapped = body.mapped.or(old.mapped);
            if old.footfall {
                body.footfall = true;
                if let (Some(new), Some(was)) = (body.discovery.as_mut(), old.discovery) {
                    new.footfalled = was.footfalled.or(new.footfalled);
                }
            }
            *old = body;
        }
        None => bodies.push(body),
    }
}

/// Records a `SAAScanComplete` (the commander's surface map) on its body.
fn mark_mapped(bodies: &mut [BodyScan], v: &serde_json::Value) {
    let efficient = v["ProbesUsed"].as_i64().unwrap_or(i64::MAX) <= v["EfficiencyTarget"].as_i64().unwrap_or(0);
    if let Some(body) = bodies.iter_mut().find(|b| Some(b.body_id as i64) == v["BodyID"].as_i64()) {
        body.mapped = Some(efficient);
    }
}

/// Records a `Disembark` on a planet (the commander on foot there) on its body.
fn mark_footfall(bodies: &mut [BodyScan], v: &serde_json::Value) {
    if v["OnPlanet"].as_bool() != Some(true) {
        return;
    }
    if let Some(body) = bodies.iter_mut().find(|b| Some(b.body_id as i64) == v["BodyID"].as_i64()) {
        body.footfall = true;
    }
}

fn body_from_scan(e: &Scan, odyssey: bool) -> BodyScan {
    let parents = e
        .parents
        .as_ref()
        .map(|pv| {
            pv.iter()
                .filter_map(|p| {
                    p.parent_id().map(|pid| BodyParent {
                        body_id: pid,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let rings = e
        .rings
        .as_ref()
        .map(|rv| {
            rv.iter()
                .map(|r| BodyRing {
                    name: r.name.clone(),
                    ring_class: r.ring_class.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    let materials = e
        .materials
        .as_ref()
        .map(|mv| {
            mv.iter()
                .map(|m| BodyMaterial {
                    name: m.name.clone(),
                    percent: m.percent,
                })
                .collect()
        })
        .unwrap_or_default();

    BodyScan {
        body_id: e.body_id,
        body_name: e.body_name.clone(),
        planet_class: e.planet_class.clone().unwrap_or_default(),
        landable: e.landable,
        scan_type: e.scan_type.clone().unwrap_or_default(),
        distance_from_arrival_ls: e.distance_from_arrival_ls.unwrap_or(0.0),
        radius: e.radius.unwrap_or(0.0),
        mass_em: e.mass_em.unwrap_or(0.0),
        surface_temperature: e.surface_temperature.unwrap_or(0.0),
        surface_gravity: e.surface_gravity.unwrap_or(0.0),
        tidal_lock: e.tidal_lock,
        volcanism: e.volcanism.clone().unwrap_or_default(),
        atmosphere: e.atmosphere.clone().unwrap_or_default(),
        terraform_state: e.terraform_state.clone().unwrap_or_default(),
        star_type: e.star_type.clone().unwrap_or_default(),
        parents,
        rings,
        materials,
        value: Some(ScanValue::of(e, odyssey)),
        mapped: None,
        footfall: false,
        composition: e.composition.as_ref().map(|c| BodyComposition {
            ice: c.ice,
            rock: c.rock,
            metal: c.metal,
        }),
        discovery: e.was_discovered.map(|discovered| Discovery {
            discovered,
            mapped: e.was_mapped,
            footfalled: e.was_footfalled,
            reported: false,
        }),        exo: Some(ExoScan {
            atmosphere_type: e.atmosphere_type.clone().unwrap_or_default(),
            so2_percent: e.atmosphere_composition.iter().flatten().find(|g| g.name == "SulphurDioxide").map_or(0.0, |g| g.percent),
            surface_pressure: e.surface_pressure.unwrap_or(0.0),
            composition: e
                .atmosphere_composition
                .iter()
                .flatten()
                .map(|g| AtmosphereGas { name: g.name.clone(), percent: g.percent })
                .collect(),
        }),
    }
}

#[derive(Clone, Debug)]
pub struct BulkUploadProgress {
    pub current_file: usize,
    pub total_files: usize,
    pub lines_done: u64,
    pub done: bool,
    pub error: Option<String>,
}

/// Where the commander is, as far as the planet tint and clouds care. The reader publishes
/// it with every snapshot, so a consumer on a thread of its own sees it at once, without
/// waiting for the GUI to drain the snapshot channel (which it does only when it draws).
#[derive(Clone, Default)]
pub struct Place {
    /// The body the commander is at: `Status.json`'s body, else the targeted destination
    /// when that is a scanned body of the system (see [`JournalData::body_name_here`]).
    pub body_name: String,
    /// `Status.json`'s body alone: the game names one only while the commander is close to
    /// it (orbital cruise, landed, on foot). Empty in open space, whatever is targeted: the
    /// star light's reference point, which a marked far-off star must not move
    /// (2026-11-09: the cockpit went red for a red star marked from beside a white one).
    pub near_body_name: String,
    /// In witch space, between systems ([`JournalData::witch_space`]; not the countdown): the
    /// class of the star being jumped to, for its light to fade in at arrival.
    pub jump: Option<String>,
    pub bodies: Vec<BodyScan>,
}

impl Place {
    pub fn of(data: &JournalData) -> Self {
        Place {
            body_name: data.body_name_here().to_string(),
            near_body_name: data.status.body_name.clone(),
            jump: data.witch_space.then(|| data.jump_star_class.clone()),
            bodies: data.bodies.clone(),
        }
    }
}

/// Where a reader publishes its [`Place`].
pub type PlaceSink = std::sync::Arc<std::sync::Mutex<Place>>;

/// Who else gets each snapshot as the reader thread reads it, before the UI sees it: the UI
/// only runs while its window is shown.
#[derive(Clone, Default)]
pub struct Sinks {
    /// Where the commander is, for the ground tint's writer.
    pub place: Option<PlaceSink>,
    /// The alerts, evaluated and spoken without waiting for the UI.
    pub alerts: Option<crate::alert_service::AlertSink>,
}

fn publish(sinks: &Sinks, data: &JournalData) {
    if let Some(sink) = &sinks.place {
        *sink.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Place::of(data);
    }
    if let Some(alerts) = &sinks.alerts {
        alerts.ingest(data);
    }
}

pub struct JournalReader {
    handle: Option<thread::JoinHandle<()>>,
    should_stop: std::sync::Arc<AtomicBool>,
    receiver: mpsc::Receiver<JournalData>,
    sinks: Sinks,
}

impl JournalReader {
    pub fn start(
        journal_dir: PathBuf,
        api_url: Option<String>,
        eddn: Option<crate::eddn::EddnConfig>,
    ) -> Self {
        Self::start_with_sinks(journal_dir, api_url, eddn, Sinks::default())
    }

    /// [`Self::start`], also handing every snapshot to `sinks` on the reader's thread.
    pub fn start_with_sinks(
        journal_dir: PathBuf,
        api_url: Option<String>,
        eddn: Option<crate::eddn::EddnConfig>,
        sinks: Sinks,
    ) -> Self {
        info!("Initializing journal reader for: {}", journal_dir.display());
        let (tx, rx) = mpsc::channel();
        let should_stop = std::sync::Arc::new(AtomicBool::new(false));
        let stop_flag = should_stop.clone();
        let thread_sinks = sinks.clone();

        let eddn_tx_opt = eddn.map(crate::eddn::start_uploader);

        let upload_tx_opt = api_url.filter(|u| !u.is_empty()).map(|url| {
            let (upload_tx, upload_rx) = mpsc::channel::<String>();
            thread::spawn(move || {
                let client = reqwest::blocking::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .unwrap();
                let endpoint = format!("{}/api/v1/journal/event", url);
                for raw_line in upload_rx {
                    if let Ok(message) = serde_json::from_str::<serde_json::Value>(&raw_line) {
                        let wrapper = serde_json::json!({
                            "$schemaRef": "edcas-client-upload/v1",
                            "message": message
                        });
                        let _ = client.post(&endpoint).json(&wrapper).send();
                    }
                }
            });
            upload_tx
        });

        let handle = thread::spawn(move || {
            let mut journal_data = JournalData::new();

            info!("Loading existing journal files from: {}", journal_dir.display());
            load_existing_files(&journal_dir, &mut journal_data);
            load_cargo_file(&journal_dir.join("Cargo.json"), &mut journal_data);
            journal_data.backpack = load_onfoot_file(&journal_dir.join("Backpack.json"));
            journal_data.shiplocker = load_onfoot_file(&journal_dir.join("ShipLocker.json"));
            if let Some((mid, items)) = load_fc_materials_file(&journal_dir.join("FCMaterials.json")) {
                journal_data.carrier_fc_materials.insert(mid, items);
            }
            load_modules_file(&journal_dir.join("ModulesInfo.json"), &mut journal_data);
            if let Some(status) = load_status_file(&journal_dir.join("Status.json")) {
                journal_data.take_status(status);
            }
            info!("Loaded {} bodies from existing files", journal_data.bodies.len());

            // Seed EDDN augmentation state (game version, current system, cmdr) from the
            // current session file without re-uploading those historical events.
            if let Some(ref eddn_tx) = eddn_tx_opt {
                seed_eddn_state(&journal_dir, eddn_tx);
            }

            publish(&thread_sinks, &journal_data);
            let _ = tx.send(journal_data.clone());
            crate::wake::wake();

            watch_latest_file(
                &journal_dir,
                &mut journal_data,
                &tx,
                &thread_sinks,
                &stop_flag,
                &upload_tx_opt,
                &eddn_tx_opt,
            );
        });

        Self {
            handle: Some(handle),
            should_stop,
            receiver: rx,
            sinks,
        }
    }

    pub fn try_recv(&self) -> Option<JournalData> {
        self.receiver.try_recv().ok()
    }

    pub fn stop(&mut self) {
        info!("Stopping journal reader");
        self.should_stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    pub fn restart(
        &mut self,
        journal_dir: PathBuf,
        api_url: Option<String>,
        eddn: Option<crate::eddn::EddnConfig>,
    ) {
        self.stop();
        *self = Self::start_with_sinks(journal_dir, api_url, eddn, std::mem::take(&mut self.sinks));
    }
}

// Shorter initial delay in tests so retry-path tests don't take seconds.
#[cfg(not(test))]
const RETRY_INITIAL_DELAY: Duration = Duration::from_millis(500);
#[cfg(test)]
const RETRY_INITIAL_DELAY: Duration = Duration::from_millis(1);

fn flush_batch(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    batch: &[serde_json::Value],
    send_errors: &mut u64,
) {
    const MAX_RETRIES: u32 = 10;
    let mut delay = RETRY_INITIAL_DELAY;

    for attempt in 0..=MAX_RETRIES {
        let result = client.post(endpoint).json(batch).send();
        match result {
            Err(e) => {
                if attempt == MAX_RETRIES {
                    error!("Batch send failed ({} events) after {} attempts: {e:#}", batch.len(), attempt + 1);
                    *send_errors += 1;
                    return;
                }
                // Network / connect error — retry after backoff
            }
            Ok(r) => {
                let status = r.status().as_u16();
                match status {
                    200 | 202 | 204 => return, // success
                    503 | 504 | 429 => {
                        if attempt == MAX_RETRIES {
                            error!("Batch send failed ({} events) after {} attempts: HTTP {}", batch.len(), attempt + 1, status);
                            eprintln!("Upload error: HTTP {status} after {} retries ({} events in batch)", attempt + 1, batch.len());
                            *send_errors += 1;
                            return;
                        }
                        // Transient server overload — retry after backoff
                    }
                    other => {
                        error!("Batch send failed ({} events): HTTP {}", batch.len(), other);
                        eprintln!("Upload error: HTTP {other} ({} events in batch)", batch.len());
                        *send_errors += 1;
                        return;
                    }
                }
            }
        }
        thread::sleep(delay);
        delay = (delay * 2).min(Duration::from_secs(30));
    }
}

/// Starts a background thread that uploads all journal files (oldest first) to the server.
/// Returns a receiver that yields progress updates.
pub fn start_bulk_upload(
    journal_dir: PathBuf,
    api_url: String,
) -> mpsc::Receiver<BulkUploadProgress> {
    let (progress_tx, progress_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut files = find_all_journal_files(&journal_dir);
        files.reverse(); // oldest first
        let total = files.len();
        if total == 0 {
            let _ = progress_tx.send(BulkUploadProgress {
                current_file: 0,
                total_files: 0,
                lines_done: 0,
                done: true,
                error: Some("No journal files found".into()),
            });
            crate::wake::wake();
            return;
        }

        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = progress_tx.send(BulkUploadProgress {
                    current_file: 0,
                    total_files: total,
                    lines_done: 0,
                    done: true,
                    error: Some(format!("Failed to create HTTP client: {e}")),
                });
                crate::wake::wake();
                return;
            }
        };
        let endpoint = format!("{}/api/v1/journal/events", api_url);
        info!("Starting batched upload to {}", endpoint);
        let mut lines_done: u64 = 0;
        let mut send_errors: u64 = 0;
        const BATCH_SIZE: usize = 50;

        for (i, file) in files.iter().enumerate() {
            let mut batch: Vec<serde_json::Value> = Vec::with_capacity(BATCH_SIZE);

            if let Ok(content) = std::fs::read_to_string(file) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.len() > 65_536 {
                        continue;
                    }
                    if let Ok(message) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        batch.push(serde_json::json!({
                            "$schemaRef": "edcas-client-upload/v1",
                            "message": message
                        }));
                        lines_done += 1;
                        if batch.len() >= BATCH_SIZE {
                            flush_batch(&client, &endpoint, &batch, &mut send_errors);
                            batch.clear();
                        }
                    }
                }
            }
            if !batch.is_empty() {
                flush_batch(&client, &endpoint, &batch, &mut send_errors);
            }

            let _ = progress_tx.send(BulkUploadProgress {
                current_file: i + 1,
                total_files: total,
                lines_done,
                done: i + 1 == total,
                error: if send_errors > 0 {
                    Some(format!("{send_errors} send error(s) — check log for details"))
                } else {
                    None
                },
            });
            crate::wake::wake();
        }
    });
    progress_rx
}

impl Drop for JournalReader {
    fn drop(&mut self) {
        self.should_stop.store(true, Ordering::SeqCst);
    }
}

/// Apply `delta` to the entry named `name` in `list` (adding it if missing and
/// `delta` is positive), dropping the entry if its count reaches zero.
fn adjust_material_count(list: &mut Vec<InventoryItem>, name: &str, name_localised: &str, delta: i32) {
    let lower = name.to_lowercase();
    if let Some(idx) = list.iter().position(|i| i.name.to_lowercase() == lower) {
        list[idx].count = (list[idx].count + delta).max(0);
        if list[idx].count == 0 {
            list.remove(idx);
        }
    } else if delta > 0 {
        list.push(InventoryItem {
            name: name.to_string(),
            localised: if name_localised.is_empty() { name.to_string() } else { name_localised.to_string() },
            count: delta,
        });
    }
}

/// On-foot inventory files (Backpack.json/ShipLocker.json) list one entry per
/// pickup rather than per material — mission-reward materials in particular
/// show up as many separate `Count:1` entries sharing the same `Name`. Merge
/// those into one entry per material so counts (and have-counts derived from
/// them) reflect the true total instead of just the first entry found.
fn parse_inventory_array(arr: &serde_json::Value) -> Vec<InventoryItem> {
    let mut merged: Vec<InventoryItem> = Vec::new();
    if let Some(items) = arr.as_array() {
        for item in items {
            let Some(name) = item.get("Name").and_then(|v| v.as_str()) else { continue };
            let localised = item.get("Name_Localised")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_owned();
            let count = item.get("Count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            if let Some(existing) = merged.iter_mut().find(|i| i.name == name) {
                existing.count += count;
            } else {
                merged.push(InventoryItem { name: name.to_owned(), localised, count });
            }
        }
    }
    merged
}

fn load_cargo_file(path: &Path, data: &mut JournalData) {
    let Ok(content) = std::fs::read_to_string(path) else { return };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) else { return };
    data.cargo = v["Inventory"].as_array()
        .map(|items| items.iter().filter_map(|item| {
            let name = item.get("Name")?.as_str()?.to_owned();
            let localised = item.get("Name_Localised")
                .and_then(|v| v.as_str())
                .unwrap_or(&name)
                .to_owned();
            let count = item.get("Count").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let stolen = item.get("Stolen").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            Some(CargoItem { name, localised, count, stolen })
        }).collect())
        .unwrap_or_default();
}

fn load_modules_file(path: &Path, data: &mut JournalData) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return };
    let Some(arr) = v["Modules"].as_array() else { return };
    let modules: Vec<ShipModule> = arr.iter().map(|m| {
        let slot = m["Slot"].as_str().unwrap_or("").to_string();
        let health      = data.loadout_health.get(&slot).copied();
        let engineering = data.loadout_engineering.get(&slot).cloned().flatten();
        ShipModule {
            slot,
            item:     m["Item"].as_str().unwrap_or("").to_string(),
            power:    m["Power"].as_f64().unwrap_or(0.0) as f32,
            priority: m["Priority"].as_u64().unwrap_or(0) as u8,
            health,
            value: None,
            engineering,
        }
    }).collect();
    data.modules = modules;
}

fn parse_suit_name(raw: &str) -> (String, u8) {
    let grade = raw.find("_class")
        .and_then(|i| raw[i + 6..].parse::<u8>().ok())
        .unwrap_or(1);
    let base = raw.find("_class").map(|i| &raw[..i]).unwrap_or(raw);
    let suit_type = match base {
        "explorationsuit" => "Artemis Suit",
        "utilitysuit"     => "Maverick Suit",
        "tacticalsuit"    => "Dominator Suit",
        other             => other,
    };
    (suit_type.to_string(), grade)
}

fn format_suit_mod(raw: &str) -> String {
    let stripped = raw.strip_prefix("suit_")
        .or_else(|| raw.strip_prefix("wpn_mod_"))
        .unwrap_or(raw);
    stripped.split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_engineering(v: &serde_json::Value) -> Option<EngineeringData> {
    if !v.is_object() {
        return None;
    }
    let raw_exp = v["ExperimentalEffect_Localised"].as_str()
        .filter(|s| !s.is_empty() && !s.starts_with('$'))
        .or_else(|| v["ExperimentalEffect"].as_str().filter(|s| !s.is_empty()))
        .unwrap_or("")
        .to_string();
    let experimental = if raw_exp.starts_with("special_") {
        // e.g. "special_weapon_efficient" -> "Weapon Efficient"
        let body = raw_exp.trim_start_matches("special_").trim_end_matches("_name");
        body.split('_')
            .map(|w| { let mut c = w.chars(); c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default() })
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        raw_exp
    };

    let modifiers = v["Modifiers"].as_array()
        .map(|arr| arr.iter().map(|m| EngineeringModifier {
            label:          camel_to_words(m["Label"].as_str().unwrap_or("")),
            value:          m["Value"].as_f64().unwrap_or(0.0) as f32,
            original_value: m["OriginalValue"].as_f64().unwrap_or(0.0) as f32,
            less_is_good:   m["LessIsGood"].as_i64().unwrap_or(0) != 0,
        }).collect())
        .unwrap_or_default();

    Some(EngineeringData {
        blueprint:    format_blueprint(v["BlueprintName"].as_str().unwrap_or("")),
        level:        v["Level"].as_u64().unwrap_or(0) as u8,
        engineer:     v["Engineer"].as_str().unwrap_or("").to_string(),
        experimental,
        modifiers,
    })
}

fn format_blueprint(raw: &str) -> String {
    let name = raw.find('_').map(|i| &raw[i + 1..]).unwrap_or(raw);
    camel_to_words(name)
}

fn camel_to_words(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.char_indices() {
        if i > 0 && ch.is_uppercase() {
            // insert space before a capital that follows a lowercase letter
            let prev = s[..i].chars().last().unwrap_or(' ');
            if prev.is_lowercase() {
                out.push(' ');
            }
        }
        out.push(ch);
    }
    out
}

fn load_fc_materials_file(path: &Path) -> Option<(i64, Vec<(String, i32)>)> {
    let content = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let market_id = v["MarketID"].as_i64()?;
    let items: Vec<(String, i32)> = v["Items"].as_array()?
        .iter()
        .filter_map(|item| {
            let stock = item["Stock"].as_i64().unwrap_or(0) as i32;
            if stock <= 0 { return None; }
            let name = item["Name_Localised"].as_str()
                .or_else(|| item["Name"].as_str())
                .map(|s| s.trim_start_matches('$').trim_end_matches("_name;").trim_end_matches(';').to_string())
                .unwrap_or_default();
            if name.is_empty() { return None; }
            Some((name, stock))
        })
        .collect();
    Some((market_id, items))
}

fn load_market_file(path: &Path) -> Option<(i64, Vec<edcas_common::api::CommodityResponse>)> {
    let content = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let market_id = v["MarketID"].as_i64()?;
    let items = v["Items"].as_array()?;
    let commodities: Vec<edcas_common::api::CommodityResponse> = items.iter().filter_map(|item| {
        let name = item["Name"].as_str()
            .map(|s| s.trim_start_matches('$').trim_end_matches("_name;").to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| item["Name_Localised"].as_str().filter(|s| !s.is_empty()).map(|s| s.to_string()))
            .unwrap_or_default();
        if name.is_empty() { return None; }
        Some(edcas_common::api::CommodityResponse {
            name,
            mean_price: item["MeanPrice"].as_i64().unwrap_or(0) as i32,
            buy_price:  item["BuyPrice"].as_i64().unwrap_or(0) as i32,
            stock:      item["Stock"].as_i64().unwrap_or(0) as i32,
            sell_price: item["SellPrice"].as_i64().unwrap_or(0) as i32,
            demand:     item["Demand"].as_i64().unwrap_or(0) as i32,
        })
    }).collect();
    if commodities.is_empty() { return None; }
    Some((market_id, commodities))
}

fn load_onfoot_file(path: &Path) -> OnFootInventory {
    let Ok(content) = std::fs::read_to_string(path) else { return OnFootInventory::default() };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) else { return OnFootInventory::default() };
    OnFootInventory {
        items: parse_inventory_array(&v["Items"]),
        components: parse_inventory_array(&v["Components"]),
        consumables: parse_inventory_array(&v["Consumables"]),
        data: parse_inventory_array(&v["Data"]),
    }
}

/// Parses `Status.json` (live ship/on-foot telemetry) into a [`StatusData`] snapshot.
/// Returns `None` if the file is missing or unparseable so the previous snapshot is kept.
fn load_status_file(path: &Path) -> Option<StatusData> {
    let content = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    let pips = v["Pips"].as_array();
    let pip = |i: usize| pips.and_then(|p| p.get(i)).and_then(|x| x.as_u64()).unwrap_or(0) as u8;
    let opt_f32 = |key: &str| v[key].as_f64().map(|n| n as f32);
    Some(StatusData {
        valid: true,
        flags: v["Flags"].as_u64().unwrap_or(0) as u32,
        flags2: v["Flags2"].as_u64().unwrap_or(0) as u32,
        pips_sys: pip(0),
        pips_eng: pip(1),
        pips_wep: pip(2),
        firegroup: v["FireGroup"].as_u64().unwrap_or(0) as u8,
        fuel_main: v["Fuel"]["FuelMain"].as_f64().unwrap_or(0.0) as f32,
        cargo_tons: v["Cargo"].as_f64().unwrap_or(0.0) as f32,
        legal_state: v["LegalState"].as_str().unwrap_or("").to_string(),
        body_name: v["BodyName"].as_str().unwrap_or("").to_string(),
        altitude: v["Altitude"].as_f64(),
        destination: v["Destination"]["Name"].as_str().unwrap_or("").to_string(),
        destination_system: v["Destination"]["System"].as_i64().unwrap_or(0),
        destination_body: v["Destination"]["Body"].as_i64().unwrap_or(0) as i32,
        health: opt_f32("Health"),
        oxygen: opt_f32("Oxygen"),
        temperature_k: opt_f32("Temperature"),
        gravity: opt_f32("Gravity"),
        // Set by the caller, which tracks the fresh-read counter across polls.
        status_seq: 0,
    })
}

fn load_existing_files(
    dir: &Path,
    data: &mut JournalData,
) {
    if !dir.exists() || !dir.is_dir() {
        warn!("Journal directory does not exist: {}", dir.display());
        return;
    }
    let files = find_all_journal_files(dir);
    if files.is_empty() {
        warn!("No journal file found in: {}", dir.display());
        return;
    }

    // Seed carrier cargo from the persisted snapshot; these carriers are skipped
    // during backfill to avoid double-counting events already included in the snapshot.
    let seeded_ids = seed_my_carrier_cargo(data);

    // Backfill carrier cargo from previous sessions before processing the current
    // file so that chronological order is preserved for correct running totals.
    let prev_files: Vec<PathBuf> = files.iter().skip(1).take(15).rev().cloned().collect();
    if !prev_files.is_empty() {
        info!("Backfilling carrier cargo from {} previous journal files", prev_files.len());
        backfill_carrier_cargo(&prev_files, data, &seeded_ids);
    }

    info!("Loading journal file: {}", files[0].display());
    read_file_lines(&files[0], data, None, None);
    // After replaying the journal the skip-window is no longer needed; live events must
    // always be applied unconditionally.
    data.carrier_cargo_skip_before = None;

    backfill_current_system(dir, data);
}

/// Most older journals [`backfill_current_system`] walks back when it can't find the
/// arrival in the current system.
const BACKFILL_MAX_FILES: usize = 100;

/// Whether the journal at `path` holds an arrival in the system: an `FSDJump` or
/// `CarrierJump` into it. A `Location` is no arrival, every game start writes one.
fn journal_arrives_in(path: &Path, system_address: i64) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    BufReader::new(file).lines().map_while(Result::ok).any(|line| {
        (line.contains("\"FSDJump\"") || line.contains("\"CarrierJump\""))
            && serde_json::from_str::<serde_json::Value>(&line).is_ok_and(|v| {
                matches!(v["event"].as_str(), Some("FSDJump" | "CarrierJump"))
                    && v["SystemAddress"].as_i64() == Some(system_address)
            })
    })
}

/// Adds the current system's scans from older journals to what the newest one has.
///
/// A game session starts with `Location`, so the scans of the system it starts in were
/// written in earlier sessions. After a restart while landed at Hegoea EN-F b39-0 B 6
/// (2026-10-14), the new journal held that body's signals alone and the 16 others were
/// lost. What this session logged wins; older scans only fill in what it hasn't.
///
/// Every restart writes another journal, so how far back the scans are isn't a fixed
/// number of files: on 2026-10-26 the commander scanned Hegoea CX-B d13-3 4 a, restarted
/// the game 10 times in that system, and the scan sat 11 files back. The walk goes back
/// to the journal of the arrival in the system (capped at [`BACKFILL_MAX_FILES`] when
/// there is none).
fn backfill_current_system(dir: &Path, data: &mut JournalData) {
    data.needs_backfill = false;
    let Some(system_address) = data.current_system.as_ref().map(|s| s.system_address) else {
        return;
    };
    // Newest first, up to and including the journal of the arrival.
    let mut older: Vec<PathBuf> = Vec::new();
    for file in find_all_journal_files(dir).into_iter().skip(1).take(BACKFILL_MAX_FILES) {
        let arrived = journal_arrives_in(&file, system_address);
        older.push(file);
        if arrived {
            break;
        }
    }
    // Oldest first, so a later visit's scans win among the older ones.
    let mut earlier = JournalData::new();
    for file in older.iter().rev() {
        load_previous_scans_for_system(file, system_address, &mut earlier, None);
    }
    if !earlier.bodies.is_empty() || !earlier.saa_data.is_empty() || !earlier.fss_signals.is_empty() {
        info!(
            "Backfilled {} bodies and {} surface scans of the current system from earlier journals",
            earlier.bodies.len(),
            earlier.saa_data.len()
        );
    }
    for body in earlier.bodies {
        match data.bodies.iter_mut().find(|b| b.body_id == body.body_id) {
            Some(current) => {
                current.mapped = current.mapped.or(body.mapped);
                if body.footfall {
                    // As in upsert_body: the footfall the galaxy knew before the commander's.
                    current.footfall = true;
                    if let (Some(now), Some(was)) = (current.discovery.as_mut(), body.discovery) {
                        now.footfalled = was.footfalled.or(now.footfalled);
                    }
                }
            }
            None => data.bodies.push(body),
        }
    }
    for (id, signals) in earlier.fss_signals {
        data.fss_signals.entry(id).or_insert(signals);
    }
    for (id, saa) in earlier.saa_data {
        data.saa_data.entry(id).or_insert(saa);
    }
    data.fss_body_count = data.fss_body_count.or(earlier.fss_body_count);
    data.fss_non_body_count = data.fss_non_body_count.or(earlier.fss_non_body_count);
    data.fss_all_bodies_found |= earlier.fss_all_bodies_found;
}

fn seed_my_carrier_cargo(data: &mut JournalData) -> std::collections::HashSet<i64> {
    let mc = crate::my_carriers::MyCarriersData::load();
    let seeded: std::collections::HashSet<i64> = mc.carriers.keys().copied().collect();
    // If the snapshot was saved mid-session, skip CargoTransfer events that are already
    // captured in the snapshot (timestamp ≤ snapshot_timestamp) to prevent double-counting
    // when EDCAS is restarted without closing the game.
    if !mc.snapshot_timestamp.is_empty() {
        data.carrier_cargo_skip_before = Some(mc.snapshot_timestamp);
    }
    data.carrier_cargo_seeded = seeded.clone();
    for (market_id, cargo) in mc.carriers {
        data.carrier_cargo.insert(market_id, cargo);
    }
    seeded
}

fn backfill_carrier_cargo(
    files: &[PathBuf],
    data: &mut JournalData,
    skip_ids: &std::collections::HashSet<i64>,
) {
    for path in files {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let reader = BufReader::new(file);
        let mut docked_carrier_market_id: Option<i64> = None;

        for line in reader.lines().flatten() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let v: serde_json::Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match v["event"].as_str().unwrap_or("") {
                "Docked" => {
                    if v["StationType"].as_str() == Some("FleetCarrier") {
                        docked_carrier_market_id = v["MarketID"].as_i64();
                    } else {
                        docked_carrier_market_id = None;
                    }
                }
                "Location" => {
                    if v["Docked"].as_bool() == Some(true)
                        && v["StationType"].as_str() == Some("FleetCarrier")
                    {
                        docked_carrier_market_id = v["MarketID"].as_i64();
                    } else {
                        docked_carrier_market_id = None;
                    }
                }
                "Undocked" => {
                    docked_carrier_market_id = None;
                }
                "CargoTransfer" => {
                    if let Some(market_id) = docked_carrier_market_id {
                        // Skip carriers already seeded from the persisted snapshot
                        if skip_ids.contains(&market_id) {
                            continue;
                        }
                        if let Some(transfers) = v["Transfers"].as_array() {
                            let cargo = data.carrier_cargo.entry(market_id).or_default();
                            for t in transfers {
                                let name = t["Type"].as_str().unwrap_or("").to_lowercase();
                                if name.is_empty() {
                                    continue;
                                }
                                if let Some(loc) =
                                    t["Type_Localised"].as_str().filter(|s| !s.is_empty())
                                {
                                    data.commodity_names
                                        .entry(name.clone())
                                        .or_insert_with(|| loc.to_string());
                                }
                                let count = t["Count"].as_i64().unwrap_or(0) as i32;
                                match t["Direction"].as_str().unwrap_or("") {
                                    "tocarrier" => *cargo.entry(name).or_insert(0) += count,
                                    "toship" => {
                                        let e = cargo.entry(name).or_insert(0);
                                        *e = (*e - count).max(0);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn watch_latest_file(
    dir: &Path,
    data: &mut JournalData,
    tx: &mpsc::Sender<JournalData>,
    sinks: &Sinks,
    stop_flag: &AtomicBool,
    upload_tx: &Option<mpsc::Sender<String>>,
    eddn_tx: &Option<mpsc::Sender<crate::eddn::EddnInput>>,
) {
    // Initialise to the file already loaded by load_existing_files so the first loop
    // iteration does an incremental tail rather than a redundant full re-read.
    let mut last_file: Option<PathBuf> = find_latest_journal_file(dir);
    let mut last_position: u64 = last_file.as_ref()
        .and_then(|f| f.metadata().ok())
        .map(|m| m.len())
        .unwrap_or(0);
    // The latest journal file only changes when a new one is created (game restart /
    // log rollover), which bumps the *directory* mtime — appends to the current file
    // don't. So cache the latest file and only re-scan the dir (read_dir + parse +
    // sort of every .log) when the dir mtime changes, instead of every tick.
    let mut latest_file: Option<PathBuf> = last_file.clone();
    let mut last_dir_mtime: Option<SystemTime> = dir.metadata().and_then(|m| m.modified()).ok();
    // Seed the companion-file mtimes to their current on-disk values (mirroring how
    // `last_position` is seeded to end-of-file) so the first loop iteration does not treat
    // pre-existing files as "changed" and re-upload stale data left over from a previous
    // session. Only files the game writes *while we are running* are uploaded.
    let mtime_of = |name: &str| -> Option<SystemTime> {
        dir.join(name).metadata().and_then(|m| m.modified()).ok()
    };
    let mut last_cargo_mtime = mtime_of("Cargo.json");
    let mut last_backpack_mtime = mtime_of("Backpack.json");
    let mut last_shiplocker_mtime = mtime_of("ShipLocker.json");
    let mut last_fc_materials_mtime = mtime_of("FCMaterials.json");
    let mut last_modules_mtime = mtime_of("ModulesInfo.json");
    let mut last_market_mtime = mtime_of("Market.json");
    let mut last_outfitting_mtime = mtime_of("Outfitting.json");
    let mut last_shipyard_mtime = mtime_of("Shipyard.json");
    let mut shipyard_polls = 0;
    let mut last_navroute_mtime = mtime_of("NavRoute.json");
    let mut last_status_mtime = mtime_of("Status.json");
    let mut status_seq: u64 = 0;

    loop {
        if stop_flag.load(Ordering::SeqCst) {
            return;
        }

        let mut changed = false;

        // ── Journal file ──────────────────────────────────────────
        // Re-scan for a newer file only when the directory mtime changed; otherwise
        // keep tailing the cached latest file (one cheap dir stat vs a full rescan).
        let dir_mtime = dir.metadata().and_then(|m| m.modified()).ok();
        if dir_mtime != last_dir_mtime {
            last_dir_mtime = dir_mtime;
            latest_file = find_latest_journal_file(dir);
        }
        if let Some(active) = latest_file.clone() {
            let file_changed = last_file.as_ref() != Some(&active);
            if file_changed {
                info!("Journal file changed to: {}", active.display());
                last_file = Some(active.clone());
                data.clear();
                // Re-seed persisted carrier cargo so it survives new-session clear.
                // A new journal file always means a fresh game session, so any snapshot
                // timestamp is from before this session — no skip needed.
                seed_my_carrier_cargo(data);
                data.carrier_cargo_skip_before = None;
                read_file_lines(&active, data, upload_tx.as_ref(), eddn_tx.as_ref());
                // A new journal usually holds the Location of a game restart in the system
                // already held, with its scans: nothing to fetch. The walk through the
                // older journals is for a Location in another system or without scans.
                if data.needs_backfill {
                    backfill_current_system(dir, data);
                }
                last_position = active.metadata().map(|m| m.len()).unwrap_or(0);
                changed = true;
            } else if let Ok(metadata) = active.metadata() {
                let file_size = metadata.len();
                if file_size > last_position {
                    if let Ok(mut file) = OpenOptions::new().read(true).open(&active) {
                        let _ = file.seek(SeekFrom::Start(last_position));
                        let reader = BufReader::new(file);
                        for line in reader.lines().flatten() {
                            let trimmed = line.trim().to_owned();
                            if !trimmed.is_empty() {
                                data.process_line(&trimmed);
                                if let Some(ref ed_tx) = eddn_tx {
                                    let _ = ed_tx.send(crate::eddn::EddnInput::Line(trimmed.clone()));
                                }
                                if let Some(ref up_tx) = upload_tx {
                                    let _ = up_tx.send(trimmed);
                                }
                                changed = true;
                            }
                        }
                        last_position = file_size;
                        // The game creates its journal before it writes the Location, so
                        // that line arrives here, not in the read of the new file above.
                        if data.needs_backfill {
                            backfill_current_system(dir, data);
                        }
                    }
                }
            }
        }

        // ── Inventory JSON files ──────────────────────────────────
        let cargo_path = dir.join("Cargo.json");
        if let Ok(mtime) = cargo_path.metadata().and_then(|m| m.modified()) {
            if Some(mtime) != last_cargo_mtime {
                last_cargo_mtime = Some(mtime);
                load_cargo_file(&cargo_path, data);
                changed = true;
            }
        }

        let backpack_path = dir.join("Backpack.json");
        if let Ok(mtime) = backpack_path.metadata().and_then(|m| m.modified()) {
            if Some(mtime) != last_backpack_mtime {
                last_backpack_mtime = Some(mtime);
                data.backpack = load_onfoot_file(&backpack_path);
                changed = true;
            }
        }

        let shiplocker_path = dir.join("ShipLocker.json");
        if let Ok(mtime) = shiplocker_path.metadata().and_then(|m| m.modified()) {
            if Some(mtime) != last_shiplocker_mtime {
                last_shiplocker_mtime = Some(mtime);
                data.shiplocker = load_onfoot_file(&shiplocker_path);
                changed = true;
            }
        }

        let fc_materials_path = dir.join("FCMaterials.json");
        if let Ok(mtime) = fc_materials_path.metadata().and_then(|m| m.modified()) {
            if Some(mtime) != last_fc_materials_mtime {
                last_fc_materials_mtime = Some(mtime);
                if let Some((mid, items)) = load_fc_materials_file(&fc_materials_path) {
                    data.carrier_fc_materials.insert(mid, items);
                }
                changed = true;
            }
        }

        let modules_path = dir.join("ModulesInfo.json");
        if let Ok(mtime) = modules_path.metadata().and_then(|m| m.modified()) {
            if Some(mtime) != last_modules_mtime {
                last_modules_mtime = Some(mtime);
                load_modules_file(&modules_path, data);
                changed = true;
            }
        }

        // ── Station companion files (uploaded to server when an API URL is set) ──
        let market_path = dir.join("Market.json");
        if let Ok(mtime) = market_path.metadata().and_then(|m| m.modified()) {
            if Some(mtime) != last_market_mtime {
                last_market_mtime = Some(mtime);
                if let Some(ref up_tx) = upload_tx {
                    upload_companion_file(&market_path, up_tx);
                }
                if let Some(ref ed_tx) = eddn_tx {
                    send_companion_to_eddn(&market_path, crate::eddn::CompanionKind::Market, ed_tx);
                }
                if let Some((mid, commodities)) = load_market_file(&market_path) {
                    if let Some(station) = data.visited_stations.iter_mut().find(|s| s.market_id == mid) {
                        station.commodities = commodities.clone();
                    }
                    data.local_market = Some((mid, commodities));
                    changed = true;
                }
            }
        }

        let outfitting_path = dir.join("Outfitting.json");
        if let Ok(mtime) = outfitting_path.metadata().and_then(|m| m.modified()) {
            if Some(mtime) != last_outfitting_mtime {
                last_outfitting_mtime = Some(mtime);
                if let Some(ref up_tx) = upload_tx {
                    upload_companion_file(&outfitting_path, up_tx);
                }
                if let Some(ref ed_tx) = eddn_tx {
                    send_companion_to_eddn(
                        &outfitting_path,
                        crate::eddn::CompanionKind::Outfitting,
                        ed_tx,
                    );
                }
            }
        }

        let shipyard_path = dir.join("Shipyard.json");
        if let Ok(mtime) = shipyard_path.metadata().and_then(|m| m.modified()) {
            if Some(mtime) != last_shipyard_mtime {
                let judged = judge_shipyard(dir, &shipyard_path);
                // The `StoredShips` line written with the file may not be in the
                // journal yet: look again on the next polls before giving up.
                if judged == ShipyardFile::Unsure && shipyard_polls < SHIPYARD_POLLS {
                    shipyard_polls += 1;
                } else {
                    last_shipyard_mtime = Some(mtime);
                    shipyard_polls = 0;
                    match judged {
                        ShipyardFile::Stock => {
                            if let Some(ref up_tx) = upload_tx {
                                upload_companion_file(&shipyard_path, up_tx);
                            }
                            if let Some(ref ed_tx) = eddn_tx {
                                send_companion_to_eddn(
                                    &shipyard_path,
                                    crate::eddn::CompanionKind::Shipyard,
                                    ed_tx,
                                );
                            }
                        }
                        ShipyardFile::OwnFleet => {
                            info!("Shipyard.json lists the commander's own ships, not the station's stock — not uploading it")
                        }
                        ShipyardFile::Unsure => {
                            warn!("Shipyard.json came without a StoredShips event to check it against — not uploading it")
                        }
                    }
                }
            }
        }

        // NavRoute.json holds the plotted route; the journal only carries a bare `NavRoute`
        // marker, so the route waypoints can only be shared from this file. EDDN-only — the
        // server already receives the marker event via the journal line stream.
        let navroute_path = dir.join("NavRoute.json");
        if let Ok(mtime) = navroute_path.metadata().and_then(|m| m.modified()) {
            if Some(mtime) != last_navroute_mtime {
                last_navroute_mtime = Some(mtime);
                if let Some(ref ed_tx) = eddn_tx {
                    send_companion_to_eddn(
                        &navroute_path,
                        crate::eddn::CompanionKind::NavRoute,
                        ed_tx,
                    );
                }
            }
        }

        // ── Status.json (live telemetry: fuel, hull, pips, flags) ──
        let status_path = dir.join("Status.json");
        if let Ok(mtime) = status_path.metadata().and_then(|m| m.modified()) {
            if Some(mtime) != last_status_mtime {
                last_status_mtime = Some(mtime);
                if let Some(mut status) = load_status_file(&status_path) {
                    status_seq += 1;
                    status.status_seq = status_seq;
                    data.take_status(status);
                    changed = true;
                }
            }
        }

        if changed {
            publish(sinks, data);
            let _ = tx.send(data.clone());
            crate::wake::wake();
        }

        thread::sleep(Duration::from_millis(500));
    }
}

fn upload_companion_file(path: &Path, tx: &std::sync::mpsc::Sender<String>) {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Ok(compact) = serde_json::to_string(&value) {
                    let _ = tx.send(compact);
                }
            }
        }
        Err(e) => error!("Failed to read companion file {}: {}", path.display(), e),
    }
}

/// Polls (half a second each) a changed `Shipyard.json` waits for its `StoredShips` line.
const SHIPYARD_POLLS: u32 = 10;

/// What a `Shipyard.json` holds, judged against the journal lines written with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShipyardFile {
    /// What the station sells.
    Stock,
    /// The commander's own ships at the station (see
    /// [`edcas_common::journal::station::shipyard_is_own_fleet`]).
    OwnFleet,
    /// No `StoredShips` event for it (yet).
    Unsure,
}

fn judge_shipyard(dir: &Path, path: &Path) -> ShipyardFile {
    let Some(shipyard) = std::fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
    else {
        return ShipyardFile::Unsure;
    };
    let Some(text) = find_latest_journal_file(dir).and_then(|f| std::fs::read_to_string(f).ok()) else {
        return ShipyardFile::Unsure;
    };
    let (mut stored, mut ship) = (None, None);
    for line in text.lines().filter(|l| l.contains("\"StoredShips\"") || l.contains("\"Loadout\"")) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        match v["event"].as_str() {
            Some("Loadout") => ship = v["Ship"].as_str().map(str::to_string),
            Some("StoredShips") if v["MarketID"] == shipyard["MarketID"] && v["timestamp"] == shipyard["timestamp"] => {
                stored = Some(v)
            }
            _ => {}
        }
    }
    match stored {
        None => ShipyardFile::Unsure,
        Some(s) if edcas_common::journal::station::shipyard_is_own_fleet(&shipyard, &s, ship.as_deref()) => {
            ShipyardFile::OwnFleet
        }
        Some(_) => ShipyardFile::Stock,
    }
}

/// Reads a companion file (Market/Outfitting/Shipyard.json) and forwards it to the EDDN
/// uploader, which converts it into the appropriate EDDN schema.
fn send_companion_to_eddn(
    path: &Path,
    kind: crate::eddn::CompanionKind,
    tx: &mpsc::Sender<crate::eddn::EddnInput>,
) {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let _ = tx.send(crate::eddn::EddnInput::Companion { kind, content });
        }
        Err(e) => error!("Failed to read companion file {}: {}", path.display(), e),
    }
}

/// Feeds the current session file to the EDDN uploader as state-only input so that
/// augmentation data (game version, current system, commander) is available before any
/// live event is uploaded, without re-sending the historical events themselves.
fn seed_eddn_state(dir: &Path, tx: &mpsc::Sender<crate::eddn::EddnInput>) {
    let Some(path) = find_latest_journal_file(dir) else {
        return;
    };
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let _ = tx.send(crate::eddn::EddnInput::Seed(trimmed.to_owned()));
            }
        }
    }
}

fn read_file_lines(
    path: &Path,
    data: &mut JournalData,
    upload_tx: Option<&mpsc::Sender<String>>,
    eddn_tx: Option<&mpsc::Sender<crate::eddn::EddnInput>>,
) {
    match File::open(path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                let trimmed = line.trim().to_owned();
                if trimmed.is_empty() {
                    continue;
                }
                data.process_line(&trimmed);
                if let Some(tx) = eddn_tx {
                    if trimmed.len() <= 65_536 {
                        let _ = tx.send(crate::eddn::EddnInput::Line(trimmed.clone()));
                    }
                }
                if let Some(tx) = upload_tx {
                    // Skip large lines (NavRoute, ShipLocker snapshots) that have no server handler.
                    if trimmed.len() <= 65_536 {
                        let _ = tx.send(trimmed);
                    }
                }
            }
        }
        Err(e) => error!("Failed to open journal file {}: {}", path.display(), e),
    }
}

pub fn find_all_journal_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = read_dir
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "log")
                .unwrap_or(false)
        })
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            parse_journal_timestamp(&name).map(|ts| (e.path(), ts))
        })
        .collect();
    files.sort_by(|a, b| b.1.cmp(&a.1));
    files.into_iter().map(|(path, _)| path).collect()
}

pub fn find_latest_journal_file(dir: &Path) -> Option<PathBuf> {
    find_all_journal_files(dir).into_iter().next()
}

/// What the explorer shows of one system's scans: the current system's comes from the
/// live journal ([`SystemScans::of`]), a past one's from [`load_system_scans`].
#[derive(Clone, Default)]
pub struct SystemScans {
    pub bodies: Vec<BodyScan>,
    pub fss_signals: HashMap<i32, Vec<BodySignal>>,
    pub saa_data: HashMap<i32, SaaBodyData>,
    pub fss_body_count: Option<u32>,
    pub fss_non_body_count: Option<u32>,
    pub fss_all_bodies_found: bool,
    pub nav_beacon_bodies: Option<u32>,
    pub discovered_signals: Vec<DiscoveredSignal>,
}

impl SystemScans {
    /// The current system's, as the live journal has it.
    pub fn of(journal: &JournalData) -> Self {
        Self {
            bodies: journal.bodies.clone(),
            fss_signals: journal.fss_signals.clone(),
            saa_data: journal.saa_data.clone(),
            fss_body_count: journal.fss_body_count,
            fss_non_body_count: journal.fss_non_body_count,
            fss_all_bodies_found: journal.fss_all_bodies_found,
            nav_beacon_bodies: journal.nav_beacon_bodies,
            discovered_signals: journal.discovered_signals.clone(),
        }
    }
}

/// How many of the newest journals [`load_system_scans`] reads: the jump history only
/// reaches back a session or two.
const SCANS_LOOKBACK: usize = 20;

/// A past system's scans, read back from the newest journals (oldest first, so a later
/// visit's scans win).
pub fn load_system_scans(dir: &Path, system_address: i64) -> SystemScans {
    let mut data = JournalData::new();
    let files: Vec<PathBuf> = find_all_journal_files(dir).into_iter().take(SCANS_LOOKBACK).collect();
    for file in files.iter().rev() {
        load_previous_scans_for_system(file, system_address, &mut data, None);
    }
    SystemScans {
        bodies: data.bodies,
        fss_signals: data.fss_signals,
        saa_data: data.saa_data,
        fss_body_count: data.fss_body_count,
        fss_non_body_count: data.fss_non_body_count,
        fss_all_bodies_found: data.fss_all_bodies_found,
        ..Default::default()
    }
}

fn load_previous_scans_for_system(
    path: &Path,
    system_address: i64,
    data: &mut JournalData,
    upload_tx: Option<&mpsc::Sender<String>>,
) {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to open previous journal file {}: {}", path.display(), e);
            return;
        }
    };

    let mut current_sys_addr: Option<i64> = None;
    let mut odyssey = data.pilot.odyssey;
    let reader = BufReader::new(file);
    for line in reader.lines().flatten() {
        let trimmed = line.trim().to_owned();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(tx) = upload_tx {
            if trimmed.len() <= 65_536 {
                let _ = tx.send(trimmed.clone());
            }
        }
        let value: serde_json::Value = match serde_json::from_str(&trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        match value["event"].as_str() {
            Some("LoadGame") => odyssey = value["Odyssey"].as_bool().unwrap_or(false),
            Some("SAAScanComplete") if value["SystemAddress"].as_i64() == Some(system_address) => {
                mark_mapped(&mut data.bodies, &value);
            }
            Some("Disembark") if value["SystemAddress"].as_i64() == Some(system_address) => {
                mark_footfall(&mut data.bodies, &value);
            }
            _ => {}
        }
        match JournalEvent::from_json(value) {
            Some(JournalEvent::FsdJump(e)) => {
                current_sys_addr = Some(e.system_address);
            }
            Some(JournalEvent::Location(e)) => {
                current_sys_addr = Some(e.system_address);
            }
            Some(JournalEvent::CarrierJump(e)) => {
                current_sys_addr = Some(e.system_address);
            }
            Some(JournalEvent::Scan(e)) if current_sys_addr == Some(system_address) => {
                upsert_body(&mut data.bodies, body_from_scan(&e, odyssey));
            }
            Some(JournalEvent::ScanBaryCentre(e)) if current_sys_addr == Some(system_address) => {
                if !data.bodies.iter().any(|b| b.body_id == e.body_id) {
                    let parents = e.parents.as_ref()
                        .map(|pv| pv.iter()
                            .filter_map(|p| p.parent_id().map(|pid| BodyParent {
                                body_id: pid,
                            }))
                            .collect())
                        .unwrap_or_default();
                    data.bodies.push(BodyScan {
                        body_id: e.body_id,
                        body_name: format!("{} Barycentre", e.star_system),
                        planet_class: String::new(),
                        landable: false,
                        scan_type: "AutoScan".into(),
                        distance_from_arrival_ls: e.distance_from_arrival_ls,
                        radius: 0.0,
                        mass_em: 0.0,
                        surface_temperature: 0.0,
                        surface_gravity: 0.0,
                        tidal_lock: false,
                        volcanism: String::new(),
                        atmosphere: String::new(),
                        terraform_state: String::new(),
                        star_type: String::new(),
                        parents,
                        rings: vec![],
                        materials: vec![],
                        value: None,
                        mapped: None,
                        footfall: false,
                        composition: None,
                        discovery: None,
                        exo: None,
                    });
                }
            }
            Some(JournalEvent::FssDiscoveryScan(e)) if current_sys_addr == Some(system_address) => {
                data.fss_body_count = Some(e.body_count);
                data.fss_non_body_count = Some(e.non_body_count);
            }
            Some(JournalEvent::FssAllBodiesFound(_)) if current_sys_addr == Some(system_address) => {
                data.fss_all_bodies_found = true;
            }
            // Callers read the journals oldest first: a later scan replaces an earlier one.
            Some(JournalEvent::FssBodySignals(e)) if current_sys_addr == Some(system_address) => {
                data.fss_signals.insert(
                    e.body_id,
                    e.signals.iter().map(|s| BodySignal {
                        signal_type: s.signal_type.clone(),
                        signal_type_localised: s.signal_type_localised.clone(),
                        count: s.count,
                    }).collect(),
                );
            }
            Some(JournalEvent::SaaSignalsFound(e)) if current_sys_addr == Some(system_address) => {
                data.saa_data.insert(e.body_id, {
                    let signals = e.signals.iter().map(|s| BodySignal {
                        signal_type: s.signal_type.clone(),
                        signal_type_localised: s.signal_type_localised.clone(),
                        count: s.count,
                    }).collect();
                    let genuses = e.genuses.as_ref()
                        .map(|gv| gv.iter()
                            .map(|g| GenusSignal {
                                symbol: g.genus.clone(),
                                name: g.genus_localised.clone().unwrap_or_else(|| g.genus.trim_end_matches(';').replace('_', " ")),
                            })
                            .collect())
                        .unwrap_or_default();
                    SaaBodyData { signals, genuses }
                });
            }
            _ => {}
        }
    }
}

fn parse_journal_timestamp(filename: &str) -> Option<u64> {
    // Journal.YYYY-MM-DDTHHMMSS.NN.log
    let parts: Vec<&str> = filename.split('.').collect();
    if parts.len() >= 3 && parts[0] == "Journal" && *parts.last()? == "log" {
        let d = parts[1]; // "YYYY-MM-DDTHHMMSS" (17 chars)
        if d.len() != 17 { return None; }
        let year: u64 = d[0..4].parse().ok()?;
        let month: u64 = d[5..7].parse().ok()?;
        let day: u64 = d[8..10].parse().ok()?;
        let hour: u64 = d[11..13].parse().ok()?;
        let min: u64 = d[13..15].parse().ok()?;
        let sec: u64 = d[15..17].parse().ok()?;
        Some(year * 10_000_000_000 + month * 100_000_000 + day * 1_000_000 + hour * 10_000 + min * 100 + sec)
    } else {
        None
    }
}

/// Builds the explorer's body tree via the shared parent→child builder in
/// `edcas_common::tree` (each body nests under the nearest ancestor present in the
/// set; roots and children sorted by body id), then adapts the generic nodes into
/// the explorer's [`TreeNode`].
pub fn build_body_tree(bodies: &[BodyScan]) -> Vec<TreeNode> {
    edcas_common::tree::build_forest(
        bodies.to_vec(),
        |b| b.body_id,
        |b| b.parents.iter().map(|p| p.body_id).collect(),
    )
    .into_iter()
    .map(convert_tree_node)
    .collect()
}

fn convert_tree_node(n: edcas_common::tree::TreeNode<BodyScan>) -> TreeNode {
    TreeNode {
        name: n.item.body_name.clone(),
        body_id: n.item.body_id,
        children: n.children.into_iter().map(convert_tree_node).collect(),
        data: Some(n.item),
    }
}

#[derive(Clone)]
pub struct TreeNode {
    pub name: String,
    pub body_id: i32,
    pub children: Vec<TreeNode>,
    pub data: Option<BodyScan>,
}

#[cfg(test)]
mod upload_tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};

    /// Spawns a minimal HTTP server on a random port.
    ///
    /// `status_fn` is called with the 0-based request index and returns the
    /// HTTP status code to send back.  Returns `(port, request_count)`.
    fn spawn_mock_server(
        status_fn: impl Fn(usize) -> u16 + Send + 'static,
    ) -> (u16, Arc<Mutex<usize>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let count = Arc::new(Mutex::new(0usize));
        let count_clone = count.clone();

        std::thread::spawn(move || {
            let mut idx = 0;
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                let status = status_fn(idx);
                serve_request(stream, status);
                *count_clone.lock().unwrap() += 1;
                idx += 1;
            }
        });

        (port, count)
    }

    fn serve_request(stream: TcpStream, status: u16) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut content_length = 0usize;

        // Read HTTP headers
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line).is_err() {
                return;
            }
            if line == "\r\n" || line.is_empty() {
                break;
            }
            let lower = line.to_lowercase();
            if lower.starts_with("content-length:") {
                content_length = line[16..].trim().parse().unwrap_or(0);
            }
        }
        // Drain the request body so the client doesn't get a broken-pipe error
        let mut body = vec![0u8; content_length];
        let _ = reader.read_exact(&mut body);

        let status_text = match status {
            200 => "OK",
            202 => "Accepted",
            204 => "No Content",
            504 => "Gateway Timeout",
            _ => "Internal Server Error",
        };
        let mut stream = reader.into_inner();
        let _ = write!(
            stream,
            "HTTP/1.1 {} {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            status, status_text
        );
    }

    /// Creates a temp directory containing a single journal file with the given lines.
    fn make_journal_dir(lines: &[&str]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "edcas_upload_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let content = lines.join("\n");
        std::fs::write(dir.join("Journal.2022-01-01T000000.01.log"), content).unwrap();
        dir
    }

    fn sample_fsd_jump() -> &'static str {
        r#"{ "timestamp":"2022-06-28T15:16:44Z", "event":"FSDJump", "Taxi":false, "Multicrew":false, "StarSystem":"Test", "SystemAddress":1, "StarPos":[0.0,0.0,0.0], "SystemAllegiance":"", "SystemEconomy":"$economy_None;", "SystemSecondEconomy":"$economy_None;", "SystemGovernment":"$government_None;", "SystemSecurity":"$GAlAXY_MAP_INFO_state_anarchy;", "Population":0, "Body":"Test", "BodyID":0, "BodyType":"Star" }"#
    }

    #[test]
    fn upload_succeeds_and_reports_correct_counts() {
        // 55 valid events → BATCH_SIZE=50 means 2 batches (50+5)
        let events: Vec<&str> = std::iter::repeat(sample_fsd_jump()).take(55).collect();
        let dir = make_journal_dir(&events);

        let (port, req_count) = spawn_mock_server(|_| 202);
        let rx = start_bulk_upload(dir.clone(), format!("http://127.0.0.1:{}", port));

        let last = rx.into_iter().last().unwrap();
        assert!(last.done, "upload should be marked done");
        assert_eq!(last.lines_done, 55, "all 55 events should be counted");
        assert!(last.error.is_none(), "no errors expected: {:?}", last.error);
        assert_eq!(*req_count.lock().unwrap(), 2, "expected 2 batch requests");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn upload_counts_504_as_send_error() {
        let events: Vec<&str> = std::iter::repeat(sample_fsd_jump()).take(5).collect();
        let dir = make_journal_dir(&events);

        let (port, _) = spawn_mock_server(|_| 504);
        let rx = start_bulk_upload(dir.clone(), format!("http://127.0.0.1:{}", port));

        let last = rx.into_iter().last().unwrap();
        assert!(last.done);
        assert!(
            last.error.is_some(),
            "504 response should be recorded as a send error"
        );
        assert!(
            last.error.as_deref().unwrap_or("").contains("send error"),
            "error message should mention 'send error', got: {:?}",
            last.error
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn upload_skips_empty_and_malformed_lines() {
        let lines = [
            sample_fsd_jump(),
            "",
            "   ",
            "not json",
            sample_fsd_jump(),
        ];
        let dir = make_journal_dir(&lines);

        let (port, req_count) = spawn_mock_server(|_| 202);
        let rx = start_bulk_upload(dir.clone(), format!("http://127.0.0.1:{}", port));

        let last = rx.into_iter().last().unwrap();
        assert!(last.done);
        assert_eq!(last.lines_done, 2, "only valid JSON lines count");
        assert!(last.error.is_none());
        // 2 valid events fit in a single batch
        assert_eq!(*req_count.lock().unwrap(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn upload_empty_directory_reports_no_files_error() {
        let dir = std::env::temp_dir().join(format!(
            "edcas_upload_test_empty_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        // No server needed — upload fails immediately
        let rx = start_bulk_upload(dir.clone(), "http://127.0.0.1:1".into());

        let last = rx.into_iter().last().unwrap();
        assert!(last.done);
        assert!(last.error.is_some(), "empty dir should report an error");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn upload_mixed_success_and_error_accumulates_count() {
        // 55 events → 2 batches (50+5).  First batch (idx=0) succeeds; all
        // subsequent requests (retries of batch 1) return 503 so that batch
        // exhausts its retries and counts as exactly 1 send error.
        let events: Vec<&str> = std::iter::repeat(sample_fsd_jump()).take(55).collect();
        let dir = make_journal_dir(&events);

        let (port, _) = spawn_mock_server(|idx| if idx == 0 { 202 } else { 503 });
        let rx = start_bulk_upload(dir.clone(), format!("http://127.0.0.1:{}", port));

        let last = rx.into_iter().last().unwrap();
        assert!(last.done);
        assert_eq!(last.lines_done, 55);
        assert!(last.error.as_deref().unwrap_or("").contains("1 send error"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod material_tracking_tests {
    use super::*;

    // ShipLocker.json/Backpack.json list one entry per pickup, not per material — a mission
    // rewarding the same material multiple times produces several `Count:1` entries sharing
    // one `Name`, e.g. the real ShipLocker.json line:
    //   { "Name":"largecapacitypowerregulator", "MissionID":1039025033, "Count":1 },
    //   { "Name":"largecapacitypowerregulator", "MissionID":1044487368, "Count":1 }, ...
    // parse_inventory_array must merge these or on-foot have-counts undercount badly.
    #[test]
    fn parse_inventory_array_merges_duplicate_mission_tied_entries() {
        let v: serde_json::Value = serde_json::from_str(r#"[
            { "Name":"largecapacitypowerregulator", "Name_Localised":"Energieregulator", "OwnerID":0, "MissionID":1, "Count":1 },
            { "Name":"largecapacitypowerregulator", "Name_Localised":"Energieregulator", "OwnerID":0, "MissionID":2, "Count":1 },
            { "Name":"largecapacitypowerregulator", "Name_Localised":"Energieregulator", "OwnerID":0, "MissionID":3, "Count":1 },
            { "Name":"ionisedgas", "Name_Localised":"Ionisiertes Gas", "Count":2 }
        ]"#).unwrap();
        let items = parse_inventory_array(&v);
        assert_eq!(items.len(), 2);
        let regulator = items.iter().find(|i| i.name == "largecapacitypowerregulator").unwrap();
        assert_eq!(regulator.count, 3);
        assert_eq!(regulator.localised, "Energieregulator");
        let gas = items.iter().find(|i| i.name == "ionisedgas").unwrap();
        assert_eq!(gas.count, 2);
    }

    fn raw_count(journal: &JournalData, name: &str) -> i32 {
        journal.materials_raw.iter().find(|i| i.name == name).map(|i| i.count).unwrap_or(0)
    }
    fn manufactured_count(journal: &JournalData, name: &str) -> i32 {
        journal.materials_manufactured.iter().find(|i| i.name == name).map(|i| i.count).unwrap_or(0)
    }
    fn encoded_count(journal: &JournalData, name: &str) -> i32 {
        journal.materials_encoded.iter().find(|i| i.name == name).map(|i| i.count).unwrap_or(0)
    }

    // The initial `Materials` snapshot only reflects counts at session start; without handling
    // the incremental events below, everything collected/spent afterward is invisible to the
    // Todo/Engineers tabs' "have" counts.
    #[test]
    fn witch_space_runs_from_the_music_stopping_after_a_hyperspace_countdown_to_the_arrival() {
        let mut j = JournalData::new();
        j.process_line(r#"{ "timestamp":"2026-11-09T12:24:22Z", "event":"StartJump", "JumpType":"Hyperspace", "StarSystem":"Slege MS-L b55-5", "SystemAddress":1, "StarClass":"M" }"#);
        assert!(!j.witch_space, "the countdown is not witch space");
        j.process_line(r#"{ "timestamp":"2026-11-09T12:24:27Z", "event":"Music", "MusicTrack":"NoTrack" }"#);
        assert!(j.witch_space);
        assert_eq!(Place::of(&j).jump.as_deref(), Some("M"));
        j.process_line(r#"{ "timestamp":"2026-11-09T12:24:28Z", "event":"FSDTarget", "Name":"Slege OY-J b56-1", "SystemAddress":2, "StarClass":"M" }"#);
        assert!(j.witch_space, "targeting the next jump changes nothing");
        j.process_line(r#"{ "timestamp":"2026-11-09T12:24:41Z", "event":"FSDJump", "StarSystem":"Slege MS-L b55-5", "SystemAddress":1, "StarPos":[0,0,0], "JumpDist":10.0, "FuelUsed":1.0, "FuelLevel":10.0 }"#);
        assert!(!j.witch_space);
        // A supercruise jump's music stop is not witch space; the music stopping with no countdown neither.
        j.process_line(r#"{ "timestamp":"2026-11-09T12:25:00Z", "event":"StartJump", "JumpType":"Supercruise" }"#);
        j.process_line(r#"{ "timestamp":"2026-11-09T12:25:05Z", "event":"Music", "MusicTrack":"NoTrack" }"#);
        assert!(!j.witch_space);
        assert_eq!(Place::of(&j).jump, None);
    }

    #[test]
    fn material_collected_after_snapshot_increments_count() {
        let mut journal = JournalData::new();
        journal.process_line(r#"{ "event":"Materials", "Raw":[ { "Name":"carbon", "Count":10 } ], "Manufactured":[], "Encoded":[] }"#);
        assert_eq!(raw_count(&journal, "carbon"), 10);

        journal.process_line(r#"{ "event":"MaterialCollected", "Category":"Raw", "Name":"carbon", "Name_Localised":"Kohlenstoff", "Count":5 }"#);
        assert_eq!(raw_count(&journal, "carbon"), 15);

        // Collecting a material with no prior snapshot entry adds it fresh.
        journal.process_line(r#"{ "event":"MaterialCollected", "Category":"Encoded", "Name":"decodedemissiondata", "Name_Localised":"Entschlüsselte Emissionsdaten", "Count":3 }"#);
        assert_eq!(encoded_count(&journal, "decodedemissiondata"), 3);
    }

    #[test]
    fn material_discarded_decrements_and_removes_at_zero() {
        let mut journal = JournalData::new();
        journal.process_line(r#"{ "event":"Materials", "Raw":[ { "Name":"carbon", "Count":10 } ], "Manufactured":[], "Encoded":[] }"#);

        journal.process_line(r#"{ "event":"MaterialDiscarded", "Category":"Raw", "Name":"carbon", "Count":4 }"#);
        assert_eq!(raw_count(&journal, "carbon"), 6);

        journal.process_line(r#"{ "event":"MaterialDiscarded", "Category":"Raw", "Name":"carbon", "Count":6 }"#);
        assert_eq!(raw_count(&journal, "carbon"), 0);
        assert!(!journal.materials_raw.iter().any(|i| i.name == "carbon"));
    }

    #[test]
    fn material_trade_moves_quantity_between_materials() {
        let mut journal = JournalData::new();
        journal.process_line(r#"{ "event":"Materials", "Raw":[], "Manufactured":[], "Encoded":[ { "Name":"decodedemissiondata", "Count":10 }, { "Name":"industrialfirmware", "Count":0 } ] }"#);

        journal.process_line(r#"{ "event":"MaterialTrade", "Paid":{ "Material":"decodedemissiondata", "Category":"Encoded", "Quantity":6 }, "Received":{ "Material":"industrialfirmware", "Category":"Encoded", "Quantity":3 } }"#);
        assert_eq!(encoded_count(&journal, "decodedemissiondata"), 4);
        assert_eq!(encoded_count(&journal, "industrialfirmware"), 3);
    }

    #[test]
    fn engineer_craft_consumes_ingredients_without_stated_category() {
        let mut journal = JournalData::new();
        journal.process_line(r#"{ "event":"Materials", "Raw":[ { "Name":"arsenic", "Count":10 } ], "Manufactured":[ { "Name":"chemicalstorageunits", "Count":8 } ], "Encoded":[] }"#);

        journal.process_line(r#"{ "event":"EngineerCraft", "Ingredients":[ { "Name":"chemicalstorageunits", "Count":5 }, { "Name":"arsenic", "Count":3 } ] }"#);
        assert_eq!(manufactured_count(&journal, "chemicalstorageunits"), 3);
        assert_eq!(raw_count(&journal, "arsenic"), 7);
    }

    #[test]
    fn synthesis_consumes_materials() {
        let mut journal = JournalData::new();
        journal.process_line(r#"{ "event":"Materials", "Raw":[], "Manufactured":[ { "Name":"basicconductors", "Count":10 }, { "Name":"heatconductionwiring", "Count":10 } ], "Encoded":[] }"#);

        journal.process_line(r#"{ "event":"Synthesis", "Name":"Heat Sink Basic", "Materials":[ { "Name":"basicconductors", "Count":2 }, { "Name":"heatconductionwiring", "Count":2 } ] }"#);
        assert_eq!(manufactured_count(&journal, "basicconductors"), 8);
        assert_eq!(manufactured_count(&journal, "heatconductionwiring"), 8);
    }

    #[test]
    fn engineer_contribution_of_materials_consumes_but_commodities_are_ignored() {
        let mut journal = JournalData::new();
        journal.process_line(r#"{ "event":"Materials", "Raw":[], "Manufactured":[], "Encoded":[ { "Name":"scandatabanks", "Count":60 } ] }"#);

        journal.process_line(r#"{ "event":"EngineerContribution", "Type":"Materials", "Material":"scandatabanks", "Quantity":50, "TotalQuantity":50 }"#);
        assert_eq!(encoded_count(&journal, "scandatabanks"), 10);

        // Commodity contributions must not be mistaken for materials.
        journal.process_line(r#"{ "event":"EngineerContribution", "Type":"Commodity", "Commodity":"xihecompanions", "Quantity":25, "TotalQuantity":25 }"#);
        assert_eq!(encoded_count(&journal, "scandatabanks"), 10);
    }
}

#[cfg(test)]
mod body_scan_tests {
    use super::*;

    // A body is scanned again when its surface map completes; the tree showed it twice.
    #[test]
    fn a_rescanned_body_is_listed_once_with_the_newer_scan() {
        let mut journal = JournalData::new();
        let scan = |scan_type: &str, value: i64| {
            format!(
                r#"{{ "timestamp":"2024-08-19T19:40:00Z", "event":"Scan", "ScanType":"{scan_type}", "BodyName":"Hypoae Aihm WT-O b36-0 B 4", "BodyID":26,
                     "Parents":[ {{"Star":3}}, {{"Null":1}}, {{"Null":0}} ], "StarSystem":"Hypoae Aihm WT-O b36-0",
                     "SystemAddress":2008131806497, "DistanceFromArrivalLS":817.97, "PlanetClass":"High metal content body",
                     "Landable":true, "WasDiscovered":false, "WasMapped":false, "EstimatedValue":{value} }}"#
            )
        };
        journal.process_line(&scan("AutoScan", 100));
        journal.process_line(&scan("Detailed", 200));
        assert_eq!(journal.bodies.len(), 1);
        assert_eq!(journal.bodies[0].scan_type, "Detailed");
        assert_eq!(journal.session.bodies_scanned, 1);
    }

    // The order the game writes a surface map in: SAAScanComplete, SAASignalsFound, then a
    // fresh Detailed scan of the body (Oochont BC-Z b19-0 A 2, 2026-10-13).
    #[test]
    fn a_surface_map_marks_the_body_and_survives_the_rescan() {
        let mut journal = JournalData::new();
        let scan = r#"{ "timestamp":"2026-10-13T06:07:42Z", "event":"Scan", "ScanType":"Detailed", "BodyName":"Oochont BC-Z b19-0 A 2", "BodyID":5, "Parents":[ {"Star":1} ], "StarSystem":"Oochont BC-Z b19-0", "SystemAddress":1, "DistanceFromArrivalLS":300.0, "PlanetClass":"Icy body", "MassEM":0.5, "Landable":true, "WasDiscovered":false, "WasMapped":false, "WasFootfalled":false }"#;
        journal.process_line(scan);
        journal.process_line(r#"{ "timestamp":"2026-10-13T06:10:01Z", "event":"SAAScanComplete", "BodyName":"Oochont BC-Z b19-0 A 2", "SystemAddress":1, "BodyID":5, "ProbesUsed":2, "EfficiencyTarget":2 }"#);
        journal.process_line(r#"{ "timestamp":"2026-10-13T06:10:01Z", "event":"SAASignalsFound", "BodyName":"Oochont BC-Z b19-0 A 2", "SystemAddress":1, "BodyID":5, "Signals":[], "Genuses":[] }"#);
        assert_eq!(journal.bodies[0].mapped, Some(true));
        journal.process_line(&scan.replace("06:07:42", "06:10:01"));
        assert_eq!(journal.bodies[0].mapped, Some(true));
        assert!(journal.bodies[0].credits() > journal.bodies[0].value.unwrap().credits(None).into());
    }

    // Pro Thaa RR-W d1-67 7 c, 2026-10-28: scanned untrodden, then the commander got out on it.
    #[test]
    fn a_disembark_on_the_planet_is_the_commanders_footfall() {
        let mut journal = JournalData::new();
        let scan = r#"{ "timestamp":"2026-10-28T12:36:51Z", "event":"Scan", "ScanType":"Detailed", "BodyName":"Pro Thaa RR-W d1-67 7 c", "BodyID":44, "StarSystem":"Pro Thaa RR-W d1-67", "SystemAddress":2310885361811, "DistanceFromArrivalLS":900.0, "PlanetClass":"Rocky body", "MassEM":0.01, "Landable":true, "WasDiscovered":false, "WasMapped":false, "WasFootfalled":false }"#;
        journal.process_line(scan);
        let disembark = r#"{ "timestamp":"2026-10-28T13:04:58Z", "event":"Disembark", "SRV":false, "Taxi":false, "Multicrew":false, "ID":153, "StarSystem":"Pro Thaa RR-W d1-67", "SystemAddress":2310885361811, "Body":"Pro Thaa RR-W d1-67 7 c", "BodyID":44, "OnStation":false, "OnPlanet":true }"#;
        // Getting out at a station is no footfall.
        journal.process_line(&disembark.replace(r#""OnStation":false, "OnPlanet":true"#, r#""OnStation":true, "OnPlanet":false"#));
        assert!(!journal.bodies[0].footfall);
        journal.process_line(disembark);
        assert!(journal.bodies[0].footfall);
        // A later scan that counts the commander's own footfall keeps it theirs, and first.
        journal.process_line(&scan.replace(r#""WasFootfalled":false"#, r#""WasFootfalled":true"#));
        assert!(journal.bodies[0].footfall);
        assert_eq!(journal.bodies[0].discovery.and_then(|d| d.footfalled), Some(false));
    }
}

#[cfg(test)]
mod shipyard_upload_tests {
    use super::*;

    fn dir_with(name: &str, journal: &str, shipyard: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("edcas-shipyard-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Journal.2026-10-14T100000.01.log"), journal).unwrap();
        std::fs::write(dir.join("Shipyard.json"), shipyard).unwrap();
        dir
    }

    // Fuxport, 2026-10-14: the file named the Type-9 flown, the shipyard sold 36 types.
    #[test]
    fn the_commanders_own_ships_are_not_uploaded() {
        let journal = r#"{ "timestamp":"2026-10-14T10:00:00Z", "event":"Loadout", "Ship":"type9", "ShipID":3 }
{ "timestamp":"2026-10-14T11:02:36Z", "event":"StoredShips", "MarketID":4210375683, "ShipsHere":[  ], "ShipsRemote":[  ] }
"#;
        let shipyard = r#"{ "timestamp":"2026-10-14T11:02:36Z", "event":"Shipyard", "MarketID":4210375683, "PriceList":[ { "id":0, "ShipType":"type9", "ShipPrice":169483352 } ] }"#;
        let dir = dir_with("own", journal, shipyard);
        assert_eq!(judge_shipyard(&dir, &dir.join("Shipyard.json")), ShipyardFile::OwnFleet);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_stock_list_is_uploaded_and_one_without_stored_ships_waits() {
        let shipyard = r#"{ "timestamp":"2026-10-14T11:02:36Z", "event":"Shipyard", "MarketID":1, "PriceList":[ { "id":1, "ShipType":"sidewinder", "ShipPrice":31200 }, { "id":2, "ShipType":"eagle", "ShipPrice":43680 } ] }"#;
        let stored = r#"{ "timestamp":"2026-10-14T10:00:00Z", "event":"Loadout", "Ship":"type9", "ShipID":3 }
{ "timestamp":"2026-10-14T11:02:36Z", "event":"StoredShips", "MarketID":1, "ShipsHere":[  ], "ShipsRemote":[  ] }
"#;
        let dir = dir_with("stock", stored, shipyard);
        assert_eq!(judge_shipyard(&dir, &dir.join("Shipyard.json")), ShipyardFile::Stock);
        std::fs::remove_dir_all(&dir).ok();
        let dir = dir_with("unsure", "", shipyard);
        assert_eq!(judge_shipyard(&dir, &dir.join("Shipyard.json")), ShipyardFile::Unsure);
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod restart_backfill_tests {
    use super::*;

    // A game restart while landed (Hegoea EN-F b39-0 B 6, 2026-10-14): the new journal
    // starts with Location and that body's signals alone; the system's other surface
    // scans are in the journal before it.
    #[test]
    fn a_restart_keeps_the_systems_earlier_scans() {
        let dir = std::env::temp_dir().join(format!("edcas-restart-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let saa = |id: i32, ts: &str, count: u32| {
            format!(r#"{{ "timestamp":"{ts}", "event":"SAASignalsFound", "BodyName":"Hegoea EN-F b39-0 B {id}", "SystemAddress":576197239121, "BodyID":{id}, "Signals":[ {{ "Type":"$SAA_SignalType_Biological;", "Type_Localised":"Biological", "Count":{count} }} ], "Genuses":[] }}"#)
        };
        let earlier = [
            r#"{ "timestamp":"2026-10-14T11:27:45Z", "event":"FSDJump", "StarSystem":"Hegoea EN-F b39-0", "SystemAddress":576197239121, "StarPos":[-7046.25,139.78125,-2778.5] }"#.to_string(),
            saa(7, "2026-10-14T11:40:00Z", 2),
            saa(8, "2026-10-14T11:45:00Z", 3),
            saa(9, "2026-10-14T11:50:00Z", 4),
        ]
        .join("\n");
        std::fs::write(dir.join("Journal.2026-10-14T074711.01.log"), earlier).unwrap();
        let now = [
            r#"{ "timestamp":"2026-10-14T12:24:38Z", "event":"Location", "Docked":false, "StarSystem":"Hegoea EN-F b39-0", "SystemAddress":576197239121, "StarPos":[-7046.25,139.78125,-2778.5], "Body":"Hegoea EN-F b39-0 B 6", "BodyID":9, "BodyType":"Planet" }"#.to_string(),
            saa(9, "2026-10-14T12:24:38Z", 5),
        ]
        .join("\n");
        std::fs::write(dir.join("Journal.2026-10-14T132358.01.log"), now).unwrap();

        let mut data = JournalData::new();
        load_existing_files(&dir, &mut data);
        std::fs::remove_dir_all(&dir).ok();
        let mut ids: Vec<i32> = data.saa_data.keys().copied().collect();
        ids.sort();
        assert_eq!(ids, [7, 8, 9]);
        // This session's scan of the landed body wins over the older one.
        assert_eq!(data.saa_data[&9].signals[0].count, 5);
    }

    // 2026-10-26: Hegoea CX-B d13-3 4 a was scanned, then the game restarted 10 times
    // without leaving the system; the scan was 11 journals back.
    #[test]
    fn the_walk_reaches_the_arrival_through_many_restarts() {
        let dir = std::env::temp_dir().join(format!("edcas-restart-many-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let jump = |ts: &str, addr: i64| {
            format!(r#"{{ "timestamp":"{ts}", "event":"FSDJump", "StarSystem":"S{addr}", "SystemAddress":{addr}, "StarPos":[0.0,0.0,0.0] }}"#)
        };
        let location = |ts: &str| {
            format!(r#"{{ "timestamp":"{ts}", "event":"Location", "Docked":false, "StarSystem":"Hegoea CX-B d13-3", "SystemAddress":42, "StarPos":[0.0,0.0,0.0], "Body":"Hegoea CX-B d13-3 4 a", "BodyID":5, "BodyType":"Planet" }}"#)
        };
        let scan = r#"{ "timestamp":"2026-10-26T08:01:00Z", "event":"Scan", "ScanType":"Detailed", "BodyName":"Hegoea CX-B d13-3 4 a", "BodyID":5, "StarSystem":"Hegoea CX-B d13-3", "SystemAddress":42, "DistanceFromArrivalLS":100.0, "Landable":true, "PlanetClass":"Rocky body" }"#;
        // A previous visit's scan, before the arrival: the walk must not need it.
        let old_scan = scan.replace("\"BodyID\":5", "\"BodyID\":6").replace("4 a", "4 b");
        std::fs::write(
            dir.join("Journal.2026-10-25T100000.01.log"),
            [jump("2026-10-25T10:00:00Z", 42), old_scan].join("\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("Journal.2026-10-26T080000.01.log"),
            [jump("2026-10-26T08:00:00Z", 42), scan.to_string()].join("\n"),
        )
        .unwrap();
        for i in 0..10 {
            std::fs::write(
                dir.join(format!("Journal.2026-10-26T09{i:02}00.01.log")),
                [location(&format!("2026-10-26T09:{i:02}:00Z"))].join("\n"),
            )
            .unwrap();
        }
        std::fs::write(
            dir.join("Journal.2026-10-26T110000.01.log"),
            location("2026-10-26T11:00:00Z"),
        )
        .unwrap();

        let mut data = JournalData::new();
        load_existing_files(&dir, &mut data);
        std::fs::remove_dir_all(&dir).ok();
        let names: Vec<&str> = data.bodies.iter().map(|b| b.body_name.as_str()).collect();
        assert_eq!(names, ["Hegoea CX-B d13-3 4 a"]);
    }
    fn jump_to(ts: &str, addr: i64) -> String {
        format!(r#"{{ "timestamp":"{ts}", "event":"FSDJump", "StarSystem":"S{addr}", "SystemAddress":{addr}, "StarPos":[0.0,0.0,0.0] }}"#)
    }
    fn location_in(ts: &str, addr: i64) -> String {
        format!(r#"{{ "timestamp":"{ts}", "event":"Location", "Docked":false, "StarSystem":"S{addr}", "SystemAddress":{addr}, "StarPos":[0.0,0.0,0.0], "Body":"S{addr} 1", "BodyID":1, "BodyType":"Planet" }}"#)
    }
    const LOAD_GAME: &str = r#"{ "timestamp":"2026-10-26T12:00:00Z", "event":"LoadGame", "Commander":"Test", "Ship":"sidewinder", "Credits":1 }"#;
    fn scan_in(addr: i64, id: i32) -> String {
        format!(r#"{{ "timestamp":"2026-10-26T08:01:00Z", "event":"Scan", "ScanType":"Detailed", "BodyName":"S{addr} {id}", "BodyID":{id}, "StarSystem":"S{addr}", "SystemAddress":{addr}, "DistanceFromArrivalLS":100.0, "Landable":true, "PlanetClass":"Rocky body" }}"#)
    }

    // 2026-10-26: the game restarted while EDCAS kept running; the new journal starts
    // with LoadGame and a Location in the system it was in, and that dropped the scans.
    #[test]
    fn a_new_journal_in_the_same_system_keeps_the_bodies() {
        let mut data = JournalData::new();
        for line in [jump_to("2026-10-26T08:00:00Z", 42), scan_in(42, 5)] {
            data.process_line(&line);
        }
        assert_eq!(data.bodies.len(), 1);

        // What the watcher does for a new journal file, then the file's events.
        data.clear();
        for line in [LOAD_GAME.to_string(), location_in("2026-10-26T12:00:01Z", 42)] {
            data.process_line(&line);
        }
        assert_eq!(data.bodies.len(), 1);
        assert_eq!(data.bodies[0].body_name, "S42 5");
        assert!(!data.needs_backfill);
    }

    #[test]
    fn a_new_journal_in_another_system_clears_the_bodies() {
        let mut data = JournalData::new();
        for line in [jump_to("2026-10-26T08:00:00Z", 42), scan_in(42, 5)] {
            data.process_line(&line);
        }
        data.clear();
        for line in [LOAD_GAME.to_string(), location_in("2026-10-26T12:00:01Z", 43)] {
            data.process_line(&line);
        }
        assert!(data.bodies.is_empty());
        assert_eq!(data.current_system.as_ref().map(|s| s.system_address), Some(43));
        assert!(data.needs_backfill);
    }

    // The watcher: the game creates the new journal first and writes LoadGame and
    // Location a moment later, so they reach it as an appended tail, not in the read of
    // the new file. A Location in a system without held scans pulls them from the
    // earlier journals.
    #[test]
    fn the_watcher_fills_a_new_journals_system_from_earlier_journals() {
        let dir = std::env::temp_dir().join(format!("edcas-watch-restart-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        // The scans of system 42 are in the first journal; the commander is in system 43.
        std::fs::write(
            dir.join("Journal.2026-10-26T080000.01.log"),
            [jump_to("2026-10-26T08:00:00Z", 42), scan_in(42, 5)].join("\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("Journal.2026-10-26T090000.01.log"),
            jump_to("2026-10-26T09:00:00Z", 43),
        )
        .unwrap();
        let mut data = JournalData::new();
        load_existing_files(&dir, &mut data);
        assert!(data.bodies.is_empty());

        let stop = AtomicBool::new(false);
        let (tx, rx) = mpsc::channel();
        let new_journal = dir.join("Journal.2026-10-26T120000.01.log");
        let mut last = None;
        thread::scope(|scope| {
            scope.spawn(|| watch_latest_file(&dir, &mut data, &tx, &Sinks::default(), &stop, &None, &None));
            thread::sleep(Duration::from_millis(700));
            std::fs::write(&new_journal, "{ \"timestamp\":\"2026-10-26T12:00:00Z\", \"event\":\"Fileheader\" }\n").unwrap();
            thread::sleep(Duration::from_millis(1200));
            let mut f = OpenOptions::new().append(true).open(&new_journal).unwrap();
            use std::io::Write;
            writeln!(f, "{LOAD_GAME}").unwrap();
            writeln!(f, "{}", location_in("2026-10-26T12:00:01Z", 42)).unwrap();
            drop(f);
            thread::sleep(Duration::from_millis(1500));
            stop.store(true, Ordering::SeqCst);
            while let Ok(d) = rx.try_recv() {
                last = Some(d);
            }
        });
        std::fs::remove_dir_all(&dir).ok();
        let last = last.expect("watcher sent data");
        let names: Vec<&str> = last.bodies.iter().map(|b| b.body_name.as_str()).collect();
        assert_eq!(names, ["S42 5"]);
    }
}

#[cfg(test)]
mod body_here_tests {
    use super::*;

    const SYSTEM: i64 = 576197239121;

    fn system() -> SystemData {
        SystemData {
            name: "Hegoea".to_string(),
            system_address: SYSTEM,
            economy: String::new(),
            second_economy: String::new(),
            government: String::new(),
            allegiance: String::new(),
            security: String::new(),
            population: 0,
            factions: vec![],
            system_faction: String::new(),
            controlling_power: None,
            powers: vec![],
            star_pos: None,
        }
    }

    /// A journal in `SYSTEM` with two scanned bodies: "Hegoea 1" (ID 4), "Hegoea 2" (ID 5).
    fn journal() -> JournalData {
        let mut j = JournalData::new();
        j.current_system = Some(system());
        for (id, name) in [(4, "Hegoea 1"), (5, "Hegoea 2")] {
            j.bodies.push(BodyScan { body_id: id, body_name: name.to_string(), ..Default::default() });
        }
        j
    }

    fn target(j: &mut JournalData, system: i64, body: i32, name: &str) {
        j.status.destination_system = system;
        j.status.destination_body = body;
        j.status.destination = name.to_string();
    }

    #[test]
    fn the_status_body_wins() {
        let mut j = journal();
        target(&mut j, SYSTEM, 5, "Hegoea 2");
        j.status.body_name = "Hegoea 1".to_string();
        assert_eq!(j.body_name_here(), "Hegoea 1");
        assert_eq!(Place::of(&j).body_name, "Hegoea 1");
    }

    #[test]
    fn without_a_status_body_the_targeted_scanned_body_of_the_system_counts() {
        let mut j = journal();
        assert_eq!(j.body_name_here(), "", "no target");
        target(&mut j, SYSTEM, 5, "Hegoea 2");
        assert_eq!(j.body_name_here(), "Hegoea 2");
        assert_eq!(Place::of(&j).body_name, "Hegoea 2");
    }

    #[test]
    fn other_targets_name_no_body() {
        let mut j = journal();
        target(&mut j, SYSTEM + 1, 5, "Hegoea 2");
        assert_eq!(j.body_name_here(), "", "another system");
        target(&mut j, SYSTEM, 0, "Orbis Station");
        assert_eq!(j.body_name_here(), "", "a station");
        target(&mut j, SYSTEM, 0, "Hegoea");
        assert_eq!(j.body_name_here(), "", "an unscanned star");
        target(&mut j, SYSTEM, 6, "Hegoea 3");
        assert_eq!(j.body_name_here(), "", "an unscanned planet");
        target(&mut j, SYSTEM, 4, "Hegoea 2");
        assert_eq!(j.body_name_here(), "", "ID and name disagree");
    }
}
