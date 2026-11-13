mod gui;

use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::{JournalData, StationData};
use edcas_common::api::{
    CommodityPricePoint, ConstructionDepotResponse, ConstructionQuery,
    LandingPadsResponse, StationEconomyResponse, StationQuery, StationResponse,
};
use std::collections::{HashMap, HashSet};

use crate::api_client::ApiClient;
use crate::todo::{ConstructionTodoItem, ConstructionTodoResource, TodoList};
use crate::views::util::{
    compute_todo_needed, effective_todo_for_market, sorted_commodities,
    FocusArea, MarketSortCol, SearchState, StationDetailTab as DetailTab,
};
use crate::views::ViewEvent;

use std::sync::{Arc, Mutex};

enum ListItem<'a> {
    Api(&'a StationResponse),
    Journal(&'a StationData),
}

/// The list section a station is shown under, in display order.
#[derive(Clone, Copy, PartialEq)]
enum ListSection {
    /// Pinned stations and tracked construction sites.
    Pinned,
    Search,
    CurrentSystem,
    History,
}

pub struct StationsView {
    search_query: String,
    search_state: SearchState,
    results: Vec<StationResponse>,
    pinned_ids: HashSet<i64>,
    pinned_results: Vec<StationResponse>,
    /// Stations fetched automatically on dock; used instead of the journal snapshot.
    auto_fetched: HashMap<i64, StationResponse>,
    /// Market IDs for which an auto-fetch was attempted (regardless of outcome),
    /// so we don't retry on every journal update.
    auto_fetch_attempted: HashSet<i64>,
    /// Tracked construction depot IDs — source of truth, persisted to pins.json.
    construction_tracked: HashSet<i64>,
    /// Full API data for tracked depots (cross-session fallback when not docked).
    depots: Vec<ConstructionDepotResponse>,
    /// Index of the currently selected resource row in the construction overview.
    construction_resource_idx: usize,
    loading_depots: bool,
    pending_depots: Arc<Mutex<Option<Result<Vec<ConstructionDepotResponse>, String>>>>,
    selected_idx: usize,
    /// Market ID of the currently selected station; used to re-resolve selected_idx
    /// when visited_stations changes order (e.g., a new dock inserts at position 0).
    selected_market_id: Option<i64>,
    /// Shown inside the Search tab, whose field does the searching: no field of its own.
    pub embedded: bool,
    /// A station asked for by [`Self::open_station`], selected once it is listed.
    want_market: Option<i64>,
    detail_scroll: usize,
    status_msg: String,
    focus: FocusArea,
    detail_tab: DetailTab,
    market_sort_col: MarketSortCol,
    market_sort_asc: bool,
    clipboard: Option<arboard::Clipboard>,
    loading_pins: bool,
    pending_pins: Arc<Mutex<Option<Result<Vec<StationResponse>, String>>>>,
    pending_search: Arc<Mutex<Option<Result<Vec<StationResponse>, String>>>>,
    pending_auto_fetch: Arc<Mutex<Option<Result<Vec<StationResponse>, String>>>>,
    auto_fetch_in_flight: Option<i64>,

    // Market commodity selection (for price history)
    market_selected_row: usize,
    /// Commodity name currently shown in the PriceHistory tab
    commodity_history_name: String,
    commodity_history_market_id: Option<i64>,
    history_buy_data: Vec<(f64, f64)>,
    history_sell_data: Vec<(f64, f64)>,
    history_time_bounds: [f64; 2],
    history_y_bounds: [f64; 2],
    history_x_labels: Vec<String>,
    loading_history: bool,
    pending_history: Arc<Mutex<Option<Result<Vec<CommodityPricePoint>, String>>>>,
}

impl StationsView {
    pub fn new() -> Self {
        let pins = crate::pins::Pins::load();
        Self {
            search_query: String::new(),
            search_state: SearchState::Idle,
            results: Vec::new(),
            pinned_ids: pins.stations,
            pinned_results: Vec::new(),
            auto_fetched: HashMap::new(),
            auto_fetch_attempted: HashSet::new(),
            construction_tracked: pins.constructions,
            depots: Vec::new(),
            construction_resource_idx: 0,
            loading_depots: false,
            pending_depots: Arc::new(Mutex::new(None)),
            selected_idx: 0,
            selected_market_id: None,
            embedded: false,
            want_market: None,
            detail_scroll: 0,
            status_msg: "Press Enter to search  |  p: pin  |  m: mark carrier mine  |  c: copy system".into(),
            focus: FocusArea::List,
            detail_tab: DetailTab::Overview,
            market_sort_col: MarketSortCol::default(),
            market_sort_asc: true,
            clipboard: arboard::Clipboard::new().ok(),
            loading_pins: false,
            pending_pins: Arc::new(Mutex::new(None)),
            pending_search: Arc::new(Mutex::new(None)),
            pending_auto_fetch: Arc::new(Mutex::new(None)),
            auto_fetch_in_flight: None,
            market_selected_row: 0,
            commodity_history_name: String::new(),
            commodity_history_market_id: None,
            history_buy_data: Vec::new(),
            history_sell_data: Vec::new(),
            history_time_bounds: [0.0, 1.0],
            history_y_bounds: [0.0, 1.0],
            history_x_labels: Vec::new(),
            loading_history: false,
            pending_history: Arc::new(Mutex::new(None)),
        }
    }

    pub fn poll_search(&mut self) {
        if let Some(result) = self.pending_pins.lock().unwrap().take() {
            self.loading_pins = false;
            match result {
                Ok(api_results) => {
                    // Merge: replace/enrich stubs with API data; keep stubs for stations the API didn't return.
                    // Skip stations that were unpinned while the request was in flight.
                    for api_station in api_results {
                        if self.pinned_ids.contains(&api_station.market_id) {
                            merge_into_pinned(&mut self.pinned_results, api_station);
                        }
                    }
                    self.pinned_results.sort_by(|a, b| a.name.cmp(&b.name));
                    if self.status_msg.starts_with("Loading") {
                        self.status_msg = "Press Enter to search  |  p: pin  |  m: mark carrier mine  |  c: copy system".into();
                    }
                }
                Err(e) => { self.status_msg = format!("API unavailable: {e}"); }
            }
        }
        if let Some(result) = self.pending_search.lock().unwrap().take() {
            match result {
                Ok(results) => {
                    let count = results.len();
                    self.results = results;
                    self.selected_idx = 0;
                    self.selected_market_id = None;
                    self.detail_scroll = 0;
                    self.focus = FocusArea::List;
                    self.detail_tab = DetailTab::Overview;
                    self.status_msg = if count == 0 {
                        format!("No stations found for '{}'", self.search_query)
                    } else {
                        format!("{count} station(s) found")
                    };
                }
                Err(e) => { self.status_msg = format!("API error: {e}"); }
            }
        }

        let history_result = self.pending_history.lock().unwrap().take();
        if let Some(result) = history_result {
            self.loading_history = false;
            match result {
                Ok(points) => self.set_commodity_history_data(points),
                Err(e) => {
                    self.history_buy_data.clear();
                    self.history_sell_data.clear();
                    self.history_x_labels = vec![format!("Error: {e}")];
                }
            }
        }
    }

    pub fn on_enter(&mut self, api: &ApiClient, journal: &JournalData) {
        let history = &journal.visited_stations;
        for &mid in &self.pinned_ids {
            if !self.pinned_results.iter().any(|r| r.market_id == mid) {
                if let Some(h) = history.iter().find(|s| s.market_id == mid) {
                    self.pinned_results.push(journal_station_to_response(h));
                }
            }
        }
        self.pinned_results.sort_by(|a, b| a.name.cmp(&b.name));
        if !self.pinned_ids.is_empty() && !self.loading_pins {
            self.refresh_pins(api);
        }
        {
            let has_unloaded = self.construction_tracked.iter()
                .any(|id| !self.depots.iter().any(|d| d.market_id == *id));
            if has_unloaded && !self.loading_depots {
                self.refresh_tracked(api);
            }
        }
    }

    fn refresh_pins(&mut self, api: &ApiClient) {
        self.loading_pins = true;
        // Do NOT clear pinned_results here — stubs remain visible while API is in flight.
        self.status_msg = "Loading pinned stations…".into();
        let pending = Arc::clone(&self.pending_pins);
        let api_owned = api.clone();
        let ids: Vec<i64> = self.pinned_ids.iter().copied().collect();
        api.spawn(async move {
            let mut results = Vec::new();
            for mid in ids {
                let query = StationQuery { market_id: Some(mid), limit: Some(1), name: None, system_name: None };
                if let Ok(mut r) = api_owned.search_stations(&query).await {
                    if let Some(s) = r.pop() {
                        results.push(s);
                    }
                }
            }
            results.sort_by(|a, b| a.name.cmp(&b.name));
            *pending.lock().unwrap() = Some(Ok(results));
        });
    }

    fn save_pins(&self) {
        let mut pins = crate::pins::Pins::load();
        pins.stations = self.pinned_ids.clone();
        pins.save();
    }

    fn save_tracked(&self) {
        let mut pins = crate::pins::Pins::load();
        pins.constructions = self.construction_tracked.clone();
        pins.save();
    }

    fn refresh_tracked(&mut self, api: &ApiClient) {
        self.loading_depots = true;
        let pending = Arc::clone(&self.pending_depots);
        let api_owned = api.clone();
        let ids: Vec<i64> = self.construction_tracked.iter()
            .filter(|id| !self.depots.iter().any(|d| &&d.market_id == id))
            .copied()
            .collect();
        api.spawn(async move {
            let mut results = Vec::new();
            for mid in ids {
                let query = ConstructionQuery {
                    market_id: Some(mid),
                    limit: Some(1),
                    name: None,
                    system_name: None,
                };
                if let Ok(mut r) = api_owned.search_construction_depots(&query).await {
                    if let Some(d) = r.pop() {
                        results.push(d);
                    }
                }
            }
            *pending.lock().unwrap() = Some(Ok(results));
        });
    }

    pub fn poll_construction(&mut self) {
        if let Some(result) = self.pending_depots.lock().unwrap().take() {
            self.loading_depots = false;
            if let Ok(new_depots) = result {
                for depot in new_depots {
                    if !self.depots.iter().any(|d| d.market_id == depot.market_id) {
                        self.depots.push(depot);
                    }
                }
                self.depots.sort_by(|a, b| {
                    a.system_name.cmp(&b.system_name).then(a.station_name.cmp(&b.station_name))
                });
            }
        }
    }

    pub fn update_construction_from_journal(
        &mut self,
        api: &ApiClient,
        journal: &JournalData,
        todo: &mut TodoList,
    ) {
        let mut any_new = false;

        for depot in journal.construction_depots.values() {
            let mid = depot.submission.market_id;
            let existing = self.depots.iter_mut().find(|d| d.market_id == mid);

            if let Some(entry) = existing {
                entry.progress = depot.submission.progress;
                entry.construction_complete = depot.submission.construction_complete;
                entry.construction_failed = depot.submission.construction_failed;
                entry.resources = depot.submission.resources.iter().map(|r| {
                    edcas_common::api::ConstructionResourceResponse {
                        name: r.name.clone(),
                        display_name: r.display_name.clone(),
                        required_amount: r.required_amount,
                        provided_amount: r.provided_amount,
                        payment: r.payment,
                    }
                }).collect();
                todo.update_construction_item(ConstructionTodoItem {
                    market_id: mid,
                    station_name: depot.submission.station_name.clone(),
                    system_name: depot.system_name.clone(),
                    resources: depot.submission.resources.iter()
                        .filter(|r| r.provided_amount < r.required_amount)
                        .map(|r| ConstructionTodoResource {
                            commodity_name: r.name.clone(),
                            display_name: r.display_name.clone(),
                            required_amount: r.required_amount,
                            provided_amount: r.provided_amount,
                            payment: r.payment,
                        })
                        .collect(),
                });
            } else {
                self.construction_tracked.insert(mid);
                any_new = true;

                {
                    let api_owned = api.clone();
                    let submission = depot.submission.clone();
                    api.spawn(async move {
                        let _ = api_owned.submit_construction_depot(&submission).await;
                    });
                }

                self.depots.push(edcas_common::api::ConstructionDepotResponse {
                    market_id: mid,
                    system_address: depot.submission.system_address,
                    station_name: depot.submission.station_name.clone(),
                    system_name: depot.system_name.clone(),
                    progress: depot.submission.progress,
                    construction_complete: depot.submission.construction_complete,
                    construction_failed: depot.submission.construction_failed,
                    last_updated: None,
                    resources: depot.submission.resources.iter().map(|r| {
                        edcas_common::api::ConstructionResourceResponse {
                            name: r.name.clone(),
                            display_name: r.display_name.clone(),
                            required_amount: r.required_amount,
                            provided_amount: r.provided_amount,
                            payment: r.payment,
                        }
                    }).collect(),
                });
            }
        }

        self.depots.sort_by(|a, b| a.system_name.cmp(&b.system_name).then(a.station_name.cmp(&b.station_name)));

        if any_new {
            self.save_tracked();
        }
    }

    /// Tracks a new construction depot and triggers an API fetch for its data.
    pub fn track_construction(&mut self, market_id: i64, api: &ApiClient) {
        if self.construction_tracked.contains(&market_id) {
            return;
        }
        self.construction_tracked.insert(market_id);
        self.save_tracked();
        if !self.depots.iter().any(|d| d.market_id == market_id) && !self.loading_depots {
            self.refresh_tracked(api);
        }
    }

    fn toggle_construction_todo(&mut self, market_id: i64, journal: &JournalData, todo: &mut TodoList) {
        if todo.construction_items.iter().any(|i| i.market_id == market_id) {
            todo.remove_construction_item(market_id);
        } else {
            let item = if let Some(local) = journal.construction_depots.get(&market_id) {
                super::construction::construction_todo_item_from_depot(local)
            } else if let Some(depot) = self.depots.iter().find(|d| d.market_id == market_id).cloned() {
                super::construction::construction_todo_item_from_response(&depot)
            } else {
                return;
            };
            todo.add_construction_item(item);
            if self.construction_tracked.insert(market_id) {
                self.save_tracked();
            }
        }
        todo.save();
    }

    /// Fetches full station data (market, outfitting, shipyard) from the API and caches it
    /// so the journal snapshot is replaced with live data without requiring the user to pin.
    /// Triggered on dock and lazily for whichever snapshot station is currently selected.
    /// The per-station attempt guard means each station is requested at most once per session.
    pub fn request_station_data(&mut self, market_id: i64, api: &ApiClient) {
        if self.pinned_ids.contains(&market_id)
            || self.auto_fetched.contains_key(&market_id)
            || self.auto_fetch_attempted.contains(&market_id)
            || self.auto_fetch_in_flight.is_some()
        {
            return;
        }
        self.auto_fetch_in_flight = Some(market_id);
        let pending = Arc::clone(&self.pending_auto_fetch);
        let api_owned = api.clone();
        api.spawn(async move {
            let query = StationQuery {
                market_id: Some(market_id),
                limit: Some(1),
                name: None,
                system_name: None,
            };
            let result = api_owned.search_stations(&query).await.map_err(|e| e.to_string());
            *pending.lock().unwrap() = Some(result);
        });
    }

    /// Called after each journal update. Stores the latest external ID sets and
    /// re-resolves selected_idx so the selection follows the same station even
    /// when visited_stations changes order.
    pub fn on_journal_update(&mut self, journal: &JournalData) {
        // Auto-unpin construction sites that are now complete.
        let completed: Vec<i64> = self.construction_tracked.iter().copied()
            .filter(|&mid| {
                journal.construction_depots.get(&mid)
                    .map(|d| d.submission.construction_complete)
                    .unwrap_or(false)
                    || self.depots.iter().any(|d| d.market_id == mid && d.construction_complete)
            })
            .collect();
        if !completed.is_empty() {
            for mid in &completed {
                self.construction_tracked.remove(mid);
                self.pinned_ids.remove(mid);
                self.pinned_results.retain(|r| r.market_id != *mid);
            }
            self.save_pins();
            self.save_tracked();
        }

        let Some(mid) = self.selected_market_id else { return; };
        let mids: Vec<i64> = self.build_display_list(journal).iter().map(|item| match item {
            ListItem::Api(s) => s.market_id,
            ListItem::Journal(s) => s.market_id,
        }).collect();
        if let Some(pos) = mids.iter().position(|&m| m == mid) {
            self.selected_idx = pos;
        } else {
            self.selected_market_id = None;
            if !mids.is_empty() {
                self.selected_idx = self.selected_idx.min(mids.len() - 1);
            }
        }
    }

    fn mid_at_idx(&self, journal: &JournalData) -> Option<i64> {
        self.selected_item(journal).map(|item| match item {
            ListItem::Api(s) => s.market_id,
            ListItem::Journal(s) => s.market_id,
        })
    }

    pub fn poll_auto_fetch(&mut self) {
        if let Some(result) = self.pending_auto_fetch.lock().unwrap().take() {
            if let Some(id) = self.auto_fetch_in_flight.take() {
                self.auto_fetch_attempted.insert(id);
            }
            if let Ok(mut results) = result {
                if let Some(station) = results.pop() {
                    let mid = station.market_id;
                    if self.pinned_ids.contains(&mid) {
                        // Already pinned — merge directly into pinned_results.
                        merge_into_pinned(&mut self.pinned_results, station);
                        self.pinned_results.sort_by(|a, b| a.name.cmp(&b.name));
                    } else {
                        self.auto_fetched.insert(mid, station);
                    }
                }
            }
        }
    }

    /// Lazily requests live data for whichever station is currently selected, when it's
    /// only a journal snapshot with no data from the current session. Called every frame
    /// while the Stations view is open; the attempt guard and single in-flight slot keep
    /// this to one request per station and one in flight at a time.
    ///
    /// The server is queried only as a last resort — when we have nothing local — because
    /// its market data may be from a previous session and stale. Current-session sources
    /// (a parsed Market.json, or the station we're docked at) always win, and carriers are
    /// skipped entirely (no public market; their stored inventory is shown instead).
    pub fn poll_lazy_fetch(&mut self, api: &ApiClient, journal: &JournalData) {
        if self.auto_fetch_in_flight.is_some() {
            return;
        }
        let mid = match self.selected_item(journal) {
            Some(ListItem::Journal(s)) => {
                // This session already captured the market here — keep it, don't fetch.
                if !s.commodities.is_empty() {
                    return;
                }
                s.market_id
            }
            _ => return,
        };
        let is_carrier = journal.visited_carriers.iter().any(|c| c.market_id == mid);
        let docked_here = journal.last_docked.as_ref().map(|(m, _, _, _)| *m == mid).unwrap_or(false);
        let local_pending = journal.local_market.as_ref().map(|(m, _)| *m == mid).unwrap_or(false);
        if is_carrier || docked_here || local_pending {
            return;
        }
        self.request_station_data(mid, api);
    }

    fn build_display_list<'a>(&'a self, journal: &'a JournalData) -> Vec<ListItem<'a>> {
        self.build_sectioned_list(journal).into_iter().map(|(_, item)| item).collect()
    }

    /// The station list in display order, each entry tagged with the list section it sits in.
    fn build_sectioned_list<'a>(&'a self, journal: &'a JournalData) -> Vec<(ListSection, ListItem<'a>)> {
        let stations    = &journal.visited_stations;
        let carriers    = &journal.visited_carriers;
        let current_sys = journal.current_system.as_ref().map(|s| s.name.as_str()).unwrap_or_default();
        let mut shown: HashSet<i64> = HashSet::new();
        let mut list: Vec<(ListSection, ListItem<'a>)> = Vec::new();
        let live_or_snapshot = |s: &'a StationData| match self.auto_fetched.get(&s.market_id) {
            Some(f) => ListItem::Api(f),
            None => ListItem::Journal(s),
        };

        // 1. Pinned stations
        for s in &self.pinned_results {
            shown.insert(s.market_id);
            list.push((ListSection::Pinned, ListItem::Api(s)));
        }
        // 1b. Tracked construction sites not already pinned
        let tracked: Vec<&StationData> = stations.iter().filter(|s| {
            self.construction_tracked.contains(&s.market_id) && !shown.contains(&s.market_id)
        }).collect();
        for s in tracked {
            shown.insert(s.market_id);
            list.push((ListSection::Pinned, live_or_snapshot(s)));
        }

        // 2. Search results
        let search: Vec<&StationResponse> = self.results.iter().filter(|s| !shown.contains(&s.market_id)).collect();
        for s in search { shown.insert(s.market_id); list.push((ListSection::Search, ListItem::Api(s))); }

        // 3. Current system
        if !current_sys.is_empty() {
            let cur_s: Vec<&StationData> = stations.iter().filter(|s| s.system_name == current_sys && !shown.contains(&s.market_id)).collect();
            for s in cur_s {
                shown.insert(s.market_id);
                list.push((ListSection::CurrentSystem, live_or_snapshot(s)));
            }
            let cur_c: Vec<&StationData> = carriers.iter().filter(|c| c.system_name == current_sys && !shown.contains(&c.market_id)).collect();
            for c in cur_c { shown.insert(c.market_id); list.push((ListSection::CurrentSystem, ListItem::Journal(c))); }
        }

        // 5. History
        let hist_s: Vec<&StationData> = stations.iter().filter(|s| s.system_name != current_sys && !shown.contains(&s.market_id)).collect();
        for s in hist_s {
            shown.insert(s.market_id);
            list.push((ListSection::History, live_or_snapshot(s)));
        }
        let hist_c: Vec<&StationData> = carriers.iter().filter(|c| c.system_name != current_sys && !shown.contains(&c.market_id)).collect();
        for c in hist_c { shown.insert(c.market_id); list.push((ListSection::History, ListItem::Journal(c))); }

        list
    }

    fn display_count(&self, journal: &JournalData) -> usize {
        self.build_display_list(journal).len()
    }

    fn selected_item<'a>(&'a self, journal: &'a JournalData) -> Option<ListItem<'a>> {
        self.build_display_list(journal).into_iter().nth(self.selected_idx)
    }

    /// Returns `Some(market_id)` when a station is being **pinned** (not unpinned),
    /// so the caller can emit `TrackConstruction` for construction sites.
    fn toggle_pin(&mut self, api: &ApiClient, journal: &JournalData) -> Option<i64> {
        let n = self.pinned_results.len();
        if self.selected_idx < n {
            // Unpin
            let mid = self.pinned_results[self.selected_idx].market_id;
            self.pinned_ids.remove(&mid);
            self.pinned_results.remove(self.selected_idx);
            let total = self.display_count(journal);
            if total > 0 {
                self.selected_idx = self.selected_idx.min(total - 1);
            } else {
                self.selected_idx = 0;
            }
            self.save_pins();
            return None;
        }
        // Clone the data from the selected item before mutating self.
        let item_data = self.build_display_list(journal).into_iter()
            .nth(self.selected_idx)
            .map(|item| match item {
                ListItem::Api(s)      => (s.market_id, Some(s.clone()), None),
                ListItem::Journal(s)  => (s.market_id, None, Some(s.clone())),
            });
        let Some((mid, api_data, journal_data)) = item_data else {
            return None;
        };
        if self.pinned_ids.contains(&mid) {
            return None;
        }
        self.pinned_ids.insert(mid);
        let stub = if let Some(s) = api_data {
            s
        } else if let Some(fetched) = self.auto_fetched.remove(&mid) {
            fetched
        } else if let Some(ref s) = journal_data {
            journal_station_to_response(s)
        } else {
            return None;
        };
        merge_into_pinned(&mut self.pinned_results, stub);
        self.pinned_results.sort_by(|a, b| a.name.cmp(&b.name));
        if let Some(pos) = self.pinned_results.iter().position(|r| r.market_id == mid) {
            self.selected_idx = pos;
        }
        self.refresh_pins(api);
        self.save_pins();
        Some(mid)
    }

    /// Searches for `q`, from the Search tab's field.
    pub fn search_for(&mut self, q: &str, api: &ApiClient) {
        self.search_query = q.to_string();
        self.search_state = SearchState::Idle;
        self.do_search(api);
    }

    /// Lists and selects one station, looked up by its market id.
    pub fn open_station(&mut self, market_id: i64, name: &str, api: &ApiClient) {
        self.search_query = name.to_string();
        self.search_state = SearchState::Idle;
        self.want_market = Some(market_id);
        self.status_msg = format!("Looking up {name}…");
        let pending = Arc::clone(&self.pending_search);
        let api_owned = api.clone();
        let query = StationQuery { name: None, system_name: None, market_id: Some(market_id), limit: Some(1) };
        api.spawn(async move {
            let result = api_owned.search_stations(&query).await.map_err(|e| e.to_string());
            *pending.lock().unwrap() = Some(result);
        });
    }

    /// Selects the station [`Self::open_station`] asked for, once it is in the list.
    pub(crate) fn select_wanted(&mut self, journal: &JournalData) {
        let Some(mid) = self.want_market else { return };
        let at = self.build_display_list(journal).iter().position(|item| match item {
            ListItem::Api(s) => s.market_id == mid,
            ListItem::Journal(s) => s.market_id == mid,
        });
        if let Some(i) = at {
            self.selected_idx = i;
            self.selected_market_id = Some(mid);
            self.focus = FocusArea::Detail;
            self.want_market = None;
        }
    }

    fn do_search(&mut self, api: &ApiClient) {
        if self.search_query.is_empty() {
            self.status_msg = "Enter a search term first".into();
            return;
        }
        self.status_msg = format!("Searching for '{}'…", self.search_query);
        let pending = Arc::clone(&self.pending_search);
        let api_owned = api.clone();
        let query = StationQuery {
            name: Some(self.search_query.clone()),
            system_name: None,
            market_id: None,
            limit: Some(50),
        };
        api.spawn(async move {
            let result = api_owned.search_stations(&query).await.map_err(|e| e.to_string());
            *pending.lock().unwrap() = Some(result);
        });
    }

    fn fetch_commodity_history(&mut self, api: &ApiClient) {
        let Some(market_id) = self.commodity_history_market_id else { return };
        let commodity = self.commodity_history_name.clone();
        if commodity.is_empty() { return; }
        self.loading_history = true;
        self.history_buy_data.clear();
        self.history_sell_data.clear();
        self.history_x_labels.clear();
        let pending = Arc::clone(&self.pending_history);
        let api_clone = api.clone();
        api.spawn(async move {
            let result = api_clone
                .fetch_commodity_price_history(market_id, &commodity, 30)
                .await
                .map_err(|e| e.to_string());
            *pending.lock().unwrap() = Some(result);
        });
    }

    fn set_commodity_history_data(&mut self, mut points: Vec<CommodityPricePoint>) {
        if points.is_empty() {
            self.history_buy_data.clear();
            self.history_sell_data.clear();
            self.history_x_labels = vec!["No data yet".to_string()];
            return;
        }

        points.dedup_by(|a, b| {
            let ta = a.timestamp.timestamp() / 3600;
            let tb = b.timestamp.timestamp() / 3600;
            ta == tb
        });

        let min_ts = points[0].timestamp.timestamp() as f64;
        let max_ts = points.last().unwrap().timestamp.timestamp() as f64;
        let total_days = ((max_ts - min_ts) / 86400.0).max(0.5);

        self.history_buy_data = points
            .iter()
            .map(|p| {
                let x = (p.timestamp.timestamp() as f64 - min_ts) / 86400.0;
                (x, p.buy_price as f64)
            })
            .collect();
        self.history_sell_data = points
            .iter()
            .map(|p| {
                let x = (p.timestamp.timestamp() as f64 - min_ts) / 86400.0;
                (x, p.sell_price as f64)
            })
            .collect();

        self.history_time_bounds = [0.0, total_days];

        let all_prices: Vec<f64> = points
            .iter()
            .flat_map(|p| [p.buy_price as f64, p.sell_price as f64])
            .filter(|&v| v > 0.0)
            .collect();
        let min_p = all_prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_p = all_prices.iter().cloned().fold(0.0_f64, f64::max);
        let padding = ((max_p - min_p) * 0.1).max(10.0);
        self.history_y_bounds = [(min_p - padding).max(0.0), max_p + padding];

        let n_x = 5usize;
        let start_ts = chrono::DateTime::from_timestamp(points[0].timestamp.timestamp(), 0)
            .unwrap_or(points[0].timestamp);
        self.history_x_labels = (0..n_x)
            .map(|i| {
                let secs = (total_days * i as f64 / (n_x - 1) as f64 * 86400.0) as i64;
                let dt = start_ts + chrono::Duration::seconds(secs);
                dt.format("%m/%d").to_string()
            })
            .collect();
    }

    /// Returns the market ID of the selected station if it has construction data.
    fn selected_construction_market_id(&self, journal: &JournalData) -> Option<i64> {
        let mid = self.mid_at_idx(journal)?;
        if journal.construction_depots.contains_key(&mid) || self.depots.iter().any(|d| d.market_id == mid) {
            Some(mid)
        } else {
            None
        }
    }

    /// True only while a live fetch for this station is actually in flight. Used to show
    /// an accurate "Loading…" message; stations we deliberately don't fetch (session data
    /// present, docked here, carriers) never report pending.
    fn live_fetch_pending(&self, market_id: i64) -> bool {
        self.auto_fetch_in_flight == Some(market_id)
    }

    /// Status message for a journal-snapshot station with no detailed data loaded yet,
    /// reflecting whether a live fetch is in progress. Carriers are never fetched (their
    /// market isn't public), so they get a distinct message rather than a perpetual "Loading…".
    fn snapshot_data_msg(&self, station: &StationData) -> &'static str {
        if station.station_type == "FleetCarrier" {
            "No public market data \u{2014} carrier markets aren't tracked here."
        } else if self.live_fetch_pending(station.market_id) {
            "Loading live data\u{2026}"
        } else {
            "No live data available for this station."
        }
    }

    /// Returns the market ID of the selected station if it is a fleet carrier.
    fn selected_carrier_market_id(&self, journal: &JournalData) -> Option<i64> {
        let mid = self.mid_at_idx(journal)?;
        journal.visited_carriers.iter().any(|c| c.market_id == mid).then_some(mid)
    }

    fn construction_resource_count(&self, market_id: i64, journal: &crate::journal_reader::JournalData) -> usize {
        if let Some(depot) = journal.construction_depots.get(&market_id) {
            super::construction::depot_resource_count_local(depot)
        } else if let Some(depot) = self.depots.iter().find(|d| d.market_id == market_id) {
            super::construction::depot_resource_count_api(depot)
        } else {
            0
        }
    }

    /// Moves the list selection to `idx`, showing that station's overview from the top.
    fn select_list_idx(&mut self, idx: usize, journal: &JournalData) {
        self.selected_idx = idx;
        self.detail_scroll = 0;
        self.construction_resource_idx = 0;
        self.market_selected_row = 0;
        self.detail_tab = DetailTab::Overview;
        self.selected_market_id = self.mid_at_idx(journal);
    }

    /// Sorts the market by `col`; choosing the current column again flips the direction.
    fn sort_market_by(&mut self, col: MarketSortCol) {
        if col == self.market_sort_col {
            self.market_sort_asc = !self.market_sort_asc;
        } else {
            self.market_sort_col = col;
            self.market_sort_asc = true;
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent, api: &ApiClient, journal: &JournalData, todo: &mut TodoList) -> ViewEvent {
        if matches!(self.search_state, SearchState::Typing) {
            match key.code {
                KeyCode::Esc => { self.search_state = SearchState::Idle; self.status_msg = "Search cancelled".into(); }
                KeyCode::Enter => { self.search_state = SearchState::Idle; self.do_search(api); }
                KeyCode::Backspace => { self.search_query.pop(); }
                KeyCode::Char(c) => { self.search_query.push(c); }
                _ => {}
            }
            return ViewEvent::Consumed;
        }

        match key.code {
            // In detail+overview on a construction site, f opens search nearest.
            KeyCode::Char('f') if self.focus == FocusArea::Detail && self.detail_tab == DetailTab::Overview
                && self.selected_construction_market_id(journal).is_some() =>
            {
                let mid = self.selected_construction_market_id(journal).unwrap();
                let resource = if let Some(local) = journal.construction_depots.get(&mid) {
                    let mut sorted = local.submission.resources.clone();
                    sorted.sort_by(|a, b| {
                        let done_a = a.provided_amount >= a.required_amount;
                        let done_b = b.provided_amount >= b.required_amount;
                        done_a.cmp(&done_b).then(a.display_name.cmp(&b.display_name))
                    });
                    sorted.into_iter().nth(self.construction_resource_idx)
                        .map(|r| (r.display_name.clone(), r.name.clone()))
                } else if let Some(depot) = self.depots.iter().find(|d| d.market_id == mid).cloned() {
                    let mut sorted = depot.resources.clone();
                    sorted.sort_by(|a, b| {
                        let done_a = a.provided_amount >= a.required_amount;
                        let done_b = b.provided_amount >= b.required_amount;
                        done_a.cmp(&done_b).then(a.display_name.cmp(&b.display_name))
                    });
                    sorted.into_iter().nth(self.construction_resource_idx)
                        .map(|r| (r.display_name.clone(), r.name.clone()))
                } else {
                    None
                };
                if let Some((commodity, raw_name)) = resource {
                    let canonical_name = super::search_nearest::resolve_commodity_canonical(&raw_name);
                    let system = journal.current_system.as_ref().map(|s| s.name.clone()).unwrap_or_default();
                    let ship_pad_size = journal.pilot.ship_pad_size;
                    return ViewEvent::OpenSearchNearest { commodity, canonical_name, system, ship_pad_size };
                }
                return ViewEvent::Consumed;
            }
            // 'g' — find the single best station covering the most missing commodities.
            KeyCode::Char('g') if self.focus == FocusArea::Detail && self.detail_tab == DetailTab::Overview
                && self.selected_construction_market_id(journal).is_some() =>
            {
                let mid = self.selected_construction_market_id(journal).unwrap();
                // Display names stay localised for the user; the API needs canonical names.
                let (commodities, canonical_names) = if let Some(local) = journal.construction_depots.get(&mid) {
                    super::search_nearest::multi_search_names(local.submission.resources.iter()
                        .filter(|r| r.provided_amount < r.required_amount)
                        .map(|r| (r.display_name.as_str(), r.name.as_str())))
                } else if let Some(depot) = self.depots.iter().find(|d| d.market_id == mid) {
                    super::search_nearest::multi_search_names(depot.resources.iter()
                        .filter(|r| r.provided_amount < r.required_amount)
                        .map(|r| (r.display_name.as_str(), r.name.as_str())))
                } else {
                    (vec![], vec![])
                };
                if !commodities.is_empty() {
                    let system = journal.current_system.as_ref().map(|s| s.name.clone()).unwrap_or_default();
                    let ship_pad_size = journal.pilot.ship_pad_size;
                    return ViewEvent::OpenMultiSearch { commodities, canonical_names, system, ship_pad_size };
                }
                return ViewEvent::Consumed;
            }
            KeyCode::Char('/') | KeyCode::Char('f') if self.embedded => return ViewEvent::FocusSearch,
            KeyCode::Char('/') | KeyCode::Char('f') => {
                self.search_query.clear();
                self.search_state = SearchState::Typing;
                self.status_msg = "Typing… (Enter to search, Esc to cancel)".into();
            }
            KeyCode::Char('p') => {
                if self.display_count(journal) > 0 {
                    if let Some(pinned_mid) = self.toggle_pin(api, journal) {
                        if journal.construction_depots.contains_key(&pinned_mid) {
                            return ViewEvent::TrackConstruction(pinned_mid);
                        }
                    }
                }
                return ViewEvent::Consumed;
            }
            KeyCode::Char('w') | KeyCode::Up => match self.focus {
                FocusArea::List => {
                    if self.selected_idx > 0 {
                        self.select_list_idx(self.selected_idx - 1, journal);
                    }
                }
                FocusArea::Detail => {
                    if self.detail_tab == DetailTab::Market {
                        self.market_selected_row = self.market_selected_row.saturating_sub(1);
                        let scroll = self.market_selected_row.saturating_sub(3);
                        self.detail_scroll = self.detail_scroll.min(scroll + 1).max(scroll.saturating_sub(1));
                        return ViewEvent::Consumed;
                    } else if self.detail_tab == DetailTab::Overview && self.selected_construction_market_id(journal).is_some() {
                        self.construction_resource_idx = self.construction_resource_idx.saturating_sub(1);
                    } else {
                        self.detail_scroll = self.detail_scroll.saturating_sub(1);
                    }
                }
            },
            KeyCode::Char('s') | KeyCode::Down => match self.focus {
                FocusArea::List => {
                    if self.selected_idx + 1 < self.display_count(journal) {
                        self.select_list_idx(self.selected_idx + 1, journal);
                    }
                }
                FocusArea::Detail => {
                    if self.detail_tab == DetailTab::Market {
                        let max_rows = self.selected_item(journal)
                            .and_then(|item| if let ListItem::Api(s) = item { Some(s.commodities.len().saturating_sub(1)) } else { None })
                            .unwrap_or(0);
                        if self.market_selected_row < max_rows {
                            self.market_selected_row += 1;
                        }
                        return ViewEvent::Consumed;
                    } else if self.detail_tab == DetailTab::Overview {
                        if let Some(mid) = self.selected_construction_market_id(journal) {
                            let max = self.construction_resource_count(mid, journal).saturating_sub(1);
                            if self.construction_resource_idx < max {
                                self.construction_resource_idx += 1;
                            }
                            return ViewEvent::Consumed;
                        }
                    }
                    self.detail_scroll += 1;
                }
            },
            KeyCode::Char('h') if self.focus == FocusArea::Detail && self.detail_tab == DetailTab::Market => {
                let history_target = if let Some(ListItem::Api(station)) = self.selected_item(journal) {
                    let effective_todo = compute_todo_needed(&todo.construction_items, &journal.cargo);
                    let local_effective = effective_todo_for_market(&station.commodities, &effective_todo);
                    let sorted = sorted_commodities(&station.commodities, &local_effective, self.market_sort_col, self.market_sort_asc);
                    sorted.get(self.market_selected_row).map(|c| (c.name.clone(), station.market_id))
                } else {
                    None
                };
                if let Some((name, market_id)) = history_target {
                    self.commodity_history_name = name;
                    self.commodity_history_market_id = Some(market_id);
                    self.detail_tab = DetailTab::PriceHistory;
                    self.detail_scroll = 0;
                    self.fetch_commodity_history(api);
                }
                return ViewEvent::Consumed;
            }
            KeyCode::PageUp => match self.focus {
                FocusArea::List => {
                    self.selected_idx = self.selected_idx.saturating_sub(10);
                    self.selected_market_id = self.mid_at_idx(journal);
                }
                FocusArea::Detail => { self.detail_scroll = self.detail_scroll.saturating_sub(10); }
            },
            KeyCode::PageDown => match self.focus {
                FocusArea::List => {
                    let max = self.display_count(journal).saturating_sub(1);
                    self.selected_idx = (self.selected_idx + 10).min(max);
                    self.selected_market_id = self.mid_at_idx(journal);
                }
                FocusArea::Detail => { self.detail_scroll += 10; }
            },
            KeyCode::Tab => {
                match self.focus {
                    FocusArea::List => {
                        if self.display_count(journal) > 0 {
                            self.focus = FocusArea::Detail;
                            self.detail_scroll = 0;
                        }
                    }
                    FocusArea::Detail => { self.focus = FocusArea::List; }
                }
            }
            KeyCode::Char('d') | KeyCode::Right => {
                if self.focus == FocusArea::Detail {
                    if let Some(next) = self.detail_tab.next() {
                        self.detail_tab = next;
                        self.detail_scroll = 0;
                    }
                }
            }
            KeyCode::Char('a') | KeyCode::Left => {
                if self.focus == FocusArea::Detail {
                    if let Some(prev) = self.detail_tab.prev() {
                        self.detail_tab = prev;
                        self.detail_scroll = 0;
                    }
                }
            }
            KeyCode::Char(c @ '1'..='8') => {
                if self.focus == FocusArea::Detail && self.detail_tab == DetailTab::Market {
                    if let Some(new_col) = MarketSortCol::from_digit(c) {
                        self.sort_market_by(new_col);
                        return ViewEvent::Consumed;
                    }
                }
            }
            KeyCode::Char('t') => {
                let ok_tab = self.focus == FocusArea::List
                    || (self.focus == FocusArea::Detail && self.detail_tab == DetailTab::Overview);
                if ok_tab {
                    let mid = self.mid_at_idx(journal);
                    let is_construction = mid.map(|m| {
                        self.construction_tracked.contains(&m)
                            || journal.construction_depots.contains_key(&m)
                            || self.depots.iter().any(|d| d.market_id == m)
                    }).unwrap_or(false);
                    if is_construction {
                        if let Some(mid) = self.selected_construction_market_id(journal) {
                            self.toggle_construction_todo(mid, journal, todo);
                        } else {
                            self.status_msg = "No resource data yet — dock at the construction depot first".into();
                        }
                        return ViewEvent::Consumed;
                    }
                }
            }
            KeyCode::Char('m') => {
                let ok_tab = self.focus == FocusArea::List
                    || (self.focus == FocusArea::Detail && self.detail_tab == DetailTab::Overview);
                if ok_tab {
                    if let Some(mid) = self.selected_carrier_market_id(journal) {
                        return ViewEvent::ToggleMyCarrier(mid);
                    }
                }
            }
            KeyCode::Char('c') => {
                if let Some(item) = self.selected_item(journal) {
                    let name = match item {
                        ListItem::Api(s) => s.system_name.trim().to_string(),
                        ListItem::Journal(s) => s.system_name.trim().to_string(),
                    };
                    if let Some(cb) = self.clipboard.as_mut() {
                        let _ = cb.set_text(&name);
                        self.status_msg = format!("Copied: {name}");
                    }
                }
                return ViewEvent::Consumed;
            }
            _ => {}
        }

        ViewEvent::None
    }
}

