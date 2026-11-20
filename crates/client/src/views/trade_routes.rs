mod gui;

use crate::api_client::ApiClient;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::JournalData;
use crate::views::ViewEvent;
use edcas_common::api::{TradeLoopResponse, TradeRouteResponse, TradeSearchQuery, TradeSearchResponse};
use std::time::{Duration, Instant};

const TYPE_OPTIONS: &[&str] = &["Routes", "Loops"];
const SUPERPOWER_OPTIONS: &[&str] = &["All", "Federation", "Empire", "Alliance", "Independent"];
const INCLUDE_OPTIONS: &[&str] = &["Exclude", "Include"];
const PAD_OPTIONS: &[&str] = &["S", "M", "L"];
/// Max distance from the arrival star; `None` = any.
const ARRIVAL_OPTIONS: &[(Option<f32>, &str)] =
    &[(None, "Any"), (Some(1_000.0), "1k Ls"), (Some(5_000.0), "5k Ls"), (Some(10_000.0), "10k Ls"), (Some(50_000.0), "50k Ls")];
/// Max price age in days.
const AGE_OPTIONS: &[(i32, &str)] = &[(1, "1 day"), (3, "3 days"), (7, "7 days"), (14, "14 days"), (30, "30 days"), (90, "90 days")];
const RADIUS_STEP: f32 = 10.0;
const MIN_RADIUS: f32 = 10.0;
const MAX_RADIUS: f32 = 200.0;
/// A filter change waits this long for further changes before it searches.
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(700);

/// Keyboard order of the filter fields (w/s moves, ←/→ changes).
mod field {
    pub const TYPE: usize = 0;
    pub const NEAR: usize = 1;
    pub const RADIUS: usize = 2;
    pub const PAD: usize = 3;
    pub const SUPERPOWER: usize = 4;
    pub const MIN_SUPPLY: usize = 5;
    pub const MIN_DEMAND: usize = 6;
    pub const MAX_AGE: usize = 7;
    pub const ARRIVAL: usize = 8;
    pub const PLANETARY: usize = 9;
    pub const SETTLEMENTS: usize = 10;
    pub const CARRIERS: usize = 11;
    pub const LAST: usize = CARRIERS;
}

#[derive(Clone, Copy, PartialEq)]
enum FocusArea {
    Filter,
    List,
    Detail,
}

#[derive(Clone, Copy, PartialEq)]
enum RouteType {
    Routes,
    Loops,
}

pub struct TradeRoutesView {
    focus: FocusArea,
    filter_field: usize,
    type_idx: usize,
    superpower_idx: usize,
    min_supply: i32,
    min_demand: i32,
    /// Fleet carrier routes are opt-in: most are price traps of carriers that
    /// only let their own squadron dock.
    include_carriers: bool,
    include_settlements: bool,
    include_planetary: bool,
    /// The system to search around. Takes over the commander's location on entering
    /// the tab (and with "Here") until they type another one.
    near: String,
    near_typed: bool,
    radius: f32,
    pad_idx: usize,
    arrival_idx: usize,
    age_idx: usize,
    /// When a debounced filter change runs its search.
    search_due: Option<Instant>,
    search_rx: Option<std::sync::mpsc::Receiver<Result<TradeSearchResponse, String>>>,
    /// What the last search covered: centre, radius, stations, whether the station cap cut it.
    searched: Option<(String, f32, i64, bool)>,
    /// Why the last search failed, shown in place of the empty list.
    search_error: Option<String>,

    routes: Vec<TradeRouteResponse>,
    loops: Vec<TradeLoopResponse>,
    selected_idx: usize,
    detail_cursor: usize,
    status_msg: String,

    route_rx: Option<std::sync::mpsc::Receiver<Result<Vec<TradeRouteResponse>, String>>>,
    loop_rx: Option<std::sync::mpsc::Receiver<Result<Vec<TradeLoopResponse>, String>>>,
    clipboard: Option<arboard::Clipboard>,
}

