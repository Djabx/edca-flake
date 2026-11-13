use std::path::PathBuf;

use tracing::{info, warn};

use crate::alerts::{Alert, Severity};
use crate::api_client::ApiClient;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::{BodyMaterial, BodyParent, BodyRing, BodyScan, Discovery, JournalData};
use crate::settings::Settings;
use crate::views::{
    CarrierView, CommanderView, ExplorerView, FactionsView, GalaxyMapView,
    NewsView, SearchNearestView, SearchView, SettingsView, StationsView,
    TodoView, TradeRoutesView, ViewEvent, WorkshopView,
};

use crate::journal_reader::JournalReader;

pub const TABS: &[&str] = &[
    "News",
    "Explorer",
    "Galaxy Map",
    "Search",
    "Trade",
    "Commander",
    "Carrier",
    "Workshop",
    "Todo",
    "Settings",
];

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum AppView {
    #[default]
    News = 0,
    Explorer = 1,
    GalaxyMap = 2,
    /// Systems, stations, factions, commodities and modules, as on edcas.de/search.
    Search = 3,
    TradeRoutes = 4,
    Commander = 5,
    Carrier = 6,
    Workshop = 7,
    Todo = 8,
    Settings = 9,
    /// Not in the tab bar — reached only via context (e.g. `f` in Todo).
    SearchNearest = 11,
}

impl AppView {
    pub fn next(&self) -> Self {
        match self {
            AppView::News => AppView::Explorer,
            AppView::Explorer => AppView::GalaxyMap,
            AppView::GalaxyMap => AppView::Search,
            AppView::Search => AppView::TradeRoutes,
            AppView::TradeRoutes => AppView::Commander,
            AppView::Commander => AppView::Carrier,
            AppView::Carrier => AppView::Workshop,
            AppView::Workshop => AppView::Todo,
            AppView::Todo => AppView::Settings,
            AppView::Settings => AppView::News,
            AppView::SearchNearest => AppView::Settings,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            AppView::News => AppView::Settings,
            AppView::Explorer => AppView::News,
            AppView::GalaxyMap => AppView::Explorer,
            AppView::Search => AppView::GalaxyMap,
            AppView::TradeRoutes => AppView::Search,
            AppView::Commander => AppView::TradeRoutes,
            AppView::Carrier => AppView::Commander,
            AppView::Workshop => AppView::Carrier,
            AppView::Todo => AppView::Workshop,
            AppView::SearchNearest => AppView::Todo,
            AppView::Settings => AppView::Todo,
        }
    }

    pub fn index(&self) -> usize {
        *self as usize
    }

    /// Maps a tab-bar position to its view, for the number-key shortcuts (`1`–`9`, `0` for
    /// the tenth; the tabs after it, such as Settings, have no number key).
    ///
    /// Only covers the tabs actually shown in the bar; `SearchNearest` is deliberately
    /// excluded because it is reached contextually (e.g. `f` in Todo), not by position.
    pub fn from_index(i: usize) -> Option<Self> {
        Some(match i {
            0 => AppView::News,
            1 => AppView::Explorer,
            2 => AppView::GalaxyMap,
            3 => AppView::Search,
            4 => AppView::TradeRoutes,
            5 => AppView::Commander,
            6 => AppView::Carrier,
            7 => AppView::Workshop,
            8 => AppView::Todo,
            9 => AppView::Settings,
            _ => return None,
        })
    }
}

