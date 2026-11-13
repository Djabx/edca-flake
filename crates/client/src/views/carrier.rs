//! The Carrier tab: the commander's own and squadron fleet carriers, found in the journals,
//! plus any carrier marked "mine" (here via search, or in Stations). For each it shows
//! location and scheduled jump, tritium and range, space, finances, crew services, trade
//! orders, cargo and jump history; the server adds the market, outfitting and shipyard.

mod gui;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use edcas_common::api::{CarrierQuery, CarrierResponse, CommodityResponse};

use crate::api_client::ApiClient;
use crate::carrier_log::{CarrierBook, CarrierKind, CarrierRecord, CarrierScanner, OrderKind, TradeOrder};
use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::JournalData;
use crate::settings::Settings;
use crate::views::ViewEvent;

/// How long the server's copy of a carrier stays fresh before it is fetched again.
const SERVER_REFRESH: Duration = Duration::from_secs(5 * 60);
/// A requested jump locks the carrier down this long before departure.
const LOCKDOWN_SECS: i64 = 200;

/// Why a carrier is in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Role {
    Own,
    Squadron,
    Marked,
}

impl Role {
    fn label(self) -> &'static str {
        match self {
            Role::Own => "Own",
            Role::Squadron => "Squadron",
            Role::Marked => "Marked",
        }
    }
}

pub(crate) struct Listed {
    pub id: i64,
    pub role: Role,
    pub title: String,
    pub callsign: Option<String>,
}

#[derive(Default)]
struct ServerEntry {
    requested: Option<Instant>,
    data: Option<CarrierResponse>,
    failed: bool,
}

type SearchSlot = Arc<Mutex<Option<Result<Vec<CarrierResponse>, String>>>>;
/// Server replies by carrier id; `None` when the request failed or the carrier is unknown.
type ServerSlot = Arc<Mutex<Vec<(i64, Option<CarrierResponse>)>>>;

pub struct CarrierView {
    scanner: Option<CarrierScanner>,
    book: Arc<CarrierBook>,
    selected: Option<i64>,
    server: HashMap<i64, ServerEntry>,
    server_rx: ServerSlot,
    search: String,
    search_rx: SearchSlot,
    search_results: Option<Result<Vec<CarrierResponse>, String>>,
    searching: bool,
    gui: gui::GuiState,
}

impl CarrierView {
    pub fn new() -> Self {
        Self {
            scanner: None,
            book: Arc::new(CarrierBook::default()),
            selected: None,
            server: HashMap::new(),
            server_rx: Arc::new(Mutex::new(Vec::new())),
            search: String::new(),
            search_rx: Arc::new(Mutex::new(None)),
            search_results: None,
            searching: false,
            gui: gui::GuiState::default(),
        }
    }

    /// Starts (or, after the journal folder changed, restarts) the journal scan. The first
    /// scan reads the whole folder, so it waits until the tab is first opened.
    pub fn on_enter(&mut self, settings: &Settings) {
        let dir = settings.journal_reader.journal_directory.trim();
        if dir.is_empty() {
            self.scanner = None;
            return;
        }
        let dir = std::path::PathBuf::from(dir);
        if self.scanner.as_ref().is_none_or(|s| s.dir != dir) {
            self.book = Arc::new(CarrierBook::default());
            self.scanner = Some(CarrierScanner::start(dir));
        }
    }

    /// Picks up new journal state, server replies and search results; asks the server for
    /// the selected carrier when its copy is stale.
    pub fn poll(&mut self, api: &ApiClient, marked: &HashSet<i64>) {
        if let Some(s) = &self.scanner {
            self.book = s.snapshot();
        }
        for (id, data) in self.server_rx.lock().unwrap().drain(..) {
            let e = self.server.entry(id).or_default();
            e.failed = data.is_none();
            if data.is_some() {
                e.data = data;
            }
        }
        if let Some(r) = self.search_rx.lock().unwrap().take() {
            self.search_results = Some(r);
            self.searching = false;
        }
        if self.scanner.is_none() {
            return;
        }
        let listed = self.listed(marked);
        if self.selected.is_none_or(|id| !listed.iter().any(|l| l.id == id)) {
            self.selected = listed.first().map(|l| l.id);
        }
        if let Some(id) = self.selected {
            self.fetch_server(api, id, false);
        }
    }