impl TradeRoutesView {
    pub fn new() -> Self {
        Self {
            focus: FocusArea::Filter,
            filter_field: 0,
            type_idx: 0,
            superpower_idx: 0,
            min_supply: 10_000,
            min_demand: 0,
            include_carriers: false,
            include_settlements: true,
            include_planetary: true,
            near: String::new(),
            near_typed: false,
            radius: 50.0,
            pad_idx: 2,
            arrival_idx: 0,
            age_idx: 4,
            search_due: None,
            search_rx: None,
            searched: None,
            search_error: None,
            routes: Vec::new(),
            loops: Vec::new(),
            selected_idx: 0,
            detail_cursor: 0,
            status_msg: "Loading pre-computed trade data…".into(),
            route_rx: None,
            loop_rx: None,
            clipboard: arboard::Clipboard::new().ok(),
        }
    }

    fn route_type(&self) -> RouteType {
        if self.type_idx == 1 { RouteType::Loops } else { RouteType::Routes }
    }

    pub fn on_enter(&mut self, api: &ApiClient, journal: &JournalData) {
        let moved = self.follow_location(journal);
        if moved || (self.routes.is_empty() && self.loops.is_empty()) {
            self.fetch_all(api, journal);
        }
    }

    /// Takes over the commander's current system as the search centre, unless they
    /// typed one. Returns whether the centre changed.
    fn follow_location(&mut self, journal: &JournalData) -> bool {
        if self.near_typed {
            return false;
        }
        match journal.current_system.as_ref().map(|s| s.name.as_str()).filter(|n| !n.is_empty()) {
            Some(name) if name != self.near => {
                self.near = name.to_string();
                true
            }
            _ => false,
        }
    }

    /// Searches around [`Self::near`], or loads the pre-computed bubble-wide lists
    /// when no system is known yet.
    fn fetch_all(&mut self, api: &ApiClient, journal: &JournalData) {
        self.search_due = None;
        if self.near.trim().is_empty() {
            self.fetch_routes(api);
            self.fetch_loops(api);
        } else {
            self.search(api, journal);
        }
    }

    /// Runs the search after [`SEARCH_DEBOUNCE`], so a burst of filter clicks searches once.
    fn schedule_search(&mut self) {
        self.search_due = Some(Instant::now() + SEARCH_DEBOUNCE);
    }

    /// Runs a scheduled search once it is due; call every frame. The location is
    /// not followed here: a commander jumping with the tab open would search on
    /// every jump. It is taken over on entering the tab, or with "Here".
    pub fn tick(&mut self, api: &ApiClient, journal: &JournalData) {
        if self.search_due.is_some_and(|t| Instant::now() >= t) {
            self.fetch_all(api, journal);
        }
    }

    fn search_query(&self, journal: &JournalData) -> TradeSearchQuery {
        let near = self.near.trim().to_string();
        // Coordinates help when the server has not seen this system yet.
        let pos = journal
            .current_system
            .as_ref()
            .filter(|s| s.name.eq_ignore_ascii_case(&near))
            .and_then(|s| s.star_pos);
        TradeSearchQuery {
            system: near,
            x: pos.map(|p| p[0]),
            y: pos.map(|p| p[1]),
            z: pos.map(|p| p[2]),
            radius: Some(self.radius),
            pad: Some(PAD_OPTIONS[self.pad_idx].to_string()),
            carriers: Some(self.include_carriers),
            settlements: Some(self.include_settlements),
            planetary: Some(self.include_planetary),
            max_arrival_ls: ARRIVAL_OPTIONS[self.arrival_idx].0,
            max_age_days: Some(AGE_OPTIONS[self.age_idx].0),
            min_supply: Some(self.min_supply.max(1)),
            min_demand: Some(self.min_demand),
        }
    }

    fn search(&mut self, api: &ApiClient, journal: &JournalData) {
        let query = self.search_query(journal);
        let (tx, rx) = std::sync::mpsc::channel();
        self.search_rx = Some(rx);
        let api_owned = api.clone();
        api.spawn(async move {
            let _ = tx.send(api_owned.trade_search(&query).await.map_err(|e| e.to_string()));
        });
        self.status_msg = format!("Searching near {}…", self.near.trim());
    }

    fn loading(&self) -> bool {
        self.route_rx.is_some() || self.loop_rx.is_some() || self.search_rx.is_some()
    }

    fn passes_route_filter(&self, r: &TradeRouteResponse) -> bool {
        if r.supply < self.min_supply { return false; }
        if r.demand < self.min_demand { return false; }
        if self.superpower_idx == 0 { return true; }
        let sp = SUPERPOWER_OPTIONS[self.superpower_idx];
        let from_ok = r.from_allegiance.is_none() || r.from_allegiance.as_deref() == Some(sp);
        let to_ok   = r.to_allegiance.is_none()   || r.to_allegiance.as_deref()   == Some(sp);
        from_ok && to_ok
    }

