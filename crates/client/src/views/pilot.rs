pub(in crate::views) mod gui;

use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::{PilotData, StatusData};
use crate::views::ViewEvent;

const COMBAT_RANKS: &[&str] = &[
    "Harmless", "Mostly Harmless", "Novice", "Competent", "Expert",
    "Master", "Dangerous", "Deadly", "Elite", "Elite I", "Elite II", "Elite III", "Elite IV", "Elite V",
];
const TRADE_RANKS: &[&str] = &[
    "Penniless", "Mostly Penniless", "Peddler", "Dealer", "Merchant",
    "Broker", "Entrepreneur", "Tycoon", "Elite", "Elite I", "Elite II", "Elite III", "Elite IV", "Elite V",
];
const EXPLORE_RANKS: &[&str] = &[
    "Aimless", "Mostly Aimless", "Scout", "Surveyor", "Trailblazer",
    "Pathfinder", "Ranger", "Pioneer", "Elite", "Elite I", "Elite II", "Elite III", "Elite IV", "Elite V",
];
const SOLDIER_RANKS: &[&str] = &[
    "Defenceless", "Mostly Defenceless", "Rookie", "Soldier", "Gunslinger",
    "Warrior", "Gladiator", "Deadeye", "Elite", "Elite I", "Elite II", "Elite III", "Elite IV", "Elite V",
];
const EXOBIO_RANKS: &[&str] = &[
    "Directionless", "Mostly Directionless", "Compiler", "Collector", "Cataloguer",
    "Taxonomist", "Ecologist", "Geneticist", "Elite", "Elite I", "Elite II", "Elite III", "Elite IV", "Elite V",
];
const FEDERATION_RANKS: &[&str] = &[
    "None", "Recruit", "Cadet", "Midshipman", "Petty Officer", "Chief Petty Officer",
    "Warrant Officer", "Ensign", "Lieutenant", "Lt. Commander", "Post Commander",
    "Post Captain", "Rear Admiral", "Vice Admiral", "Admiral",
];
const EMPIRE_RANKS: &[&str] = &[
    "None", "Outsider", "Serf", "Master", "Squire", "Knight",
    "Lord", "Baron", "Viscount", "Count", "Earl", "Marquis", "Duke", "Prince", "King",
];
const CQC_RANKS: &[&str] = &[
    "Helpless", "Mostly Helpless", "Amateur", "Semi Professional", "Professional",
    "Champion", "Hero", "Legend", "Elite", "Elite I", "Elite II", "Elite III", "Elite IV", "Elite V",
];

fn rank_name(ranks: &'static [&'static str], value: u8) -> &'static str {
    ranks.get(value as usize).copied().unwrap_or("Unknown")
}

use edcas_common::format::credits as format_credits;

pub struct PilotView {
    scroll: u16,
}

impl PilotView {
    pub fn new() -> Self {
        Self { scroll: 0 }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> ViewEvent {
        match key.code {
            KeyCode::Up | KeyCode::Char('w') => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('s') => {
                self.scroll += 1;
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.scroll += 10;
            }
            _ => return ViewEvent::None,
        }
        ViewEvent::Consumed
    }
}

/// The installed game expansions, as shown next to "Expansions".
fn expansions(p: &PilotData) -> &'static str {
    match (p.horizons, p.odyssey) {
        (true, true) => "Horizons + Odyssey",
        (true, false) => "Horizons",
        (false, true) => "Odyssey",
        (false, false) => "Base",
    }
}

/// The ship's flight mode from Status.json flags.
fn flight_mode(s: &StatusData) -> &'static str {
    if s.docked() { "Docked" }
    else if s.landed() { "Landed" }
    else if s.supercruise() { "Supercruise" }
    else if s.in_srv() { "SRV" }
    else if s.in_fighter() { "Fighter" }
    else { "Normal Space" }
}

/// How a status flag is emphasised.
#[derive(Clone, Copy)]
enum Tone {
    Normal,
    Dim,
    Good,
    Warn,
}

/// The active Status.json flags worth surfacing, in display order.
fn status_flags(s: &StatusData) -> Vec<(&'static str, Tone)> {
    let mut flags = Vec::new();
    if s.landing_gear() { flags.push(("Gear", Tone::Normal)); }
    if s.hardpoints() { flags.push(("Hardpoints", Tone::Normal)); }
    if s.cargo_scoop() { flags.push(("Scoop", Tone::Normal)); }
    if s.lights_on() { flags.push(("Lights", Tone::Dim)); }
    if s.night_vision() { flags.push(("NightVis", Tone::Dim)); }
    if s.silent_running() { flags.push(("Silent", Tone::Warn)); }
    if s.shields_up() { flags.push(("Shields", Tone::Good)); } else { flags.push(("NO SHIELDS", Tone::Warn)); }
    if s.mass_locked() { flags.push(("MassLock", Tone::Dim)); }
    if s.fsd_charging() { flags.push(("FSD Charging", Tone::Good)); }
    if s.fsd_cooldown() { flags.push(("FSD Cooldown", Tone::Dim)); }
    if s.overheating() { flags.push(("OVERHEAT", Tone::Warn)); }
    if s.being_interdicted() { flags.push(("INTERDICTED", Tone::Warn)); }
    if s.in_danger() { flags.push(("DANGER", Tone::Warn)); }
    flags
}