    fn fetch_server(&mut self, api: &ApiClient, id: i64, force: bool) {
        let e = self.server.entry(id).or_default();
        if !force && e.requested.is_some_and(|t| t.elapsed() < SERVER_REFRESH) {
            return;
        }
        e.requested = Some(Instant::now());
        let api2 = api.clone();
        let rx = Arc::clone(&self.server_rx);
        api.spawn(async move {
            let q = CarrierQuery { market_id: Some(id), limit: Some(1), ..Default::default() };
            let r = api2.search_carriers(&q).await.ok().map(|v| v.into_iter().next());
            // `Some(None)`: the server answered but doesn't know the carrier; keep that
            // apart from a failed request so a later refresh can still fill it in.
            let data = match r {
                Some(Some(c)) => Some(c),
                _ => None,
            };
            rx.lock().unwrap().push((id, data));
        });
    }

    pub(crate) fn start_search(&mut self, api: &ApiClient) {
        let q = self.search.trim().to_owned();
        if q.len() < 2 {
            self.search_results = None;
            return;
        }
        self.searching = true;
        let api2 = api.clone();
        let slot = Arc::clone(&self.search_rx);
        api.spawn(async move {
            let query = CarrierQuery { name: Some(q), limit: Some(25), ..Default::default() };
            let r = api2.search_carriers(&query).await.map_err(|e| e.to_string());
            *slot.lock().unwrap() = Some(r);
        });
    }

    /// Own and squadron carriers from the journals, then the ones marked "mine".
    pub(crate) fn listed(&self, marked: &HashSet<i64>) -> Vec<Listed> {
        let mut out: Vec<Listed> = self
            .book
            .managed()
            .into_iter()
            .map(|c| Listed {
                id: c.id,
                role: if c.kind == CarrierKind::Squadron { Role::Squadron } else { Role::Own },
                title: c.title(),
                callsign: c.callsign.clone(),
            })
            .collect();
        let mut extra: Vec<Listed> = marked
            .iter()
            .filter(|id| !out.iter().any(|l| l.id == **id))
            .map(|&id| {
                let server = self.server.get(&id).and_then(|e| e.data.as_ref());
                let rec = self.book.get(id);
                let callsign = rec.and_then(|r| r.callsign.clone()).or_else(|| server.map(|s| s.name.clone()));
                let title = rec
                    .and_then(|r| r.name.clone())
                    .or_else(|| server.and_then(|s| s.carrier_name.clone()))
                    .or_else(|| callsign.clone())
                    .unwrap_or_else(|| id.to_string());
                Listed { id, role: Role::Marked, title, callsign }
            })
            .collect();
        extra.sort_by(|a, b| a.title.cmp(&b.title));
        out.extend(extra);
        out
    }

    fn select_step(&mut self, marked: &HashSet<i64>, step: isize) {
        let listed = self.listed(marked);
        if listed.is_empty() {
            return;
        }
        let i = self.selected.and_then(|id| listed.iter().position(|l| l.id == id)).unwrap_or(0) as isize;
        let n = listed.len() as isize;
        self.selected = Some(listed[((i + step) % n + n) as usize % n as usize].id);
    }

    pub fn handle_key(&mut self, key: &KeyEvent, api: &ApiClient, marked: &HashSet<i64>) -> ViewEvent {
        match key.code {
            KeyCode::Char('a') | KeyCode::Left => self.select_step(marked, -1),
            KeyCode::Char('d') | KeyCode::Right => self.select_step(marked, 1),
            KeyCode::Char('r') => {
                if let Some(id) = self.selected {
                    self.fetch_server(api, id, true);
                }
            }
            _ => return ViewEvent::None,
        }
        ViewEvent::Consumed
    }