pub struct App {
    pub view: AppView,
    pub settings: Settings,
    pub journal: JournalData,
    pub journal_reader: Option<JournalReader>,
    /// Owns the tokio runtime — must outlive `api` which holds a `Handle`.
    _rt: tokio::runtime::Runtime,
    pub api: ApiClient,
    /// The current system's bodies and stations from the server, keyed by its address.
    pub api_rx: Option<std::sync::mpsc::Receiver<(i64, Vec<edcas_common::api::BodyResponse>, Vec<edcas_common::api::StationSearchResult>)>>,
    pub last_api_system: i64,
    /// Bodies fetched from the server API for the current system.
    /// Re-merged on every journal update so they aren't lost when the watcher
    /// overwrites `self.journal` with a fresh data snapshot.
    cached_api_bodies: Vec<BodyScan>,
    /// Latest client build the server advertises (`/api/v1/client-version`), fetched
    /// once at startup. `None` until it arrives / if the server is unreachable; when
    /// it differs from `build_info::BUILD` the status bar flags the client outdated.
    pub(crate) latest_build: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    pub news: NewsView,
    pub commander: CommanderView,
    /// Exploration data not yet sold, followed across all journals.
    pub unsold: crate::cartography::Unsold,
    /// Organic and geological finds per body, followed across all journals.
    pub finds: crate::body_finds::Finds,
    pub carrier: CarrierView,
    pub explorer: ExplorerView,
    pub galaxy_map: GalaxyMapView,
    pub workshop: WorkshopView,
    pub stations: StationsView,
    pub factions: FactionsView,
    pub search: SearchView,
    pub trade_routes: TradeRoutesView,
    pub todo_view: TodoView,
    pub search_nearest: SearchNearestView,
    pub settings_view: SettingsView,
    /// Market IDs of fleet carriers the commander has marked as "mine".
    /// Authoritative in-memory copy of `my_carriers.json` keys; survives the
    /// wholesale `self.journal` replacement on each watcher snapshot.
    pub(crate) owned_carriers: std::collections::HashSet<i64>,
    /// Last owned-carrier cargo subset written to `my_carriers.json`, used to skip
    /// redundant disk writes when a snapshot leaves the inventory unchanged.
    last_persisted_carriers: std::collections::HashMap<i64, std::collections::HashMap<String, i32>>,
    /// Detects alert conditions from journal snapshots and the clock, and speaks them, on
    /// threads of its own; hands the fired alerts here for the toasts.
    alerts: crate::alert_service::AlertService,
    /// On-screen alert toasts awaiting display/expiry.
    pub(crate) active_toasts: Vec<Toast>,
    /// The planet ground tint EDCAS hands 3DMigoto for the body the commander is at.
    ground_tint: crate::ground_tint::GroundTint,
    /// Whether the keyboard-reference window (`?`) is showing.
    pub(crate) help_open: bool,
}

/// An alert currently shown as an on-screen toast, with the time it appeared.
pub(crate) struct Toast {
    pub(crate) alert: Alert,
    pub(crate) shown: std::time::Instant,
}

/// How long a toast stays on screen, by severity.
pub(crate) fn toast_ttl(sev: Severity) -> std::time::Duration {
    use std::time::Duration;
    match sev {
        Severity::Info => Duration::from_secs(6),
        Severity::Warning => Duration::from_secs(10),
        Severity::Critical => Duration::from_secs(20),
    }
}

