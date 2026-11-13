//! What the journals say about the commander's fleet carriers: identity, finances, fuel,
//! crew, trade orders, location and jump history, rebuilt by replaying every `Carrier*`
//! event since the carrier was bought.
//!
//! The game only reports most of this when the carrier management panel is opened
//! (`CarrierStats`), so a single session's journal rarely has it; [`CarrierScanner`] reads
//! the whole journal folder once in the background and then follows the newest file
//! (see [`crate::journal_follow`]).

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::journal_follow::{JournalFollower, JournalLog};

/// How long after its departure time a requested jump counts as done when no later event
/// says otherwise (the jump itself takes about a minute; this leaves room for a late cancel).
const JUMP_SETTLE: chrono::TimeDelta = chrono::TimeDelta::minutes(5);
/// Balance readings kept per carrier for the chart.
const MAX_BALANCE_POINTS: usize = 2000;

/// A fleet carrier's maximum jump, and its tritium depot.
pub const MAX_JUMP_LY: f64 = 500.0;
pub const FUEL_CAPACITY: i32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CarrierKind {
    /// The commander's own carrier.
    #[default]
    Fleet,
    /// The squadron's carrier, whose panel squadron members can open.
    Squadron,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpaceUsage {
    pub total: i32,
    pub crew: i32,
    pub cargo: i32,
    pub reserved: i32,
    pub ship_packs: i32,
    pub module_packs: i32,
    pub free: i32,
}

impl SpaceUsage {
    /// Tonnage the jump fuel formula counts: everything but free space.
    pub fn used(&self) -> i32 {
        self.total - self.free
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Finance {
    pub balance: i64,
    pub reserve: i64,
    pub available: i64,
    pub reserve_percent: i32,
    /// Service → tariff in percent, e.g. ("refuel", 10).
    pub taxes: Vec<(String, i32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrewRole {
    pub role: String,
    pub activated: bool,
    pub enabled: bool,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pack {
    pub theme: String,
    pub tier: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub system: String,
    pub system_address: i64,
    pub body: Option<String>,
    pub body_id: Option<i32>,
    /// When the carrier was last known to be here.
    pub since: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingJump {
    pub to: Place,
    pub requested: DateTime<Utc>,
    pub departure: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JumpRecord {
    /// Departure time; for a jump only noticed at the next login, when it was noticed.
    pub at: DateTime<Utc>,
    /// Whether `at` is the jump's own time rather than when it was noticed.
    pub exact: bool,
    pub from: Option<String>,
    pub to: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderKind {
    /// The carrier buys: commanders sell to it.
    Buy,
    /// The carrier sells.
    Sell,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TradeOrder {
    pub commodity: String,
    pub localised: Option<String>,
    pub kind: OrderKind,
    pub quantity: i32,
    pub price: i64,
    pub black_market: bool,
    pub set_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Purchase {
    pub at: DateTime<Utc>,
    pub system: String,
    pub price: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BankTransfer {
    pub at: DateTime<Utc>,
    /// Positive for a deposit, negative for a withdrawal.
    pub amount: i64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CarrierRecord {
    pub id: i64,
    pub kind: CarrierKind,
    pub callsign: Option<String>,
    pub name: Option<String>,
    /// Seen in `CarrierStats` or `CarrierBuy`: the commander's or their squadron's carrier.
    pub managed: bool,
    pub stats_at: Option<DateTime<Utc>>,
    pub bought: Option<Purchase>,
    pub docking_access: Option<String>,
    pub allow_notorious: Option<bool>,
    pub fuel: Option<i32>,
    pub fuel_at: Option<DateTime<Utc>>,
    pub jump_range: Option<(f64, f64)>,
    pub pending_decommission: bool,
    pub decommission_at: Option<DateTime<Utc>>,
    pub space: Option<SpaceUsage>,
    pub finance: Option<Finance>,
    pub finance_at: Option<DateTime<Utc>>,
    /// (unix seconds, balance), oldest first.
    pub balance_history: Vec<(i64, i64)>,
    pub transfers: Vec<BankTransfer>,
    pub crew: Vec<CrewRole>,
    pub ship_packs: Vec<Pack>,
    pub module_packs: Vec<Pack>,
    pub location: Option<Place>,
    pub pending_jump: Option<PendingJump>,
    /// Completed jumps, oldest first.
    pub jumps: Vec<JumpRecord>,
    /// Open orders by commodity key, as last set in the carrier's market management.
    pub orders: BTreeMap<String, TradeOrder>,
}

impl CarrierRecord {
    /// Display name: the vanity name, else the callsign, else the id.
    pub fn title(&self) -> String {
        self.name.clone().or_else(|| self.callsign.clone()).unwrap_or_else(|| self.id.to_string())
    }

    pub fn decommissioned(&self, now: DateTime<Utc>) -> bool {
        self.pending_decommission && self.decommission_at.is_some_and(|t| t <= now)
    }

    fn push_balance(&mut self, at: DateTime<Utc>, balance: i64) {
        let t = at.timestamp();
        if self.balance_history.last().is_some_and(|&(lt, lb)| lt == t || (lb == balance && t - lt < 3600)) {
            if let Some(last) = self.balance_history.last_mut() {
                *last = (t.max(last.0), balance);
            }
            return;
        }
        self.balance_history.push((t, balance));
        if self.balance_history.len() > MAX_BALANCE_POINTS {
            self.balance_history.remove(0);
        }
    }

    /// Completes the requested jump once its departure is far enough behind `now`.
    fn settle(&mut self, now: DateTime<Utc>) {
        if self.pending_jump.as_ref().is_some_and(|p| now >= p.departure + JUMP_SETTLE) {
            let p = self.pending_jump.take().expect("checked above");
            self.arrive(p.departure, p.to, true);
        }
    }

    fn arrive(&mut self, at: DateTime<Utc>, to: Place, exact: bool) {
        let from = self.location.as_ref().map(|l| l.system.clone());
        // A `CarrierJump` seen aboard after the request already settled describes the
        // same jump: refresh it instead of adding another.
        let repeat = self.jumps.last().is_some_and(|j| j.to == to.system && (at - j.at).num_minutes().abs() < 30);
        if repeat {
            let last = self.jumps.last_mut().expect("checked above");
            last.at = at;
            last.exact |= exact;
            if to.body.is_some() {
                last.body = to.body.clone();
            }
        } else if from.is_some() && from.as_deref() != Some(to.system.as_str()) {
            self.jumps.push(JumpRecord { at, exact, from, to: to.system.clone(), body: to.body.clone() });
        }
        self.location = Some(Place { since: at, ..to });
    }
}

/// Tritium a fleet carrier's jump of `ly` takes with `used` tonnes aboard and `fuel` in the
/// depot (the depot's own tritium counts as mass). Checked against the commander's own jump
/// log; squadron carriers burn noticeably less and don't fit it.
pub fn jump_fuel(ly: f64, used: i32, fuel: i32) -> i32 {
    (5.0 + ly * (25_000.0 + used as f64 + fuel as f64) / 200_000.0).round() as i32
}

/// How many full-range jumps the depot covers, burning the fuel (and its mass) as it goes.
pub fn max_range_jumps(used: i32, fuel: i32) -> u32 {
    let mut fuel = fuel;
    let mut n = 0;
    loop {
        let need = jump_fuel(MAX_JUMP_LY, used, fuel);
        if need > fuel || n >= 100 {
            return n;
        }
        fuel -= need;
        n += 1;
    }
}

/// Every carrier the journals mention, by carrier id (which is also its market id).
#[derive(Debug, Clone, Default)]
pub struct CarrierBook {
    pub carriers: HashMap<i64, CarrierRecord>,
    /// Whether the first full read of the journal folder has finished.
    pub loaded: bool,
}

fn ts(v: &Value) -> Option<DateTime<Utc>> {
    v["timestamp"].as_str()?.parse().ok()
}

fn s(v: &Value, k: &str) -> Option<String> {
    v[k].as_str().filter(|x| !x.is_empty()).map(str::to_owned)
}

fn kind_of(v: &Value) -> Option<CarrierKind> {
    match v["CarrierType"].as_str()? {
        "SquadronCarrier" => Some(CarrierKind::Squadron),
        _ => Some(CarrierKind::Fleet),
    }
}

fn packs(v: &Value) -> Vec<Pack> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|p| Pack {
                    theme: p["PackTheme"].as_str().unwrap_or("?").to_owned(),
                    tier: p["PackTier"].as_i64().unwrap_or(0) as i32,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn taxes(v: &Value) -> Vec<(String, i32)> {
    let mut out: Vec<(String, i32)> = v
        .as_object()
        .map(|o| {
            o.iter()
                .filter_map(|(k, x)| Some((k.strip_prefix("TaxRate_")?.to_owned(), x.as_i64()? as i32)))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

impl CarrierBook {
    /// Applies one journal event; anything that isn't about a carrier is ignored.
    pub fn apply(&mut self, v: &Value) {
        let Some(event) = v["event"].as_str() else { return };
        if !event.starts_with("Carrier") {
            return;
        }
        let Some(at) = ts(v) else { return };
        let id = match v["CarrierID"].as_i64() {
            Some(id) => id,
            // `CarrierJump` names the carrier by its market id, and not at all on foot.
            None if event == "CarrierJump" => match v["MarketID"].as_i64().or_else(|| self.jump_target(v)) {
                Some(id) => id,
                None => return,
            },
            None => return,
        };
        let rec = self.carriers.entry(id).or_insert_with(|| CarrierRecord { id, ..Default::default() });
        rec.settle(at);
        if let Some(k) = kind_of(v) {
            rec.kind = k;
        }
        match event {
            "CarrierStats" => {
                rec.managed = true;
                rec.stats_at = Some(at);
                rec.callsign = s(v, "Callsign").or(rec.callsign.take());
                rec.name = s(v, "Name").or(rec.name.take());
                rec.docking_access = s(v, "DockingAccess");
                rec.allow_notorious = v["AllowNotorious"].as_bool();
                if let Some(f) = v["FuelLevel"].as_i64() {
                    rec.fuel = Some(f as i32);
                    rec.fuel_at = Some(at);
                }
                if let (Some(c), Some(m)) = (v["JumpRangeCurr"].as_f64(), v["JumpRangeMax"].as_f64()) {
                    rec.jump_range = Some((c, m));
                }
                rec.pending_decommission = v["PendingDecommission"].as_bool().unwrap_or(false);
                let su = &v["SpaceUsage"];
                if su.is_object() {
                    let g = |k: &str| su[k].as_i64().unwrap_or(0) as i32;
                    rec.space = Some(SpaceUsage {
                        total: g("TotalCapacity"),
                        crew: g("Crew"),
                        cargo: g("Cargo"),
                        reserved: g("CargoSpaceReserved"),
                        ship_packs: g("ShipPacks"),
                        module_packs: g("ModulePacks"),
                        free: g("FreeSpace"),
                    });
                }
                if v["Finance"].is_object() {
                    rec.apply_finance(&v["Finance"], at);
                }
                if let Some(crew) = v["Crew"].as_array() {
                    rec.crew = crew
                        .iter()
                        .map(|c| CrewRole {
                            role: c["CrewRole"].as_str().unwrap_or("?").to_owned(),
                            activated: c["Activated"].as_bool().unwrap_or(false),
                            enabled: c["Enabled"].as_bool().unwrap_or(false),
                            name: s(c, "CrewName"),
                        })
                        .collect();
                }
                rec.ship_packs = packs(&v["ShipPacks"]);
                rec.module_packs = packs(&v["ModulePacks"]);
            }
            "CarrierBuy" => {
                rec.managed = true;
                rec.kind = kind_of(v).unwrap_or(CarrierKind::Fleet);
                rec.callsign = s(v, "Callsign").or(rec.callsign.take());
                rec.bought = Some(Purchase {
                    at,
                    system: s(v, "Location").unwrap_or_default(),
                    price: v["Price"].as_i64().unwrap_or(0),
                });
                if let (Some(system), Some(addr)) = (s(v, "Location"), v["SystemAddress"].as_i64()) {
                    rec.location = Some(Place { system, system_address: addr, body: None, body_id: None, since: at });
                }
            }
            "CarrierFinance" => rec.apply_finance(v, at),
            "CarrierBankTransfer" => {
                let amount = v["Deposit"].as_i64().unwrap_or(0) - v["Withdraw"].as_i64().unwrap_or(0);
                rec.transfers.push(BankTransfer { at, amount });
                if let Some(b) = v["CarrierBalance"].as_i64() {
                    let f = rec.finance.get_or_insert_with(Finance::default);
                    f.available += b - f.balance;
                    f.balance = b;
                    rec.finance_at = Some(at);
                    rec.push_balance(at, b);
                }
            }
            "CarrierDepositFuel" => {
                if let Some(t) = v["Total"].as_i64() {
                    rec.fuel = Some(t as i32);
                    rec.fuel_at = Some(at);
                }
            }
            "CarrierDockingPermission" => {
                rec.docking_access = s(v, "DockingAccess").or(rec.docking_access.take());
                rec.allow_notorious = v["AllowNotorious"].as_bool().or(rec.allow_notorious);
            }
            "CarrierCrewServices" => {
                let role = v["CrewRole"].as_str().unwrap_or("?");
                let i = match rec.crew.iter().position(|c| c.role == role) {
                    Some(i) => i,
                    None => {
                        rec.crew.push(CrewRole { role: role.to_owned(), activated: false, enabled: false, name: None });
                        rec.crew.len() - 1
                    }
                };
                let c = &mut rec.crew[i];
                match v["Operation"].as_str().unwrap_or("") {
                    "Activate" => {
                        c.activated = true;
                        c.enabled = true;
                    }
                    "Deactivate" => {
                        c.activated = false;
                        c.enabled = false;
                    }
                    "Pause" => c.enabled = false,
                    "Resume" => c.enabled = true,
                    _ => {}
                }
                if let Some(n) = s(v, "CrewName") {
                    c.name = Some(n);
                }
            }
            "CarrierModulePack" | "CarrierShipPack" => {
                let list = if event == "CarrierModulePack" { &mut rec.module_packs } else { &mut rec.ship_packs };
                let theme = v["PackTheme"].as_str().unwrap_or("?").to_owned();
                let tier = v["PackTier"].as_i64().unwrap_or(0) as i32;
                match v["Operation"].as_str().unwrap_or("") {
                    "BuyPack" => list.push(Pack { theme, tier }),
                    "SellPack" => {
                        if let Some(i) = list.iter().position(|p| p.theme == theme && p.tier == tier) {
                            list.remove(i);
                        }
                    }
                    _ => {}
                }
            }
            "CarrierTradeOrder" => {
                let Some(commodity) = v["Commodity"].as_str().map(str::to_lowercase) else { return };
                let kind = if v["PurchaseOrder"].is_i64() {
                    Some((OrderKind::Buy, v["PurchaseOrder"].as_i64().unwrap_or(0)))
                } else if v["SaleOrder"].is_i64() {
                    Some((OrderKind::Sell, v["SaleOrder"].as_i64().unwrap_or(0)))
                } else {
                    None
                };
                match kind {
                    Some((kind, quantity)) if !v["CancelTrade"].as_bool().unwrap_or(false) => {
                        rec.orders.insert(
                            commodity.clone(),
                            TradeOrder {
                                commodity,
                                localised: s(v, "Commodity_Localised"),
                                kind,
                                quantity: quantity as i32,
                                price: v["Price"].as_i64().unwrap_or(0),
                                black_market: v["BlackMarket"].as_bool().unwrap_or(false),
                                set_at: at,
                            },
                        );
                    }
                    _ => {
                        rec.orders.remove(&commodity);
                    }
                }
            }
            "CarrierNameChange" => {
                rec.name = s(v, "Name").or(rec.name.take());
                rec.callsign = s(v, "Callsign").or(rec.callsign.take());
            }
            "CarrierDecommission" => {
                rec.pending_decommission = true;
                rec.decommission_at = v["ScrapTime"].as_i64().and_then(|t| DateTime::from_timestamp(t, 0));
            }
            "CarrierCancelDecommission" => {
                rec.pending_decommission = false;
                rec.decommission_at = None;
            }
            "CarrierJumpRequest" => {
                let (Some(system), Some(addr)) = (s(v, "SystemName"), v["SystemAddress"].as_i64()) else { return };
                // Older journals have no departure time; the countdown was 15 minutes then.
                let departure = v["DepartureTime"]
                    .as_str()
                    .and_then(|d| d.parse().ok())
                    .unwrap_or(at + chrono::TimeDelta::minutes(15));
                rec.pending_jump = Some(PendingJump {
                    to: Place {
                        system,
                        system_address: addr,
                        body: s(v, "Body"),
                        body_id: v["BodyID"].as_i64().map(|b| b as i32),
                        since: departure,
                    },
                    requested: at,
                    departure,
                });
            }
            "CarrierJumpCancelled" => rec.pending_jump = None,
            "CarrierLocation" => {
                let (Some(system), Some(addr)) = (s(v, "StarSystem"), v["SystemAddress"].as_i64()) else { return };
                let body_id = v["BodyID"].as_i64().map(|b| b as i32);
                if rec.pending_jump.as_ref().is_some_and(|p| p.to.system_address == addr) {
                    let p = rec.pending_jump.take().expect("checked above");
                    rec.arrive(p.departure.min(at), p.to, true);
                } else if rec.location.as_ref().is_none_or(|l| l.system_address != addr) {
                    rec.arrive(at, Place { system, system_address: addr, body: None, body_id, since: at }, false);
                } else if let Some(l) = rec.location.as_mut() {
                    if body_id.is_some() && l.body_id != body_id {
                        l.body_id = body_id;
                        l.body = None;
                    }
                }
            }
            "CarrierJump" => {
                let (Some(system), Some(addr)) = (s(v, "StarSystem"), v["SystemAddress"].as_i64()) else { return };
                let pending = rec.pending_jump.take().filter(|p| p.to.system_address == addr);
                let body = s(v, "Body").or_else(|| pending.as_ref().and_then(|p| p.to.body.clone()));
                let body_id = v["BodyID"].as_i64().map(|b| b as i32);
                rec.arrive(at, Place { system, system_address: addr, body, body_id, since: at }, true);
            }
            _ => {}
        }
    }

    /// The carrier a `CarrierJump` without a market id belongs to: the one whose requested
    /// jump goes to that system.
    fn jump_target(&self, v: &Value) -> Option<i64> {
        let addr = v["SystemAddress"].as_i64()?;
        self.carriers
            .values()
            .find(|c| c.pending_jump.as_ref().is_some_and(|p| p.to.system_address == addr))
            .map(|c| c.id)
    }

    /// Completes every requested jump whose departure is behind `now`.
    pub fn settle(&mut self, now: DateTime<Utc>) {
        for c in self.carriers.values_mut() {
            c.settle(now);
        }
    }

    /// The commander's own and squadron carriers, own first, then by name.
    pub fn managed(&self) -> Vec<&CarrierRecord> {
        let now = Utc::now();
        let mut out: Vec<&CarrierRecord> = self.carriers.values().filter(|c| c.managed).collect();
        out.sort_by_key(|c| (c.decommissioned(now), c.kind != CarrierKind::Fleet, c.title()));
        out
    }

    pub fn get(&self, id: i64) -> Option<&CarrierRecord> {
        self.carriers.get(&id)
    }
}

impl CarrierRecord {
    fn apply_finance(&mut self, f: &Value, at: DateTime<Utc>) {
        let g = |k: &str| f[k].as_i64();
        let Some(balance) = g("CarrierBalance") else { return };
        self.finance = Some(Finance {
            balance,
            reserve: g("ReserveBalance").unwrap_or(0),
            available: g("AvailableBalance").unwrap_or(balance),
            reserve_percent: g("ReservePercent").unwrap_or(0) as i32,
            taxes: taxes(f),
        });
        self.finance_at = Some(at);
        self.push_balance(at, balance);
    }
}

// ---------------------------------------------------------------- scanner


/// Follows the journal folder for the [`CarrierBook`].
pub type CarrierScanner = JournalFollower<CarrierBook>;

impl JournalLog for CarrierBook {
    const NAME: &'static str = "carrier-scan";
    const EVENTS: &'static [&'static [u8]] = &[b"\"event\":\"Carrier"];

    fn apply(&mut self, v: &Value) {
        CarrierBook::apply(self, v);
    }

    fn settle(&mut self, now: DateTime<Utc>) -> bool {
        let pending = |b: &Self| b.carriers.values().filter(|c| c.pending_jump.is_some()).count();
        let before = pending(self);
        CarrierBook::settle(self, now);
        before != pending(self)
    }

    fn set_loaded(&mut self) {
        self.loaded = true;
    }

    fn summary(&self) -> String {
        format!("{} carriers ({} managed)", self.carriers.len(), self.carriers.values().filter(|c| c.managed).count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal_follow::feed;

    fn book(lines: &[&str]) -> CarrierBook {
        let mut b = CarrierBook::default();
        let joined = lines.join("\n") + "\n";
        feed(&mut b, joined.as_bytes());
        b
    }

    const STATS: &str = r#"{ "timestamp":"2026-08-30T10:23:55Z", "event":"CarrierStats", "CarrierID":7, "CarrierType":"FleetCarrier", "Callsign":"Q2K-BHB", "Name":"FUXBAU", "DockingAccess":"all", "AllowNotorious":false, "FuelLevel":900, "JumpRangeCurr":500.0, "JumpRangeMax":500.0, "PendingDecommission":false, "SpaceUsage":{ "TotalCapacity":25000, "Crew":1030, "Cargo":2000, "CargoSpaceReserved":500, "ShipPacks":0, "ModulePacks":0, "FreeSpace":21470 }, "Finance":{ "CarrierBalance":1000, "ReserveBalance":100, "AvailableBalance":900, "ReservePercent":5, "TaxRate_refuel":10 }, "Crew":[ { "CrewRole":"Refuel", "Activated":true, "Enabled":true, "CrewName":"Donna Moon" }, { "CrewRole":"Shipyard", "Activated":false } ], "ShipPacks":[], "ModulePacks":[] }"#;

    #[test]
    fn stats_make_a_managed_carrier() {
        let b = book(&[STATS]);
        let c = b.get(7).unwrap();
        assert!(c.managed);
        assert_eq!(c.title(), "FUXBAU");
        assert_eq!(c.fuel, Some(900));
        assert_eq!(c.space.as_ref().unwrap().used(), 3530);
        assert_eq!(c.finance.as_ref().unwrap().taxes, vec![("refuel".to_owned(), 10)]);
        assert_eq!(c.crew.len(), 2);
    }

    #[test]
    fn requested_jump_settles_after_departure_and_a_cancel_drops_it() {
        let b = book(&[
            STATS,
            r#"{ "timestamp":"2026-08-30T11:00:00Z", "event":"CarrierLocation", "CarrierType":"FleetCarrier", "CarrierID":7, "StarSystem":"Home", "SystemAddress":1, "BodyID":0 }"#,
            r#"{ "timestamp":"2026-08-30T12:00:00Z", "event":"CarrierJumpRequest", "CarrierType":"FleetCarrier", "CarrierID":7, "SystemName":"Away", "Body":"Away A", "SystemAddress":2, "BodyID":1, "DepartureTime":"2026-08-30T12:15:00Z" }"#,
            r#"{ "timestamp":"2026-08-30T13:00:00Z", "event":"CarrierJumpRequest", "CarrierType":"FleetCarrier", "CarrierID":7, "SystemName":"Third", "SystemAddress":3, "BodyID":0, "DepartureTime":"2026-08-30T13:15:00Z" }"#,
            r#"{ "timestamp":"2026-08-30T13:05:00Z", "event":"CarrierJumpCancelled", "CarrierType":"FleetCarrier", "CarrierID":7 }"#,
        ]);
        let c = b.get(7).unwrap();
        assert_eq!(c.location.as_ref().unwrap().system, "Away");
        assert_eq!(c.location.as_ref().unwrap().body.as_deref(), Some("Away A"));
        assert!(c.pending_jump.is_none());
        assert_eq!(c.jumps.len(), 1, "{:?}", c.jumps);
        assert_eq!(c.jumps[0].from.as_deref(), Some("Home"));
        assert!(c.jumps[0].exact);
    }

    #[test]
    fn jump_seen_aboard_does_not_double_count() {
        let b = book(&[
            STATS,
            r#"{ "timestamp":"2026-08-30T11:00:00Z", "event":"CarrierLocation", "CarrierID":7, "StarSystem":"Home", "SystemAddress":1, "BodyID":0 }"#,
            r#"{ "timestamp":"2026-08-30T12:00:00Z", "event":"CarrierJumpRequest", "CarrierID":7, "SystemName":"Away", "SystemAddress":2, "BodyID":1, "DepartureTime":"2026-08-30T12:15:00Z" }"#,
            r#"{ "timestamp":"2026-08-30T12:16:10Z", "event":"CarrierJump", "Docked":false, "StarSystem":"Away", "SystemAddress":2, "Body":"Away A", "BodyID":1 }"#,
            r#"{ "timestamp":"2026-08-30T14:00:00Z", "event":"CarrierStats", "CarrierID":7, "Callsign":"Q2K-BHB", "FuelLevel":800 }"#,
        ]);
        let c = b.get(7).unwrap();
        assert_eq!(c.jumps.len(), 1);
        assert_eq!(c.location.as_ref().unwrap().body.as_deref(), Some("Away A"));
        assert_eq!(c.fuel, Some(800));
    }

    #[test]
    fn trade_orders_follow_set_and_cancel() {
        let b = book(&[
            r#"{ "timestamp":"2022-08-12T22:57:37Z", "event":"CarrierTradeOrder", "CarrierID":7, "BlackMarket":false, "Commodity":"bauxite", "Commodity_Localised":"Bauxit", "SaleOrder":19191, "Price":1500 }"#,
            r#"{ "timestamp":"2022-08-12T22:57:49Z", "event":"CarrierTradeOrder", "CarrierID":7, "BlackMarket":false, "Commodity":"gold", "PurchaseOrder":100, "Price":40000 }"#,
            r#"{ "timestamp":"2022-08-13T10:00:00Z", "event":"CarrierTradeOrder", "CarrierID":7, "BlackMarket":false, "Commodity":"bauxite", "CancelTrade":true }"#,
        ]);
        let c = b.get(7).unwrap();
        assert_eq!(c.orders.len(), 1);
        let gold = &c.orders["gold"];
        assert_eq!((gold.kind, gold.quantity, gold.price), (OrderKind::Buy, 100, 40000));
        assert!(!c.managed, "orders alone don't say whose carrier it is");
    }

    #[test]
    fn fuel_formula_matches_logged_jumps() {
        // From the commander's journal: 481.5 ly with 16 915 t used and ~868 t of tritium
        // burned 108 t; 499.8 ly with 23 296 t and ~924 t burned 128 t.
        assert_eq!(jump_fuel(481.5, 16_915, 868), 108);
        assert_eq!(jump_fuel(499.8, 23_296, 924), 128);
        assert_eq!(jump_fuel(0.0, 25_000, 1000), 5);
        // An empty carrier with a full depot manages 14 full jumps.
        assert_eq!(max_range_jumps(0, 1000), 14);
        assert_eq!(max_range_jumps(0, 0), 0);
    }

    #[test]
    fn partial_line_waits_for_its_newline() {
        let mut b = CarrierBook::default();
        let (used, _) = feed(&mut b, STATS.as_bytes());
        assert_eq!(used, 0);
        let with_nl = format!("{STATS}\n{{\"event\":\"Carr");
        let (used, changed) = feed(&mut b, with_nl.as_bytes());
        assert_eq!(used, STATS.len() + 1);
        assert!(changed);
    }
}