    fn passes_loop_filter(&self, l: &TradeLoopResponse) -> bool {
        if l.supply_out < self.min_supply || l.supply_back < self.min_supply { return false; }
        if l.demand_out < self.min_demand || l.demand_back < self.min_demand { return false; }
        if self.superpower_idx == 0 { return true; }
        let sp = SUPERPOWER_OPTIONS[self.superpower_idx];
        let a_ok = l.allegiance_a.is_none() || l.allegiance_a.as_deref() == Some(sp);
        let b_ok = l.allegiance_b.is_none() || l.allegiance_b.as_deref() == Some(sp);
        a_ok && b_ok
    }

    fn filtered_routes(&self) -> Vec<&TradeRouteResponse> {
        self.routes.iter().filter(|r| self.passes_route_filter(r)).collect()
    }

    fn filtered_loops(&self) -> Vec<&TradeLoopResponse> {
        self.loops.iter().filter(|l| self.passes_loop_filter(l)).collect()
    }

    fn fetch_routes(&mut self, api: &ApiClient) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.route_rx = Some(rx);
        let api_owned = api.clone();
        let carriers = self.include_carriers;
        api.spawn(async move {
            let _ = tx.send(api_owned.fetch_trade_routes(carriers).await.map_err(|e| e.to_string()));
        });
    }

    fn fetch_loops(&mut self, api: &ApiClient) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.loop_rx = Some(rx);
        let api_owned = api.clone();
        let carriers = self.include_carriers;
        api.spawn(async move {
            let _ = tx.send(api_owned.fetch_trade_loops(carriers).await.map_err(|e| e.to_string()));
        });
        self.status_msg = "Loading…".into();
    }

    pub fn poll_results(&mut self) {
        if let Some(rx) = &self.search_rx {
            if let Ok(outcome) = rx.try_recv() {
                self.search_rx = None;
                match outcome {
                    Ok(r) => {
                        self.search_error = None;
                        self.searched = Some((r.center_system.clone(), r.radius_ly, r.stations, r.capped));
                        self.routes = r.routes;
                        self.loops = r.loops;
                        self.reset_list();
                        self.refresh_status();
                    }
                    Err(e) => {
                        // Nothing on screen may look like an answer for this search.
                        self.routes.clear();
                        self.loops.clear();
                        self.searched = None;
                        self.reset_list();
                        self.status_msg = format!("Search failed: {e}");
                        self.search_error = Some(e);
                    }
                }
            }
        }
        if let Some(rx) = &self.route_rx {
            if let Ok(outcome) = rx.try_recv() {
                self.route_rx = None;
                match outcome {
                    Ok(r) => {
                        self.routes = r;
                        self.refresh_status();
                    }
                    Err(e) => self.status_msg = format!("Routes error: {e}"),
                }
            }
        }
        if let Some(rx) = &self.loop_rx {
            if let Ok(outcome) = rx.try_recv() {
                self.loop_rx = None;
                match outcome {
                    Ok(l) => {
                        self.loops = l;
                        self.refresh_status();
                    }
                    Err(e) => self.status_msg = format!("Loops error: {e}"),
                }
            }
        }
    }

    fn refresh_status(&mut self) {
        let r = self.routes.len();
        let l = self.loops.len();
        if r > 0 || l > 0 {
            let scope = match &self.searched {
                Some((c, radius, n, capped)) if !self.near.trim().is_empty() => {
                    let cap = if *capped { " (nearest stations only)" } else { "" };
                    format!("near {c}, {radius:.0} Ly{cap}, {} stations", format_num(*n as i32))
                }
                _ => "bubble-wide, pre-computed".to_string(),
            };
            self.status_msg = format!(
                "{r} routes  {l} loops  {scope}  |  w/s: navigate  |  Tab: filters  |  c: copy systems  |  r: refresh"
            );
            if self.focus != FocusArea::Filter {
                self.focus = FocusArea::List;
            }
        } else if self.near.trim().is_empty() {
            self.status_msg = "No data yet — cache may still be computing".into();
        } else {
            self.status_msg = format!("No routes near {} with these filters — widen the radius or relax a filter", self.near.trim());
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent, api: &ApiClient, journal: &JournalData) -> ViewEvent {
        match self.focus {
            FocusArea::Filter => self.handle_filter_key(key, api, journal),
            FocusArea::List   => self.handle_list_key(key, api, journal),
            FocusArea::Detail => self.handle_detail_key(key),
        }
    }

    fn handle_filter_key(&mut self, key: &KeyEvent, api: &ApiClient, journal: &JournalData) -> ViewEvent {
        match key.code {
            KeyCode::Tab => {
                self.focus = FocusArea::List;
                return ViewEvent::Consumed;
            }
            KeyCode::Char('s') | KeyCode::Down => {
                if self.filter_field < field::LAST {
                    self.filter_field += 1;
                } else {
                    self.focus = FocusArea::List;
                }
                return ViewEvent::Consumed;
            }
            KeyCode::Char('w') | KeyCode::Up => {
                if self.filter_field > 0 {
                    self.filter_field -= 1;
                }
                return ViewEvent::Consumed;
            }
            KeyCode::Right | KeyCode::Left => {
                let forward = key.code == KeyCode::Right;
                self.step_filter(self.filter_field, forward, api, journal);
                return ViewEvent::Consumed;
            }
            _ => {}
        }
        ViewEvent::None
    }

    fn handle_list_key(&mut self, key: &KeyEvent, api: &ApiClient, journal: &JournalData) -> ViewEvent {
        let count = self.current_count();
        match key.code {
            KeyCode::Tab => {
                self.focus = FocusArea::Detail;
                return ViewEvent::Consumed;
            }
            KeyCode::Up | KeyCode::Char('w') => {
                if self.selected_idx > 0 {
                    self.selected_idx -= 1;
                } else {
                    self.focus = FocusArea::Filter;
                }
                return ViewEvent::Consumed;
            }
            KeyCode::Down | KeyCode::Char('s') => {
                if count > 0 {
                    self.selected_idx = (self.selected_idx + 1).min(count - 1);
                }
                return ViewEvent::Consumed;
            }
            KeyCode::PageUp => {
                self.selected_idx = self.selected_idx.saturating_sub(10);
                return ViewEvent::Consumed;
            }
            KeyCode::PageDown => {
                if count > 0 {
                    self.selected_idx = (self.selected_idx + 10).min(count - 1);
                }
                return ViewEvent::Consumed;
            }
            KeyCode::Char('r') => {
                self.fetch_all(api, journal);
                return ViewEvent::Consumed;
            }
            KeyCode::Char('c') => {
                let text = match self.route_type() {
                    RouteType::Routes => self
                        .filtered_routes()
                        .into_iter()
                        .nth(self.selected_idx)
                        .map(|r| format!("{} → {}", r.from_system_name, r.to_system_name)),
                    RouteType::Loops => self
                        .filtered_loops()
                        .into_iter()
                        .nth(self.selected_idx)
                        .map(|l| format!("{} ⇄ {}", l.system_name_a, l.system_name_b)),
                };
                if let Some(t) = text {
                    if let Some(cb) = &mut self.clipboard {
                        let _ = cb.set_text(&t);
                        self.status_msg = format!("Copied: {t}");
                    }
                }
                return ViewEvent::Consumed;
            }
            _ => {}
        }
        ViewEvent::None
    }

    fn handle_detail_key(&mut self, key: &KeyEvent) -> ViewEvent {
        match key.code {
            KeyCode::Tab => {
                self.focus = FocusArea::Filter;
                ViewEvent::Consumed
            }
            KeyCode::Up | KeyCode::Char('w') => {
                if self.detail_cursor > 0 { self.detail_cursor -= 1; }
                ViewEvent::Consumed
            }
            KeyCode::Down | KeyCode::Char('s') => {
                self.detail_cursor = (self.detail_cursor + 1).min(1);
                ViewEvent::Consumed
            }
            KeyCode::Char('c') => {
                let text = match self.route_type() {
                    RouteType::Routes => self
                        .filtered_routes()
                        .into_iter()
                        .nth(self.selected_idx)
                        .map(|r| if self.detail_cursor == 0 {
                            r.from_system_name.clone()
                        } else {
                            r.to_system_name.clone()
                        }),
                    RouteType::Loops => self
                        .filtered_loops()
                        .into_iter()
                        .nth(self.selected_idx)
                        .map(|l| if self.detail_cursor == 0 {
                            l.system_name_a.clone()
                        } else {
                            l.system_name_b.clone()
                        }),
                };
                if let Some(t) = text {
                    if let Some(cb) = &mut self.clipboard {
                        let _ = cb.set_text(&t);
                        self.status_msg = format!("Copied: {t}");
                    }
                }
                ViewEvent::Consumed
            }
            _ => ViewEvent::None,
        }
    }

    /// One ←/→ step on filter field `idx`. Server-side filters schedule a new search;
    /// Type and Superpower only filter what is loaded.
    fn step_filter(&mut self, idx: usize, forward: bool, api: &ApiClient, journal: &JournalData) {
        let step = |v: i32| if forward { v + 1_000 } else { (v - 1_000).max(0) };
        match idx {
            field::TYPE => self.type_idx = cycle(self.type_idx, TYPE_OPTIONS.len(), forward),
            field::SUPERPOWER => self.superpower_idx = cycle(self.superpower_idx, SUPERPOWER_OPTIONS.len(), forward),
            field::NEAR => return,
            field::RADIUS => {
                let r = if forward { self.radius + RADIUS_STEP } else { self.radius - RADIUS_STEP };
                self.radius = r.clamp(MIN_RADIUS, MAX_RADIUS);
            }
            field::PAD => self.pad_idx = step_clamped(self.pad_idx, PAD_OPTIONS.len(), forward),
            field::MIN_SUPPLY => self.min_supply = step(self.min_supply),
            field::MIN_DEMAND => self.min_demand = step(self.min_demand),
            field::MAX_AGE => self.age_idx = step_clamped(self.age_idx, AGE_OPTIONS.len(), forward),
            field::ARRIVAL => self.arrival_idx = step_clamped(self.arrival_idx, ARRIVAL_OPTIONS.len(), forward),
            field::PLANETARY => self.include_planetary = forward,
            field::SETTLEMENTS => self.include_settlements = forward,
            field::CARRIERS => {
                // The bubble-wide lists come in a with- and a without-carriers variant.
                self.include_carriers = forward;
                if self.near.trim().is_empty() {
                    self.reset_list();
                    self.fetch_all(api, journal);
                    return;
                }
            }
            _ => return,
        }
        if idx == field::TYPE || idx == field::SUPERPOWER {
            self.reset_list();
        } else {
            self.search_filter_changed();
        }
    }

    /// After a server-side filter changed: back to the top, and search again soon.
    fn search_filter_changed(&mut self) {
        self.reset_list();
        if !self.near.trim().is_empty() {
            self.schedule_search();
        }
    }

    /// Sets a search-side toggle from the mouse (Exclude = 0 / Include = 1).
    fn set_toggle(&mut self, idx: usize, include: bool, api: &ApiClient, journal: &JournalData) {
        let current = match idx {
            field::PLANETARY => self.include_planetary,
            field::SETTLEMENTS => self.include_settlements,
            field::CARRIERS => self.include_carriers,
            _ => return,
        };
        if current != include {
            self.step_filter(idx, include, api, journal);
        }
    }

    /// Back to the top of the list, after a filter changed what it holds.
    fn reset_list(&mut self) {
        self.selected_idx = 0;
    }

    fn current_count(&self) -> usize {
        match self.route_type() {
            RouteType::Routes => self.filtered_routes().len(),
            RouteType::Loops => self.filtered_loops().len(),
        }
    }
}

fn cycle(idx: usize, len: usize, forward: bool) -> usize {
    if forward { (idx + 1) % len } else { (idx + len - 1) % len }
}

/// One step along an ordered option list, stopping at both ends.
fn step_clamped(idx: usize, len: usize, forward: bool) -> usize {
    if forward { (idx + 1).min(len - 1) } else { idx.saturating_sub(1) }
}

fn format_num(n: i32) -> String {
    edcas_common::format::group_thousands(n as i64)
}

fn pad_str(pad: &Option<String>) -> &str {
    match pad.as_deref() {
        Some("L") => "Large pad",
        Some("M") => "Medium pad",
        Some("S") => "Small pad",
        _ => "Unknown pad",
    }
}