    /// Everything the panels show for the selected carrier.
    fn snapshot<'a>(&'a self, journal: &'a JournalData) -> Option<Snapshot<'a>> {
        let id = self.selected?;
        let rec = self.book.get(id);
        let entry = self.server.get(&id);
        let server = entry.and_then(|e| e.data.as_ref());
        let local = journal.local_market.as_ref().filter(|(m, _)| *m == id).map(|(_, c)| c);
        let market = match (local, server) {
            (Some(c), _) => Some(Market { items: c, at: None, local: true }),
            (None, Some(s)) if !s.commodities.is_empty() => Some(Market { items: &s.commodities, at: s.market_updated_at, local: false }),
            _ => None,
        };
        Some(Snapshot {
            id,
            rec,
            server,
            server_loading: entry.is_none_or(|e| e.data.is_none() && !e.failed),
            server_failed: entry.is_some_and(|e| e.failed && e.data.is_none()),
            market,
        })
    }
}

pub(crate) struct Market<'a> {
    pub items: &'a [CommodityResponse],
    pub at: Option<DateTime<Utc>>,
    /// From the game's own Market.json rather than the server.
    pub local: bool,
}

pub(crate) struct Snapshot<'a> {
    pub id: i64,
    pub rec: Option<&'a CarrierRecord>,
    pub server: Option<&'a CarrierResponse>,
    pub server_loading: bool,
    pub server_failed: bool,
    pub market: Option<Market<'a>>,
}

/// One trade order with what the latest market reading says is left of it.
pub(crate) struct OrderRow {
    pub name: String,
    pub kind: OrderKind,
    pub quantity: i32,
    pub price: i64,
    pub live: Option<i32>,
    pub set_at: Option<DateTime<Utc>>,
}

impl Snapshot<'_> {
    pub fn callsign(&self) -> Option<String> {
        self.rec.and_then(|r| r.callsign.clone()).or_else(|| self.server.map(|s| s.name.clone()))
    }

    /// Where the carrier is, when that was learned and from where: the journal's
    /// reading, or the server's when another commander has seen it since.
    pub fn location(&self) -> Option<(String, DateTime<Utc>, &'static str)> {
        let journal = self.rec.and_then(|r| r.location.as_ref()).map(|l| (l.body.clone().unwrap_or_else(|| l.system.clone()), l.since, "journal"));
        let server = self.server.and_then(|s| Some((s.system_name.clone(), s.updated_at?, "EDDN")));
        match (journal, server) {
            (Some(j), Some(s)) if s.1 > j.1 + chrono::TimeDelta::minutes(30) && !j.0.starts_with(&s.0) => Some(s),
            (Some(j), _) => Some(j),
            (None, s) => s,
        }
    }

    pub fn system_name(&self) -> Option<String> {
        let (place, _, src) = self.location()?;
        if src == "journal" {
            self.rec.and_then(|r| r.location.as_ref()).map(|l| l.system.clone())
        } else {
            Some(place)
        }
    }

    /// Open orders: from the journal's order log when there is one (with what the market
    /// reading says is left), else read off the market itself.
    pub fn orders(&self) -> Vec<OrderRow> {
        let name = |o: &TradeOrder| o.localised.clone().unwrap_or_else(|| prettify(&o.commodity));
        let mut rows: Vec<OrderRow> = match self.rec.filter(|r| !r.orders.is_empty()) {
            Some(r) => r
                .orders
                .values()
                .map(|o| {
                    let item = self.market.as_ref().and_then(|m| m.items.iter().find(|c| c.name.eq_ignore_ascii_case(&o.commodity)));
                    let newer = self.market.as_ref().is_some_and(|m| m.local || m.at.is_none_or(|t| t >= o.set_at));
                    let live = item.filter(|_| newer).map(|c| if o.kind == OrderKind::Sell { c.stock } else { c.demand });
                    OrderRow { name: name(o), kind: o.kind, quantity: o.quantity, price: o.price, live, set_at: Some(o.set_at) }
                })
                .collect(),
            None => self
                .market
                .iter()
                .flat_map(|m| m.items.iter())
                .filter_map(|c| {
                    if c.stock > 0 && c.buy_price > 0 {
                        Some(OrderRow { name: prettify(&c.name), kind: OrderKind::Sell, quantity: c.stock, price: c.buy_price as i64, live: Some(c.stock), set_at: None })
                    } else if c.demand > 0 && c.sell_price > 0 {
                        Some(OrderRow { name: prettify(&c.name), kind: OrderKind::Buy, quantity: c.demand, price: c.sell_price as i64, live: Some(c.demand), set_at: None })
                    } else {
                        None
                    }
                })
                .collect(),
        };
        // Orders with something left first, then by kind and name.
        rows.sort_by(|a, b| {
            (a.live == Some(0)).cmp(&(b.live == Some(0))).then((a.kind == OrderKind::Buy).cmp(&(b.kind == OrderKind::Buy))).then(a.name.cmp(&b.name))
        });
        rows
    }
}

