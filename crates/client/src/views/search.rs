//! The Search tab: one field over everything edcas knows, as on edcas.de/search. All
//! shows a few matches of each kind; each other sub-tab searches one kind. Stations and
//! Factions are their full views (pins, history, details), fed from the shared field.

mod gui;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use edcas_common::api::{
    FactionRank, FactionSearchResult, NearestCommodityQuery, NearestCommodityResult, NearestModuleResult,
    OutfittingModuleInfo, PopularStation, PopularSystem, StationSuggestion, SystemSearchResult, SystemSuggestion,
};

use crate::api_client::ApiClient;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::JournalData;
use crate::todo::TodoList;
use crate::views::{FactionsView, StationsView, ViewEvent};

/// A background request's answer, picked up by [`SearchView::poll`].
type Slot<T> = Arc<Mutex<Option<Result<T, String>>>>;

/// How long the field has to rest before a search-as-you-type tab searches.
const TYPING_PAUSE: Duration = Duration::from_millis(300);
/// Matches of each kind on All.
const ALL_PER_KIND: usize = 6;
/// Rows a one-kind tab lists.
const TAB_ROWS: usize = 30;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchTab {
    #[default]
    All,
    Systems,
    Stations,
    Factions,
    Commodities,
    Modules,
}

impl SearchTab {
    pub const TABS: [SearchTab; 6] = [
        SearchTab::All,
        SearchTab::Systems,
        SearchTab::Stations,
        SearchTab::Factions,
        SearchTab::Commodities,
        SearchTab::Modules,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SearchTab::All => "All",
            SearchTab::Systems => "Systems",
            SearchTab::Stations => "Stations",
            SearchTab::Factions => "Factions",
            SearchTab::Commodities => "Commodities",
            SearchTab::Modules => "Modules",
        }
    }

    fn placeholder(self) -> &'static str {
        match self {
            SearchTab::All => "System, station, faction, commodity or module…",
            SearchTab::Systems => "Search systems by name…",
            SearchTab::Stations => "Search stations & carriers by name, Enter…",
            SearchTab::Factions => "Search factions by name, Enter…",
            SearchTab::Commodities => "Commodity, e.g. Tritium…",
            SearchTab::Modules => "Module, e.g. Hpt_railgun…",
        }
    }

    /// Whether the tab searches as you type. Stations and Factions fetch whole records,
    /// so they wait for Enter.
    fn live(self) -> bool {
        !matches!(self, SearchTab::Stations | SearchTab::Factions)
    }

    fn index(self) -> usize {
        Self::TABS.iter().position(|t| *t == self).unwrap_or(0)
    }
}

/// A few matches of each kind the server searches, for All.
#[derive(Clone, Default)]
pub struct Matches {
    pub systems: Vec<SystemSuggestion>,
    pub stations: Vec<StationSuggestion>,
    pub factions: Vec<FactionSearchResult>,
}

/// What All shows before anything is typed: where commanders go most.
#[derive(Clone, Default)]
pub struct Overview {
    pub systems: Vec<PopularSystem>,
    pub stations: Vec<PopularStation>,
    pub factions: Vec<FactionRank>,
}

pub struct SearchView {
    pub tab: SearchTab,
    pub query: String,
    /// Put the caret in the field on the next frame.
    focus_field: bool,
    /// What the field showed last frame: a different `query` was set by a pick, and the
    /// caret goes to its end.
    field_text: String,
    /// When the field last changed, for the tabs that search as you type.
    edited: Option<Instant>,
    /// The query the shown All / Systems results are for.
    searched: String,
    status: String,
    loading: bool,
    /// The highlighted row of a one-kind tab's list.
    selected: usize,

    matches: Matches,
    matches_slot: Slot<Matches>,
    overview: Option<Overview>,
    overview_slot: Slot<Overview>,
    systems: Vec<SystemSearchResult>,
    systems_slot: Slot<Vec<SystemSearchResult>>,

    /// The commodity whose nearest markets are shown.
    commodity: Option<String>,
    commodity_rows: Vec<NearestCommodityResult>,
    commodity_slot: Slot<Vec<NearestCommodityResult>>,

    /// Every module some station sells, fetched once.
    modules: Vec<OutfittingModuleInfo>,
    modules_slot: Slot<Vec<OutfittingModuleInfo>>,
    modules_asked: bool,
    /// The module whose nearest sellers are shown.
    module: Option<String>,
    module_rows: Vec<NearestModuleResult>,
    module_slot: Slot<Vec<NearestModuleResult>>,
}