impl App {
    pub fn new() -> Self {
        let settings = Settings::default();
        info!("Settings loaded");
        crate::diagnostics::log_startup(&settings);
        crate::shader_mods::maintain(&settings);
        crate::theme::set_accent(&settings.appearance.color);

        let journal_dir = if !settings.journal_reader.journal_directory.is_empty() {
            let dir = PathBuf::from(&settings.journal_reader.journal_directory);
            info!("Journal directory configured: {}", dir.display());
            Some(dir)
        } else {
            warn!("No journal directory configured");
            None
        };

        let journal = JournalData::new();

        let persisted = crate::my_carriers::MyCarriersData::load();
        let owned_carriers: std::collections::HashSet<i64> = persisted.carriers.keys().copied().collect();
        let last_persisted_carriers = persisted.carriers;

        // Writes the planet tint and clouds on a thread of its own; the reader tells it
        // where the commander is, so it needs no GUI frame.
        let ground_tint = crate::ground_tint::GroundTint::start(&settings);

        let alerts = crate::alert_service::AlertService::start(
            settings.alerts.to_config(),
            settings.alerts.voice,
            settings.alerts.voice_volume,
        );

        let journal_reader = journal_dir.map(|dir| {
            info!("Starting journal reader for directory: {}", dir.display());
            let sinks = crate::journal_reader::Sinks { place: Some(ground_tint.place()), alerts: Some(alerts.sink()) };
            JournalReader::start_with_sinks(dir, settings.edcas_api_url(), settings.eddn_config(), sinks)
        });

        let rt = tokio::runtime::Runtime::new().expect("failed to build async runtime");
        let api = ApiClient::new(&settings.api_url, rt.handle().clone());

        let mut app = Self {
            view: AppView::default(),
            settings,
            journal,
            journal_reader,
            _rt: rt,
            api,
            api_rx: None,
            last_api_system: 0,
            cached_api_bodies: Vec::new(),
            latest_build: std::sync::Arc::new(std::sync::Mutex::new(None)),
            news: NewsView::new(),
            commander: CommanderView::new(),
            unsold: Default::default(),
            finds: Default::default(),
            carrier: CarrierView::new(),
            explorer: ExplorerView::new(),
            galaxy_map: GalaxyMapView::new(),
            workshop: WorkshopView::new(),
            stations: StationsView::new(),
            factions: FactionsView::new(),
            search: SearchView::new(),
            trade_routes: TradeRoutesView::new(),
            todo_view: TodoView::new(),
            search_nearest: SearchNearestView::new(),
            settings_view: SettingsView::new(),
            owned_carriers,
            last_persisted_carriers,
            alerts,
            active_toasts: Vec::new(),
            ground_tint,
            help_open: false,
        };
        app.news.start_fetch(&app.api);
        app.refresh_client_version();
        app
    }

    /// Writes the cargo of all carriers marked "mine" to `my_carriers.json`, stamped with
    /// the latest processed event timestamp so a restart can skip already-counted transfers.
    /// No-ops on disk when the owned-carrier inventory is unchanged since the last write.
    fn persist_my_carriers(&mut self) {
        use std::collections::HashMap;
        let carriers: HashMap<i64, HashMap<String, i32>> = self.owned_carriers.iter()
            .map(|&id| {
                let cargo = self.journal.carrier_cargo.get(&id)
                    .map(|c| c.iter().filter(|(_, &v)| v != 0).map(|(k, &v)| (k.clone(), v)).collect())
                    .unwrap_or_default();
                (id, cargo)
            })
            .collect();
        if carriers == self.last_persisted_carriers {
            return;
        }
        self.last_persisted_carriers = carriers.clone();
        let data = crate::my_carriers::MyCarriersData {
            carriers,
            snapshot_timestamp: self.journal.latest_event_timestamp.clone(),
        };
        data.save();
    }

