mod gui;

use crate::api_client::ApiClient;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::JournalData;
use crate::views::ViewEvent;
use edcas_common::api::{TradeLoopResponse, TradeRouteResponse};

const TYPE_OPTIONS: &[&str] = &["Routes", "Loops"];
const SUPERPOWER_OPTIONS: &[&str] = &["All", "Federation", "Empire", "Alliance", "Independent"];

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

    pub fn on_enter(&mut self, api: &ApiClient, _journal: &JournalData) {
        if self.routes.is_empty() && self.loops.is_empty() {
            self.fetch_all(api);
        }
    }

    fn fetch_all(&mut self, api: &ApiClient) {
        self.fetch_routes(api);
        self.fetch_loops(api);
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
        api.spawn(async move {
            let _ = tx.send(api_owned.fetch_trade_routes().await.map_err(|e| e.to_string()));
        });
    }

    fn fetch_loops(&mut self, api: &ApiClient) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.loop_rx = Some(rx);
        let api_owned = api.clone();
        api.spawn(async move {
            let _ = tx.send(api_owned.fetch_trade_loops().await.map_err(|e| e.to_string()));
        });
        self.status_msg = "Loading…".into();
    }

    pub fn poll_results(&mut self) {
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
            self.status_msg = format!(
                "{r} routes  {l} loops  |  w/s: navigate  |  Tab: filters  |  c: copy systems  |  r: refresh"
            );
            self.focus = FocusArea::List;
        } else {
            self.status_msg = "No data yet — cache may still be computing".into();
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent, api: &ApiClient, journal: &JournalData) -> ViewEvent {
        match self.focus {
            FocusArea::Filter => self.handle_filter_key(key, api, journal),
            FocusArea::List   => self.handle_list_key(key, api, journal),
            FocusArea::Detail => self.handle_detail_key(key),
        }
    }

    fn handle_filter_key(&mut self, key: &KeyEvent, _api: &ApiClient, _journal: &JournalData) -> ViewEvent {
        match key.code {
            KeyCode::Tab => {
                self.focus = FocusArea::List;
                return ViewEvent::Consumed;
            }
            KeyCode::Char('s') | KeyCode::Down => {
                if self.filter_field < 3 {
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
                match self.filter_field {
                    0 => {
                        self.type_idx = cycle(self.type_idx, TYPE_OPTIONS.len(), forward);
                        self.reset_list();
                    }
                    1 => {
                        self.superpower_idx = cycle(self.superpower_idx, SUPERPOWER_OPTIONS.len(), forward);
                        self.reset_list();
                    }
                    2 => {
                        self.min_supply = if forward {
                            self.min_supply + 1_000
                        } else {
                            (self.min_supply - 1_000).max(0)
                        };
                        self.reset_list();
                    }
                    3 => {
                        self.min_demand = if forward {
                            self.min_demand + 1_000
                        } else {
                            (self.min_demand - 1_000).max(0)
                        };
                        self.reset_list();
                    }
                    _ => {}
                }
                return ViewEvent::Consumed;
            }
            _ => {}
        }
        ViewEvent::None
    }

    fn handle_list_key(&mut self, key: &KeyEvent, api: &ApiClient, _journal: &JournalData) -> ViewEvent {
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
                self.fetch_all(api);
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