fn slot<T>() -> Slot<T> {
    Arc::new(Mutex::new(None))
}

/// Runs `fetch` in the background and leaves its answer in `slot`.
fn fetch<T, F>(api: &ApiClient, slot: &Slot<T>, fetch: impl FnOnce(ApiClient) -> F + Send + 'static)
where
    T: Send + 'static,
    F: std::future::Future<Output = anyhow::Result<T>> + Send + 'static,
{
    let slot = Arc::clone(slot);
    let api_owned = api.clone();
    api.spawn(async move {
        let result = fetch(api_owned).await.map_err(|e| format!("{e:#}"));
        *slot.lock().unwrap() = Some(result);
    });
}

fn take<T>(slot: &Slot<T>) -> Option<Result<T, String>> {
    slot.lock().unwrap().take()
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

impl Default for SearchView {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchView {
    pub fn new() -> Self {
        Self {
            tab: SearchTab::All,
            query: String::new(),
            focus_field: true,
            field_text: String::new(),
            edited: None,
            searched: String::new(),
            status: String::new(),
            loading: false,
            selected: 0,
            matches: Matches::default(),
            matches_slot: slot(),
            overview: None,
            overview_slot: slot(),
            systems: Vec::new(),
            systems_slot: slot(),
            commodity: None,
            commodity_rows: Vec::new(),
            commodity_slot: slot(),
            modules: Vec::new(),
            modules_slot: slot(),
            modules_asked: false,
            module: None,
            module_rows: Vec::new(),
            module_slot: slot(),
        }
    }

    /// Entering the tab: fetches what All and Modules need once, and refreshes the
    /// embedded view of the current sub-tab.
    pub fn on_enter(&mut self, api: &ApiClient, journal: &JournalData, stations: &mut StationsView, factions: &mut FactionsView) {
        self.focus_field = true;
        if self.overview.is_none() {
            fetch(api, &self.overview_slot, |api| async move {
                let (systems, stations, factions) = tokio::join!(api.popular_systems(), api.popular_stations(), api.top_factions());
                Ok(Overview { systems: systems?, stations: stations?, factions: factions? })
            });
        }
        if !self.modules_asked {
            self.modules_asked = true;
            fetch(api, &self.modules_slot, |api| async move { api.outfitting_modules().await });
        }
        match self.tab {
            SearchTab::Stations => stations.on_enter(api, journal),
            SearchTab::Factions => factions.on_enter(api),
            _ => {}
        }
    }

    /// Switches the sub-tab, searching the new one for what is in the field.
    pub fn set_tab(&mut self, tab: SearchTab, api: &ApiClient, journal: &JournalData, stations: &mut StationsView, factions: &mut FactionsView) {
        if tab == self.tab {
            return;
        }
        self.tab = tab;
        self.selected = 0;
        self.status.clear();
        self.focus_field = true;
        match tab {
            SearchTab::Stations => stations.on_enter(api, journal),
            SearchTab::Factions => factions.on_enter(api),
            _ => {}
        }
        if !self.query.trim().is_empty() {
            self.submit(api, journal, stations, factions);
        }
    }

    /// Opens Factions on `name`, from another tab (the explorer's faction links).
    pub fn open_faction(&mut self, name: &str, api: &ApiClient, factions: &mut FactionsView) {
        self.tab = SearchTab::Factions;
        self.query = name.to_string();
        factions.prefill_search(name, api);
    }

    /// The field changed.
    fn edited_now(&mut self) {
        self.edited = Some(Instant::now());
        self.selected = 0;
    }

    /// Enter in the field, or a sub-tab opened with something typed.
    fn submit(&mut self, api: &ApiClient, journal: &JournalData, stations: &mut StationsView, factions: &mut FactionsView) {
        self.edited = None;
        let q = self.query.trim().to_string();
        match self.tab {
            SearchTab::All | SearchTab::Systems => self.search_live(api),
            SearchTab::Stations => stations.search_for(&q, api),
            SearchTab::Factions => factions.prefill_search(&q, api),
            SearchTab::Commodities => {
                if let Some(name) = self.commodity_matches(1).first().copied() {
                    self.pick_commodity(name, api, journal);
                }
            }
            SearchTab::Modules => {
                if let Some(name) = self.module_matches(1).first().map(|m| m.name.clone()) {
                    self.pick_module(&name, api, journal);
                }
            }
        }
    }

    /// Searches All or Systems for the field's text.
    fn search_live(&mut self, api: &ApiClient) {
        let q = self.query.trim().to_string();
        self.searched = q.clone();
        if q.is_empty() {
            self.matches = Matches::default();
            self.systems.clear();
            self.loading = false;
            return;
        }
        self.loading = true;
        match self.tab {
            SearchTab::All => fetch(api, &self.matches_slot, move |api| async move {
                let n = ALL_PER_KIND as u32;
                let (systems, stations, factions) =
                    tokio::join!(api.suggest_systems(&q, n), api.suggest_stations(&q, n), api.find_factions(&q, n));
                Ok(Matches { systems: systems?, stations: stations?, factions: factions? })
            }),
            SearchTab::Systems => {
                fetch(api, &self.systems_slot, move |api| async move { api.search_systems(&q, TAB_ROWS as u32).await })
            }
            _ => self.loading = false,
        }
    }

    /// Shows the markets nearest the commander that sell `name`.
    fn pick_commodity(&mut self, name: &str, api: &ApiClient, journal: &JournalData) {
        let Some(system) = journal.current_system.as_ref().map(|s| s.name.clone()) else {
            self.status = "Your current system isn't known yet — start the game first".into();
            return;
        };
        self.commodity = Some(name.to_string());
        self.query = name.to_string();
        self.commodity_rows.clear();
        self.selected = 0;
        self.loading = true;
        self.status = format!("Markets selling {name} nearest {system}…");
        let query = NearestCommodityQuery { commodity: name.to_string(), reference_system: system, limit: Some(25) };
        fetch(api, &self.commodity_slot, move |api| async move { api.search_nearest_commodity(&query).await });
    }

    /// Shows the stations nearest the commander that sell module `name`.
    fn pick_module(&mut self, name: &str, api: &ApiClient, journal: &JournalData) {
        let Some(system) = journal.current_system.as_ref().map(|s| s.name.clone()) else {
            self.status = "Your current system isn't known yet — start the game first".into();
            return;
        };
        self.module = Some(name.to_string());
        self.query = name.to_string();
        self.module_rows.clear();
        self.selected = 0;
        self.loading = true;
        self.status = format!("Stations selling {name} nearest {system}…");
        let name = name.to_string();
        fetch(api, &self.module_slot, move |api| async move { api.nearest_outfitting(&name, &system, 20).await });
    }

    /// Commodities whose name contains the field's text, those starting with it first.
    fn commodity_matches(&self, n: usize) -> Vec<&'static str> {
        let q = self.query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let names = crate::views::search_nearest::commodity_names();
        let mut hits: Vec<&'static str> = names.iter().copied().filter(|c| c.to_lowercase().starts_with(&q)).collect();
        hits.extend(names.iter().copied().filter(|c| !c.to_lowercase().starts_with(&q) && c.to_lowercase().contains(&q)));
        hits.truncate(n);
        hits
    }

    /// Modules whose symbol holds every word of the field's text, the most stocked first.
    /// Symbols join their words with `_`: "railgun fixed" finds `Hpt_railgun_fixed_medium`.
    fn module_matches(&self, n: usize) -> Vec<&OutfittingModuleInfo> {
        let words: Vec<String> = self.query.split(|c: char| c.is_whitespace() || c == '_').filter(|w| !w.is_empty()).map(str::to_lowercase).collect();
        if words.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<&OutfittingModuleInfo> =
            self.modules.iter().filter(|m| words.iter().all(|w| contains_ci(&m.name, w))).collect();
        hits.sort_by_key(|m| std::cmp::Reverse(m.market_count));
        hits.truncate(n);
        hits
    }

    /// Picks up finished requests, and searches once typing pauses.
    pub fn poll(&mut self, api: &ApiClient) {
        if self.edited.is_some_and(|t| t.elapsed() >= TYPING_PAUSE) {
            self.edited = None;
            if matches!(self.tab, SearchTab::All | SearchTab::Systems) {
                self.search_live(api);
            }
        }
        if let Some(r) = take(&self.overview_slot) {
            match r {
                Ok(o) => self.overview = Some(o),
                Err(e) => self.status = format!("Couldn't load what's popular: {e}"),
            }
        }
        if let Some(r) = take(&self.modules_slot) {
            match r {
                Ok(m) => self.modules = m,
                Err(e) => {
                    self.modules_asked = false;
                    self.status = format!("Couldn't load the module list: {e}");
                }
            }
        }
        if let Some(r) = take(&self.matches_slot) {
            self.loading = false;
            match r {
                Ok(m) => self.matches = m,
                Err(e) => self.status = format!("Search failed: {e}"),
            }
        }
        if let Some(r) = take(&self.systems_slot) {
            self.loading = false;
            match r {
                Ok(s) => {
                    self.status = if s.is_empty() { format!("No systems match '{}'", self.searched) } else { String::new() };
                    self.systems = s;
                }
                Err(e) => self.status = format!("Search failed: {e}"),
            }
        }
        if let Some(r) = take(&self.commodity_slot) {
            self.loading = false;
            match r {
                Ok(rows) => {
                    self.status = match &self.commodity {
                        Some(c) if rows.is_empty() => format!("No market is known to sell {c}"),
                        _ => String::new(),
                    };
                    self.commodity_rows = rows;
                }
                Err(e) => self.status = format!("Search failed: {e}"),
            }
        }
        if let Some(r) = take(&self.module_slot) {
            self.loading = false;
            match r {
                Ok(rows) => {
                    self.status = match &self.module {
                        Some(m) if rows.is_empty() => format!("No station is known to sell {m}"),
                        _ => String::new(),
                    };
                    self.module_rows = rows;
                }
                Err(e) => self.status = format!("Search failed: {e}"),
            }
        }
    }

    /// Keys while no text field has focus. `/` goes to the field, `[` / `]` change the
    /// sub-tab; Stations and Factions get the rest.
    pub fn handle_key(
        &mut self,
        key: &KeyEvent,
        api: &ApiClient,
        journal: &JournalData,
        todo: &mut TodoList,
        stations: &mut StationsView,
        factions: &mut FactionsView,
    ) -> ViewEvent {
        match key.code {
            KeyCode::Char('/') => {
                self.focus_field = true;
                return ViewEvent::Consumed;
            }
            KeyCode::Char('[') | KeyCode::Char(']') => {
                let i = self.tab.index() as isize + if key.code == KeyCode::Char(']') { 1 } else { -1 };
                let n = SearchTab::TABS.len() as isize;
                let tab = SearchTab::TABS[i.rem_euclid(n) as usize];
                self.set_tab(tab, api, journal, stations, factions);
                return ViewEvent::Consumed;
            }
            _ => {}
        }
        let event = match self.tab {
            SearchTab::Stations => stations.handle_key(key, api, journal, todo),
            SearchTab::Factions => factions.handle_key(key, api),
            _ => self.list_key(key, api, journal, stations, factions),
        };
        if matches!(event, ViewEvent::FocusSearch) {
            self.focus_field = true;
            return ViewEvent::Consumed;
        }
        event
    }

    /// w/s and Enter in a one-kind tab's list.
    fn list_key(&mut self, key: &KeyEvent, api: &ApiClient, journal: &JournalData, stations: &mut StationsView, factions: &mut FactionsView) -> ViewEvent {
        let rows = match self.tab {
            SearchTab::Systems => self.systems.len(),
            SearchTab::Commodities if self.commodity_rows.is_empty() => self.commodity_matches(TAB_ROWS).len(),
            SearchTab::Commodities => self.commodity_rows.len(),
            SearchTab::Modules if self.module_rows.is_empty() => self.module_matches(TAB_ROWS).len(),
            SearchTab::Modules => self.module_rows.len(),
            _ => 0,
        };
        match key.code {
            KeyCode::Char('w') | KeyCode::Up if self.selected > 0 => self.selected -= 1,
            KeyCode::Char('s') | KeyCode::Down if self.selected + 1 < rows => self.selected += 1,
            KeyCode::Enter => return self.open_row(self.selected, api, journal, stations, factions),
            _ => return ViewEvent::None,
        }
        ViewEvent::Consumed
    }

    /// Opens row `i` of a one-kind tab: a system on the galaxy map, a commodity's or
    /// module's nearest sellers, or a seller in Stations.
    fn open_row(&mut self, i: usize, api: &ApiClient, journal: &JournalData, stations: &mut StationsView, factions: &mut FactionsView) -> ViewEvent {
        self.selected = i;
        match self.tab {
            SearchTab::Systems => {
                if let Some(s) = self.systems.get(i) {
                    return ViewEvent::OpenSystem { address: s.system_address, name: s.name.clone() };
                }
            }
            SearchTab::Commodities if self.commodity_rows.is_empty() => {
                if let Some(name) = self.commodity_matches(TAB_ROWS).get(i).copied() {
                    self.pick_commodity(name, api, journal);
                }
            }
            SearchTab::Commodities => {
                if let Some(r) = self.commodity_rows.get(i).cloned() {
                    self.open_station(r.market_id, &r.station_name, api, journal, stations, factions);
                }
            }
            SearchTab::Modules if self.module_rows.is_empty() => {
                if let Some(name) = self.module_matches(TAB_ROWS).get(i).map(|m| m.name.clone()) {
                    self.pick_module(&name, api, journal);
                }
            }
            SearchTab::Modules => {
                if let Some(r) = self.module_rows.get(i).cloned() {
                    self.open_station(r.market_id, &r.station_name, api, journal, stations, factions);
                }
            }
            _ => {}
        }
        ViewEvent::Consumed
    }

    /// Shows one station in the Stations sub-tab.
    fn open_station(&mut self, market_id: i64, name: &str, api: &ApiClient, journal: &JournalData, stations: &mut StationsView, factions: &mut FactionsView) {
        self.set_tab(SearchTab::Stations, api, journal, stations, factions);
        self.query = name.to_string();
        stations.open_station(market_id, name, api);
    }

    /// Shows one faction in the Factions sub-tab.
    fn open_faction_tab(&mut self, name: &str, api: &ApiClient, journal: &JournalData, stations: &mut StationsView, factions: &mut FactionsView) {
        self.set_tab(SearchTab::Factions, api, journal, stations, factions);
        self.open_faction(name, api, factions);
    }

    /// A commodity picked on All.
    fn open_commodity(&mut self, name: &str, api: &ApiClient, journal: &JournalData, stations: &mut StationsView, factions: &mut FactionsView) {
        self.query = name.to_string();
        self.set_tab(SearchTab::Commodities, api, journal, stations, factions);
        self.pick_commodity(name, api, journal);
    }

    /// A module picked on All.
    fn open_module(&mut self, name: &str, api: &ApiClient, journal: &JournalData, stations: &mut StationsView, factions: &mut FactionsView) {
        self.query = name.to_string();
        self.set_tab(SearchTab::Modules, api, journal, stations, factions);
        self.pick_module(name, api, journal);
    }

    /// The key hints for the status bar.
    pub fn hints(&self) -> &'static [(&'static str, &'static str)] {
        match self.tab {
            SearchTab::Stations => &[("/", "search"), ("[ ]", "kind"), ("w/s", "navigate"), ("tab", "panel"), ("a/d", "sub-tabs"), ("c", "copy system"), ("p", "pin"), ("m", "mark carrier"), ("t", "todo")],
            SearchTab::Factions => &[("/", "search"), ("[ ]", "kind"), ("w/s", "navigate"), ("tab", "panel"), ("a/d", "sub-tabs"), ("c", "copy system"), ("p", "pin")],
            _ => &[("/", "search"), ("[ ]", "kind"), ("w/s", "navigate"), ("enter", "open")],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commodities_starting_with_the_text_come_first() {
        let mut view = SearchView::new();
        view.query = "trit".into();
        assert_eq!(view.commodity_matches(5).first(), Some(&"Tritium"));
        view.query = "metal".into();
        let hits = view.commodity_matches(30);
        assert!(hits.iter().all(|h| h.to_lowercase().contains("metal")));
    }

    #[test]
    fn modules_match_anywhere_in_the_symbol_most_stocked_first() {
        let mut view = SearchView::new();
        view.modules = vec![
            OutfittingModuleInfo { name: "Hpt_railgun_fixed_small".into(), category: None, market_count: 3 },
            OutfittingModuleInfo { name: "Hpt_railgun_fixed_medium".into(), category: None, market_count: 9 },
            OutfittingModuleInfo { name: "Int_cargorack_size2_class1".into(), category: None, market_count: 40 },
        ];
        view.query = "RAILGUN".into();
        let names: Vec<&str> = view.module_matches(5).iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["Hpt_railgun_fixed_medium", "Hpt_railgun_fixed_small"]);
        view.query = "railgun medium".into();
        let names: Vec<&str> = view.module_matches(5).iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["Hpt_railgun_fixed_medium"]);
    }

    #[test]
    fn brackets_cycle_the_sub_tabs() {
        assert_eq!(SearchTab::TABS[(SearchTab::All.index() as isize - 1).rem_euclid(6) as usize], SearchTab::Modules);
        assert!(!SearchTab::Stations.live() && SearchTab::Systems.live());
    }
}