    pub fn poll_journal_updates(&mut self) {
        // Drain into a Vec first so the borrow on `self.journal_reader` is released
        // before we call methods that need full `&mut self`.
        let updates: Vec<_> = match self.journal_reader {
            Some(ref reader) => {
                let mut v = Vec::new();
                while let Some(data) = reader.try_recv() {
                    v.push(data);
                }
                v
            }
            None => Vec::new(),
        };

        // Pick up any settings changes (toggles, voice) for the alerts the reader evaluates.
        let a = &self.settings.alerts;
        self.alerts.configure(a.to_config(), a.voice, a.voice_volume);

        for data in updates {
            // This fires for *every* event, so keep it to context that's meaningful regardless
            // of the event type: which event triggered the refresh and where we are. Event-
            // specific detail (body names, scan counts, station, …) is logged at debug level in
            // `JournalData::process_line`, where the parsed event is actually available.
            let system_name = data.current_system.as_ref().map(|s| s.name.clone()).unwrap_or_default();
            info!("Journal update received: event={}, system={}", data.latest_event, system_name);
            if data.latest_event == "Shutdown" {
                // The game has let go of its files: a foreign d3d11.dll gives way to ours, old shaders to this version's.
                crate::shader_mods::maintain(&self.settings);
            }

            let new_addr = data.current_system.as_ref().map(|s| s.system_address).unwrap_or(0);
            if new_addr != 0 && new_addr != self.last_api_system && !self.settings.api_url.is_empty() {
                self.last_api_system = new_addr;
                self.cached_api_bodies.clear();
                self.ground_tint.set_api_bodies(&[]);
                let (tx, rx) = std::sync::mpsc::channel();
                self.api_rx = Some(rx);
                let api = self.api.clone();
                info!("Fetching bodies and stations from API for system {}", new_addr);
                self.api.spawn(async move {
                    let (bodies, stations) = tokio::join!(api.get_bodies(new_addr), api.get_system_stations(new_addr));
                    let bodies = bodies.unwrap_or_else(|e| {
                        warn!("API bodies fetch failed for system {}: {:#}", new_addr, e);
                        Vec::new()
                    });
                    let stations = stations.unwrap_or_else(|e| {
                        warn!("API stations fetch failed for system {}: {:#}", new_addr, e);
                        Vec::new()
                    });
                    info!("API returned {} bodies, {} stations for system {}", bodies.len(), stations.len(), new_addr);
                    let _ = tx.send((new_addr, bodies, stations));
                });
            }

            self.journal = data;

            // The watcher's snapshot only knows about carriers it seeded at *its* startup,
            // so re-assert marks made (or carriers marked mine) later this session. This keeps
            // the "mine" indicator and inventory stable across the wholesale journal swap.
            let owned: Vec<i64> = self.owned_carriers.iter().copied().collect();
            for id in owned {
                self.journal.carrier_cargo.entry(id).or_default();
                self.journal.carrier_cargo_seeded.insert(id);
            }

            // Keep the stations view selection on the same station even when
            // visited_stations changes order (e.g., a new dock prepends a station).
            self.stations.on_journal_update(&self.journal);

            // Note: we intentionally do NOT fetch server market data on dock. The game's
            // local Market.json for the station you're docked at is the freshest source;
            // server data may be from an earlier session and stale. Live data is requested
            // lazily (poll_lazy_fetch) only for selected stations that have no session data.

            // Re-merge API bodies: the watcher snapshot doesn't include them,
            // so without this they're lost on every subsequent journal update.
            let local_ids: std::collections::HashSet<i32> =
                self.journal.bodies.iter().map(|b| b.body_id).collect();
            for body in &self.cached_api_bodies {
                if !local_ids.contains(&body.body_id) {
                    self.journal.bodies.push(body.clone());
                }
            }
            self.explorer.update(&self.journal);

            self.stations.update_construction_from_journal(&self.api, &self.journal, &mut self.todo_view.todo);

            // Persist owned-carrier inventory after applying this snapshot; the helper
            // skips the write when nothing changed, so this is cheap for non-cargo events.
            self.persist_my_carriers();
        }

        // The writer thread follows Status.json's body on its own; it only needs the
        // current settings from here.
        self.ground_tint.sync(&self.settings);
        self.settings_view.cloud_status = crate::ground_tint::GroundTint::cloud_status(&self.settings, &self.journal);

        let fired = self.alerts.fired();
        self.show_alerts(fired);
        self.prune_toasts();

        self.trade_routes.poll_results();

        if let Some(ref rx) = self.api_rx {
            if let Ok((addr, api_bodies, api_stations)) = rx.try_recv() {
                self.api_rx = None;
                self.explorer.set_server_stations(addr, &api_stations);
                let local_ids: std::collections::HashSet<i32> =
                    self.journal.bodies.iter().map(|b| b.body_id).collect();
                let mut added = 0usize;
                for br in &api_bodies {
                    if !local_ids.contains(&br.id) {
                        let body = body_from_api(br);
                        self.cached_api_bodies.push(body.clone());
                        self.journal.bodies.push(body);
                        added += 1;
                    }
                }
                if added > 0 {
                    self.ground_tint.set_api_bodies(&self.cached_api_bodies);
                    info!("Merged {} bodies from API into explorer", added);
                }
                self.explorer.update(&self.journal);
            }
        }
    }