// ---------------------------------------------------------------- formatting

pub(crate) fn thousands(n: i64) -> String {
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// Credits in a compact form: 11.88 B, 764.5 M, 12,500.
pub(crate) fn credits(n: i64) -> String {
    let a = n.unsigned_abs() as f64;
    let sign = if n < 0 { "-" } else { "" };
    if a >= 1e9 {
        format!("{sign}{:.2} B", a / 1e9)
    } else if a >= 1e6 {
        format!("{sign}{:.1} M", a / 1e6)
    } else {
        format!("{sign}{}", thousands(a as i64))
    }
}

pub(crate) fn ago(now: DateTime<Utc>, t: DateTime<Utc>) -> String {
    let d = now - t;
    if d.num_minutes() < 1 {
        "just now".into()
    } else if d.num_hours() < 1 {
        format!("{} min ago", d.num_minutes())
    } else if d.num_days() < 1 {
        format!("{} h ago", d.num_hours())
    } else if d.num_days() < 60 {
        format!("{} days ago", d.num_days())
    } else {
        t.format("%Y-%m-%d").to_string()
    }
}

/// "in 12m 05s", "lockdown · 2m 40s", "jumping…".
pub(crate) fn countdown(now: DateTime<Utc>, departure: DateTime<Utc>) -> String {
    let secs = (departure - now).num_seconds();
    if secs <= 0 {
        return "jumping…".into();
    }
    let t = if secs >= 3600 { format!("{}h {:02}m", secs / 3600, secs % 3600 / 60) } else { format!("{}m {:02}s", secs / 60, secs % 60) };
    if secs <= LOCKDOWN_SECS { format!("lockdown · departs in {t}") } else { format!("departs in {t}") }
}

pub(crate) fn in_lockdown(now: DateTime<Utc>, departure: DateTime<Utc>) -> bool {
    (0..=LOCKDOWN_SECS).contains(&(departure - now).num_seconds())
}

pub(crate) fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// A commodity key like `agronomictreatment` as a readable fallback name.
pub(crate) fn prettify(s: &str) -> String {
    capitalise(&s.replace('_', " "))
}

/// Who may dock, as the carrier panel words it.
pub(crate) fn docking_label(access: &str) -> &str {
    match access {
        "all" => "Everyone",
        "none" => "Owner only",
        "friends" => "Friends",
        "squadron" => "Squadron",
        "squadronfriends" => "Squadron and friends",
        other => other,
    }
}

/// The in-game name of a crew role / carrier service.
pub(crate) fn service_name(role: &str) -> &str {
    match role {
        "BlackMarket" => "Secure Warehouse",
        "VoucherRedemption" => "Redemption Office",
        "CarrierFuel" => "Tritium Depot",
        "VistaGenomics" => "Vista Genomics",
        "PioneerSupplies" => "Pioneer Supplies",
        "Exploration" => "Universal Cartographics",
        "Commodities" => "Commodity Market",
        "Bartender" => "Bartender",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formats() {
        assert_eq!(thousands(1_234_567), "1,234,567");
        assert_eq!(thousands(-950), "-950");
        assert_eq!(credits(11_878_563_496), "11.88 B");
        assert_eq!(credits(764_518_249), "764.5 M");
        assert_eq!(credits(12_500), "12,500");
    }

    #[test]
    fn countdown_reads_lockdown() {
        let now: DateTime<Utc> = "2026-10-12T11:00:00Z".parse().unwrap();
        assert_eq!(countdown(now, now + chrono::TimeDelta::seconds(725)), "departs in 12m 05s");
        assert_eq!(countdown(now, now + chrono::TimeDelta::seconds(160)), "lockdown · departs in 2m 40s");
        assert_eq!(countdown(now, now - chrono::TimeDelta::seconds(5)), "jumping…");
        assert!(in_lockdown(now, now + chrono::TimeDelta::seconds(160)));
    }
}