/// Fallback display name for a commodity key when no localised name is known:
/// turns "low_temperature_diamonds" / "lowtemperaturediamond" into Title Case.
fn prettify_commodity(name: &str) -> String {
    name.split(|c| c == '_' || c == ' ')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn journal_station_to_response(s: &StationData) -> StationResponse {
    StationResponse {
        market_id: s.market_id,
        system_address: 0,
        system_name: s.system_name.clone(),
        name: s.name.clone(),
        station_type: Some(s.station_type.clone()),
        faction_name: if s.faction.is_empty() { None } else { Some(s.faction.clone()) },
        government: if s.government.is_empty() { None } else { Some(s.government.clone()) },
        economy: if s.economy.is_empty() { None } else { Some(s.economy.clone()) },
        economies: s.secondary_economies.iter()
            .map(|(name, prop)| StationEconomyResponse { name: name.clone(), proportion: *prop })
            .collect(),
        services: s.services.clone(),
        landing_pads: s.landing_pads.map(|(small, medium, large)| LandingPadsResponse { small, medium, large }),
        dist_from_star_ls: if s.dist_from_star_ls > 0.0 { Some(s.dist_from_star_ls) } else { None },
        carrier_name: None,
        updated_at: None,
        market_updated_at: None,
        commodities: s.commodities.clone(),
        modules: Vec::new(),
        ships: Vec::new(),
    }
}

fn merge_into_pinned(pinned: &mut Vec<StationResponse>, api_station: StationResponse) {
    if let Some(existing) = pinned.iter_mut().find(|r| r.market_id == api_station.market_id) {
        let keep_commodities = if api_station.commodities.is_empty() {
            existing.commodities.clone()
        } else {
            api_station.commodities.clone()
        };
        *existing = api_station;
        existing.commodities = keep_commodities;
    } else {
        pinned.push(api_station);
    }
}