    /// Everything the UI loop does between frames: apply journal snapshots, collect
    /// background search/fetch results and bulk-upload progress.
    pub fn tick(&mut self) {
        self.poll_journal_updates();
        self.poll_search_results();
        self.settings_view.poll_bulk_upload();
        self.settings_view.poll_shader_mods();
    }

    pub fn restart_journal_reader(&mut self) {
        let dir = PathBuf::from(&self.settings.journal_reader.journal_directory);
        let url = self.settings.edcas_api_url();
        let eddn = self.settings.eddn_config();
        if dir.exists() {
            info!("Restarting journal reader with directory: {}", dir.display());
            if let Some(ref mut reader) = self.journal_reader {
                reader.restart(dir, url, eddn);
            } else {
                let sinks = crate::journal_reader::Sinks {
                    place: Some(self.ground_tint.place()),
                    alerts: Some(self.alerts.sink()),
                };
                self.journal_reader = Some(JournalReader::start_with_sinks(dir, url, eddn, sinks));
            }
        } else {
            warn!("Journal directory does not exist: {}", dir.display());
        }
    }

    /// Fetch the latest available client build from the server (once, in the
    /// background). The status bar compares it to this build to flag outdated.
    pub fn refresh_client_version(&mut self) {
        let slot = std::sync::Arc::clone(&self.latest_build);
        let api = self.api.clone();
        self.api.spawn(async move {
            if let Ok(build) = api.get_latest_build().await {
                *slot.lock().unwrap() = Some(build);
            }
        });
    }

    /// Shows fired alerts as toasts (the alert service has logged and spoken them already).
    fn show_alerts(&mut self, alerts: Vec<Alert>) {
        for a in alerts {
            self.active_toasts.push(Toast { alert: a, shown: std::time::Instant::now() });
        }
        // Keep only the most recent few on screen.
        const MAX_TOASTS: usize = 5;
        if self.active_toasts.len() > MAX_TOASTS {
            let drop = self.active_toasts.len() - MAX_TOASTS;
            self.active_toasts.drain(0..drop);
        }
    }

    /// Drops toasts that have outlived their severity-dependent display time.
    fn prune_toasts(&mut self) {
        self.active_toasts.retain(|t| t.shown.elapsed() < toast_ttl(t.alert.severity));
    }

    pub(crate) fn view_hints(&self) -> &'static [(&'static str, &'static str)] {
        match self.view {
            AppView::News        => &[("w/s", "scroll"), ("pgup/pgdn", "jump")],
            AppView::Commander   => &[("tab", "switch view"), ("w/s", "scroll/navigate")],
            AppView::Carrier     => &[("a/d", "carriers"), ("r", "refresh")],
            AppView::Explorer    => &[("tab", "panel"), ("w/s", "factions"), ("a/d", "systems"), ("space", "open")],
            AppView::GalaxyMap   => &[("/", "search"), ("c", "center on you"), ("g", "whole galaxy"), ("wasd", "move"), ("+/-", "zoom"), ("↑/↓", "bodies"), ("esc", "clear")],
            AppView::Workshop    => &[("tab", "switch view"), ("w/s", "navigate"), ("a/d", "panels/grade")],
            AppView::Search      => self.search.hints(),
            AppView::TradeRoutes => &[("tab", "cycle panels"), ("w/s", "navigate"), ("c", "copy system(s)"), ("r", "refresh")],
            AppView::Todo        => &[("tab", "switch panel"), ("w/s", "navigate"), ("r/Del", "remove"), ("f", "search nearest")],
            AppView::SearchNearest => &[("w/s", "navigate"), ("q", "back to Todo")],
            AppView::Settings    => &[("w/s", "rows"), ("a", "sidebar"), ("d", "fields"), ("space", "toggle"), ("enter", "save")],
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) {
        // The help window takes no keys but the ones that close it.
        if self.help_open {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
                self.help_open = false;
            }
            return;
        }

        let event = match self.view {
            AppView::News => self.news.handle_key(key),
            AppView::Commander => self.commander.handle_key(key, &self.journal),
            AppView::Carrier => self.carrier.handle_key(key, &self.api, &self.owned_carriers),
            AppView::Explorer => self.explorer.handle_key(key),
            AppView::GalaxyMap => self.galaxy_map.handle_key(key, &self.journal),
            AppView::Workshop => self.workshop.handle_key(key),
            AppView::Search => self.search.handle_key(key, &self.api, &self.journal, &mut self.todo_view.todo, &mut self.stations, &mut self.factions),
            AppView::TradeRoutes => self.trade_routes.handle_key(key, &self.api, &self.journal),
            AppView::Todo => self.todo_view.handle_key(key, &self.journal),
            AppView::SearchNearest => self.search_nearest.handle_key(key, &self.api),
            AppView::Settings => self.settings_view.handle_key(key, &mut self.settings),
        };

        if self.apply_view_event(event) {
            return;
        }
        self.handle_global_key(key);
    }

    /// Acts on what a view asked for, from a keypress or a click. Returns whether the
    /// input was handled; `false` (a `ViewEvent::None`) lets global shortcuts see the key.
    pub fn apply_view_event(&mut self, event: ViewEvent) -> bool {
        match event {
            ViewEvent::Consumed => true,
            ViewEvent::SettingsChanged => {
                info!("Settings changed, saving");
                self.settings_view.save_settings(&self.settings);
                // The game exports its shaders only while something switched on needs one.
                crate::shader_mods::sync_export(&self.settings);
                // Apply the accent colour live so editing it in Settings takes effect immediately.
                crate::theme::set_accent(&self.settings.appearance.color);
                self.restart_journal_reader();
                true
            }
            ViewEvent::OpenFactions(name) => {
                info!("Opening factions tab for: {}", name);
                self.search.open_faction(&name, &self.api, &mut self.factions);
                self.view = AppView::Search;
                true
            }
            ViewEvent::OpenSystem { address, name } => {
                info!("Showing {name} on the galaxy map");
                self.select_view(AppView::GalaxyMap);
                self.galaxy_map.show_system(&self.api, address, name);
                true
            }
            ViewEvent::FocusSearch => true,
            ViewEvent::OpenSearchNearest { commodity, canonical_name, system, ship_pad_size } => {
                info!("Opening Search Nearest: commodity={}, canonical={}, system={}, pad={}", commodity, canonical_name, system, ship_pad_size);
                self.search_nearest.prefill_and_search(&commodity, &canonical_name, &system, ship_pad_size, &self.api);
                self.view = AppView::SearchNearest;
                true
            }
            ViewEvent::OpenMultiSearch { commodities, canonical_names, system, ship_pad_size } => {
                info!("Opening Supply Scout: {} commodities near {}, pad={}", commodities.len(), system, ship_pad_size);
                self.search_nearest.start_multi_search(commodities, canonical_names, system, ship_pad_size, &self.api);
                self.view = AppView::SearchNearest;
                true
            }
            ViewEvent::TrackConstruction(market_id) => {
                info!("Tracking construction site {} from Stations pin", market_id);
                self.stations.track_construction(market_id, &self.api);
                true
            }
            ViewEvent::ToggleMyCarrier(market_id) => {
                if self.owned_carriers.remove(&market_id) {
                    info!("Unmarked carrier {} as mine", market_id);
                    self.journal.carrier_cargo.remove(&market_id);
                    self.journal.carrier_cargo_seeded.remove(&market_id);
                } else {
                    info!("Marked carrier {} as mine", market_id);
                    self.owned_carriers.insert(market_id);
                    // Keep any cargo already tallied live this session; default to empty.
                    self.journal.carrier_cargo.entry(market_id).or_default();
                    self.journal.carrier_cargo_seeded.insert(market_id);
                }
                self.persist_my_carriers();
                true
            }
            ViewEvent::TestVoice => {
                let a = &self.settings.alerts;
                self.alerts.configure(a.to_config(), a.voice, a.voice_volume);
                self.alerts.say(format!("Voice alerts ready. Volume {} percent.", a.voice_volume));
                true
            }
            ViewEvent::SaveDiagnostics => {
                let live = crate::diagnostics::Live::of(&self.settings, &self.journal);
                self.settings_view.start_diagnostics(&self.settings, live);
                true
            }
            ViewEvent::Back => {
                self.go_prev_tab();
                true
            }
            ViewEvent::None => false,
        }
    }

    /// Shortcuts that work on every tab, for keys the current view didn't claim.
    fn handle_global_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Char('e') => { self.go_next_tab(); }
            KeyCode::Char('q') => { self.go_prev_tab(); }
            KeyCode::Char('?') => self.help_open = true,
            // Toasts sit over the content area for up to 20s (Critical) and can cover
            // the very rows the commander wants to read. Views see Esc first, so this
            // only fires when nothing else claimed it.
            KeyCode::Esc if !self.active_toasts.is_empty() => {
                self.active_toasts.clear();
            }
            // Jump straight to a tab by its number in the bar. Nine tabs behind a
            // sequential q/e cycle meant up to eight keypresses to cross the app.
            // No view binds a digit, so these only ever reach here unconsumed —
            // and Settings' edit mode returns early, so typing numbers into a
            // field still works.
            KeyCode::Char(c @ '0'..='9') => {
                // `0` is the tenth tab, as on a keyboard's number row.
                let idx = (c as usize - '0' as usize + 9) % 10;
                if let Some(view) = AppView::from_index(idx) {
                    if view != self.view {
                        self.view = view;
                        info!("Tab changed to: {}", TABS.get(idx).copied().unwrap_or("?"));
                        self.on_tab_enter();
                    }
                }
            }
            _ => {}
        }
    }

    /// Switches to `view` (e.g. from a click on the tab bar), running its on-enter refresh.
    pub fn select_view(&mut self, view: AppView) {
        if view != self.view {
            self.view = view;
            info!("Tab changed to: {}", TABS.get(view.index()).copied().unwrap_or("SearchNearest"));
            self.on_tab_enter();
        }
    }

    fn go_next_tab(&mut self) {
        self.view = self.view.next();
        info!("Tab changed to: {}", TABS.get(self.view.index()).copied().unwrap_or("SearchNearest"));
        self.on_tab_enter();
    }

    fn go_prev_tab(&mut self) {
        self.view = self.view.prev();
        info!("Tab changed to: {}", TABS.get(self.view.index()).copied().unwrap_or("SearchNearest"));
        self.on_tab_enter();
    }

    pub fn on_tab_enter(&mut self) {
        match self.view {
            AppView::News => self.news.start_fetch(&self.api),
            AppView::GalaxyMap => self.galaxy_map.on_enter(&self.api),
            AppView::Carrier => self.carrier.on_enter(&self.settings),
            AppView::Search => self.search.on_enter(&self.api, &self.journal, &mut self.stations, &mut self.factions),
            AppView::TradeRoutes => {
                let journal = &self.journal;
                self.trade_routes.on_enter(&self.api, journal);
            }
            AppView::Todo => {
                // The Engineers subtab writes engineering todos to its own TodoList
                // copy (workshop.engineers.todo) and persists them to disk. This view
                // holds a separate copy, so reload from disk on entry to pick those up.
                self.todo_view.todo = crate::todo::TodoList::load();
            }
            AppView::Workshop => {
                // Reload so the Engineers subtab's dedup counts reflect any todo
                // changes made in the Todo tab (remove/decrement) since we last saw it.
                self.workshop.engineers.todo = crate::todo::TodoList::load();
            }
            _ => {}
        }
    }

    pub fn poll_search_results(&mut self) {
        self.news.poll();
        self.factions.poll_search();
        self.stations.poll_search();
        self.stations.poll_auto_fetch();
        self.search.poll(&self.api);
        if matches!(self.view, AppView::Search) && self.search.tab == crate::views::search::SearchTab::Stations {
            self.stations.poll_lazy_fetch(&self.api, &self.journal);
        }
        self.stations.poll_construction();
        self.search_nearest.poll_search();
        self.galaxy_map.poll(&self.api);
        self.explorer.poll_history(&self.api);
        self.explorer.poll_scans(&self.settings.journal_reader.journal_directory);
        self.unsold.follow(&self.settings.journal_reader.journal_directory);
        let unsold = self.unsold.data();
        self.explorer.unsold = std::sync::Arc::clone(&unsold);
        self.commander.set_unsold(unsold);
        self.finds.follow(&self.settings.journal_reader.journal_directory);
        self.explorer.finds = self.finds.data();
        self.carrier.poll(&self.api, &self.owned_carriers);
    }
}

pub fn body_from_api(br: &edcas_common::api::BodyResponse) -> BodyScan {
    let rings = br
        .rings
        .iter()
        .map(|r| BodyRing {
            name: r.name.clone(),
            ring_class: r.ring_class.clone(),
        })
        .collect();

    let materials = br
        .materials
        .iter()
        .map(|m| BodyMaterial {
            name: m.name.clone(),
            percent: m.percent,
        })
        .collect();

    let parents = br
        .parents
        .iter()
        .map(|p| BodyParent {
            body_id: p.parent_id,
        })
        .collect();

    BodyScan {
        body_id: br.id,
        body_name: br.name.clone(),
        planet_class: if br.is_star {
            String::new()
        } else {
            br.body_class.clone().unwrap_or_default()
        },
        landable: br.landable,
        scan_type: "API".into(),
        distance_from_arrival_ls: br.distance_from_arrival_ls.unwrap_or(0.0),
        radius: br.radius.unwrap_or(0.0),
        mass_em: br.mass_em.unwrap_or(0.0),
        surface_temperature: br.surface_temperature.unwrap_or(0.0),
        surface_gravity: br.surface_gravity.unwrap_or(0.0),
        tidal_lock: br.tidal_lock,
        volcanism: br.volcanism.clone().unwrap_or_default(),
        atmosphere: br.atmosphere.clone().unwrap_or_default(),
        terraform_state: br.terraform_state.clone().unwrap_or_default(),
        star_type: if br.is_star {
            br.body_class.clone().unwrap_or_default()
        } else {
            String::new()
        },
        parents,
        rings,
        materials,
        value: None,
        mapped: None,
        footfall: false,
        composition: None,
        discovery: Some(Discovery {
            discovered: br.was_discovered,
            mapped: br.was_mapped,
            footfalled: br.was_footfalled,
            reported: true,
        }),        exo: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `1`–`9` shortcuts map a tab-bar position to a view, and the bar is drawn
    /// straight from `TABS`. If someone adds a tab to `TABS` without extending
    /// `from_index`, the digit would silently land on the wrong view (or nothing at
    /// all), so pin the two together.
    #[test]
    fn from_index_covers_every_tab_in_the_bar() {
        for i in 0..TABS.len() {
            let view = AppView::from_index(i)
                .unwrap_or_else(|| panic!("no view mapped for tab {} ({})", i, TABS[i]));
            assert_eq!(
                view.index(),
                i,
                "tab {} ({}) maps to a view whose index is {}",
                i,
                TABS[i],
                view.index()
            );
        }
    }

    #[test]
    fn from_index_stops_at_the_end_of_the_bar() {
        assert!(AppView::from_index(TABS.len()).is_none());
        assert!(AppView::from_index(99).is_none());
    }

    /// SearchNearest is reached contextually (`f` in Todo), never by position, so it
    /// must stay out of the numbered bar.
    #[test]
    fn search_nearest_is_not_reachable_by_number() {
        for i in 0..TABS.len() {
            assert_ne!(AppView::from_index(i), Some(AppView::SearchNearest));
        }
    }
}
