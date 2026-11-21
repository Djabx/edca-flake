//! The Search tab: one field over everything edcas knows, as on edcas.de/search. All
//! shows a few matches of each kind; each other sub-tab searches one kind. Stations and
//! Factions are their full views (pins, history, details), fed from the shared field.
//! Stars finds the nearest known stars of a type, Bodies the nearest known planets of a
//! class with filters; there the field is the reference system.

mod gui;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use edcas_common::api::{
    FactionRank, FactionSearchResult, NearestCommodityQuery, NearestCommodityResult, NearestModuleResult,
    BodyClassCatalog, NearestBodies, NearestStars, OutfittingModuleInfo, StarTypeCatalog, StarTypeGroup, PopularStation, PopularSystem, StationSuggestion, SystemSearchResult, SystemSuggestion,
};

use crate::api_client::{ApiClient, ServerMessage};
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
    Stars,
    Bodies,
}

impl SearchTab {
    pub const TABS: [SearchTab; 8] = [
        SearchTab::All,
        SearchTab::Systems,
        SearchTab::Stations,
        SearchTab::Factions,
        SearchTab::Commodities,
        SearchTab::Modules,
        SearchTab::Stars,
        SearchTab::Bodies,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SearchTab::All => "All",
            SearchTab::Systems => "Systems",
            SearchTab::Stations => "Stations",
            SearchTab::Factions => "Factions",
            SearchTab::Commodities => "Commodities",
            SearchTab::Modules => "Modules",
            SearchTab::Stars => "Stars",
            SearchTab::Bodies => "Bodies",
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
            SearchTab::Stars | SearchTab::Bodies => "Reference system (empty: your current system), Enter…",
        }
    }

    /// Whether the tab searches as you type. Stations and Factions fetch whole records,
    /// and Stars and Bodies a search over millions of stars or planets, so they wait for
    /// Enter.
    fn live(self) -> bool {
        !matches!(self, SearchTab::Stations | SearchTab::Factions | SearchTab::Stars | SearchTab::Bodies)
    }

    /// Whether the field is the reference system of a nearest search (Stars, Bodies).
    fn takes_reference(self) -> bool {
        matches!(self, SearchTab::Stars | SearchTab::Bodies)
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

// ------------------------------------------------------------------- Stars

/// What the Stars sub-tab searches for: a group of star types or one type, by the name
/// the server takes in `star_type`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StarChoice {
    Group(String),
    Type(String),
}

impl StarChoice {
    /// The `star_type` token.
    pub fn token(&self) -> &str {
        match self {
            StarChoice::Group(g) | StarChoice::Type(g) => g,
        }
    }

    pub fn label(&self) -> String {
        match self {
            StarChoice::Group(g) => group_label(g),
            StarChoice::Type(t) => t.clone(),
        }
    }

    /// "Main star only" as a pilot means it: a star to scoop at (Scoopable, or one of its
    /// types O B A F G K M) or to boost at (Neutron, type N) right on arrival. Other kinds
    /// (giants, white dwarfs, carbon stars …) are worth a detour to any star of the system.
    pub fn main_star_default(&self) -> bool {
        match self {
            StarChoice::Group(g) => matches!(g.as_str(), "scoopable" | "neutron"),
            StarChoice::Type(t) => ["O", "B", "A", "F", "G", "K", "M", "N"].iter().any(|s| s.eq_ignore_ascii_case(t)),
        }
    }
}

/// The groups this client knows, with their readable labels. The picker sorts by label.
const STAR_GROUP_LABELS: &[(&str, &str)] = &[
    ("scoopable", "Scoopable (KGBFOAM)"),
    ("neutron", "Neutron"),
    ("white_dwarf", "White dwarf"),
    ("black_hole", "Black hole"),
    ("carbon", "Carbon"),
    ("wolf_rayet", "Wolf-Rayet"),
    ("s_type", "S-type"),
    ("brown_dwarf", "Brown dwarf"),
    ("proto", "Proto"),
    ("giant", "Giant"),
];

/// A group's readable label; a group this client does not know yet gets its name with
/// spaces for underscores.
pub fn group_label(name: &str) -> String {
    STAR_GROUP_LABELS
        .iter()
        .find(|(g, _)| *g == name)
        .map(|(_, l)| l.to_string())
        .unwrap_or_else(|| spaced_title(name))
}

/// `dark_matter` as "Dark matter".
fn spaced_title(name: &str) -> String {
    let spaced = name.replace('_', " ");
    let mut c = spaced.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// The picker's entries: the groups, then every single type the data has with its count
/// (`None` for a group). Each of the two parts is in alphabetical order of what the list
/// shows, whatever order the server sends (it sends the types by count). Without a catalog
/// (the request failed) it falls back to the known groups.
pub fn star_choices(catalog: Option<&StarTypeCatalog>) -> Vec<(StarChoice, Option<i64>)> {
    let groups: Vec<StarTypeGroup> = match catalog {
        Some(c) if !c.groups.is_empty() => c.groups.clone(),
        _ => STAR_GROUP_LABELS.iter().map(|(g, _)| StarTypeGroup { name: g.to_string(), types: Vec::new() }).collect(),
    };
    let by_label = |entries: &mut Vec<(StarChoice, Option<i64>)>| entries.sort_by_cached_key(|(c, _)| c.label().to_lowercase());
    let mut out: Vec<(StarChoice, Option<i64>)> = groups.into_iter().map(|g| (StarChoice::Group(g.name), None)).collect();
    by_label(&mut out);
    if let Some(c) = catalog {
        let mut types: Vec<(StarChoice, Option<i64>)> = c.types.iter().map(|t| (StarChoice::Type(t.label.clone()), Some(t.count))).collect();
        by_label(&mut types);
        out.extend(types);
    }
    out
}

/// The query pairs for `GET /api/v1/nearest/stars`, or what is wrong with the input.
/// `system` is the reference system (already defaulted to the current one); `max_distance`
/// is the field's text, empty for no limit.
pub fn star_query(choice: &StarChoice, system: &str, main_star: bool, max_distance: &str, limit: usize) -> Result<Vec<(&'static str, String)>, String> {
    let system = system.trim();
    if system.is_empty() {
        return Err("Your current system isn't known yet — type a reference system".into());
    }
    let mut q = vec![
        ("star_type", choice.token().to_string()),
        ("reference_system", system.to_string()),
        ("limit", limit.to_string()),
    ];
    if main_star {
        q.push(("main_star", "true".into()));
    }
    if let Some(d) = parse_max_distance(max_distance)? {
        q.push(("max_distance", d.to_string()));
    }
    Ok(q)
}

/// The max-distance field in light years: empty is no limit. One decimal separator, `.` or
/// `,`, is allowed ("12,5" is 12.5); a separator before exactly three digits ("1,000") could
/// be either a decimal or a thousands separator, so it is refused, as is more than one.
pub fn parse_max_distance(text: &str) -> Result<Option<f64>, String> {
    let raw = text.trim();
    let d = raw.strip_suffix("ly").unwrap_or(raw).trim();
    if d.is_empty() {
        return Ok(None);
    }
    let seps: Vec<usize> = d.char_indices().filter(|(_, c)| matches!(c, '.' | ',')).map(|(i, _)| i).collect();
    match seps.as_slice() {
        [] => {}
        [i] => {
            let (whole, frac) = (&d[..*i], &d[i + 1..]);
            if frac.len() == 3 && frac.chars().all(|c| c.is_ascii_digit()) {
                let small = format!("{whole}.{frac}").parse::<f64>().map(|v| v.to_string()).unwrap_or_else(|_| format!("{whole}.{frac}"));
                return Err(format!("{d} could be {small} or {whole}{frac}: write it without separators"));
            }
        }
        _ => return Err(format!("{d} has more than one separator: write it without separators")),
    }
    match d.replace(',', ".").parse::<f64>() {
        Ok(v) if v.is_finite() && v > 0.0 => Ok(Some(v)),
        _ => Err(format!("Max distance '{d}' isn't a number of light years above 0")),
    }
}

/// The search's failure as a status line: for a time-out (503) the client's own advice,
/// otherwise the server's reason, capitalised (with the system for an unknown one), or the
/// connection error.
pub fn star_error(e: &anyhow::Error, system: &str) -> String {
    match e.downcast_ref::<ServerMessage>() {
        // The server's reason names its own parameters; say it the client's way.
        Some(m) if m.status == 503 => "The search took too long. Set a max distance and try again.".into(),
        Some(m) => {
            let mut c = m.text.chars();
            let text: String = c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default();
            if m.status == 404 { format!("{text}: '{system}'") } else { text }
        }
        None => format!("Search failed: {e:#}"),
    }
}

/// `n` in words for the coverage line: "1,490 known stars of this kind".
pub fn known_line(n: i64) -> String {
    match n {
        0 => "No stars of this kind are known yet".into(),
        1 => "1 known star of this kind".into(),
        n => format!("{} known stars of this kind", edcas_common::format::group_thousands(n)),
    }
}

/// Rows the Stars sub-tab asks for.
const STAR_ROWS: usize = 25;

// ------------------------------------------------------------------ Bodies

/// What the Bodies sub-tab searches for: any planet, a group of planet classes or one
/// class, by the name the server takes in `planet_class`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyChoice {
    Any,
    Group(String),
    Class(String),
}

impl BodyChoice {
    /// The `planet_class` token; none for any planet.
    pub fn token(&self) -> Option<&str> {
        match self {
            BodyChoice::Any => None,
            BodyChoice::Group(g) | BodyChoice::Class(g) => Some(g),
        }
    }

    pub fn label(&self) -> String {
        match self {
            BodyChoice::Any => "Any planet".into(),
            BodyChoice::Group(g) => body_group_label(g),
            BodyChoice::Class(c) => c.clone(),
        }
    }

    /// The kind in a sentence: "No Earthlike world is known …".
    fn noun(&self) -> String {
        match self {
            BodyChoice::Any => "planet".into(),
            c => c.label(),
        }
    }
}

/// Planet class groups in the order the picker shows them, with their readable labels.
const BODY_GROUP_LABELS: &[(&str, &str)] = &[
    ("terrestrial", "Terrestrial"),
    ("high_value", "High value (ELW, WW, AW)"),
    ("elw", "Earthlike world"),
    ("ww", "Water world"),
    ("aw", "Ammonia world"),
    ("hmc", "High metal content"),
    ("gas_giant", "Gas giant"),
    ("life_giant", "Gas giant with life"),
];

/// A planet class group's readable label (a group this client does not know yet gets its
/// name with spaces for underscores).
pub fn body_group_label(name: &str) -> String {
    BODY_GROUP_LABELS.iter().find(|(g, _)| *g == name).map(|(_, l)| l.to_string()).unwrap_or_else(|| spaced_title(name))
}

/// The picker's entries: Any planet, the groups, then every class the data has; groups
/// and classes each in alphabetical order of what the list shows (the server sends the
/// classes by count). Counts come from the catalog: a class's own, a group's the sum of
/// its classes, Any the sum of all; `None` while the server has no counts. Without a
/// catalog: Any and the known groups.
pub fn body_choices(catalog: Option<&BodyClassCatalog>) -> Vec<(BodyChoice, Option<i64>)> {
    let counted = catalog.filter(|c| !c.classes.is_empty());
    let count_of = |class: &str| counted.map(|c| c.classes.iter().filter(|e| e.label == class).map(|e| e.count).sum::<i64>());
    let mut out = vec![(BodyChoice::Any, counted.map(|c| c.classes.iter().map(|e| e.count).sum()))];
    let groups: Vec<(String, Vec<String>)> = match catalog {
        Some(c) if !c.groups.is_empty() => c.groups.iter().map(|g| (g.name.clone(), g.classes.clone())).collect(),
        _ => BODY_GROUP_LABELS.iter().map(|(g, _)| (g.to_string(), Vec::new())).collect(),
    };
    let by_label = |entries: &mut Vec<(BodyChoice, Option<i64>)>| entries.sort_by_cached_key(|(c, _)| c.label().to_lowercase());
    let mut kinds: Vec<(BodyChoice, Option<i64>)> = groups
        .into_iter()
        .map(|(name, classes)| (BodyChoice::Group(name), counted.map(|_| classes.iter().filter_map(|c| count_of(c)).sum())))
        .collect();
    by_label(&mut kinds);
    out.extend(kinds);
    if let Some(c) = catalog {
        let mut classes: Vec<(BodyChoice, Option<i64>)> = c.classes.iter().map(|e| (BodyChoice::Class(e.label.clone()), Some(e.count))).collect();
        by_label(&mut classes);
        out.extend(classes);
    }
    out
}

/// A yes / no filter that may be off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tri {
    #[default]
    Any,
    Yes,
    No,
}

impl Tri {
    pub const ALL: [Tri; 3] = [Tri::Any, Tri::Yes, Tri::No];

    pub fn label(self) -> &'static str {
        match self {
            Tri::Any => "any",
            Tri::Yes => "yes",
            Tri::No => "no",
        }
    }

    fn param(self) -> Option<&'static str> {
        match self {
            Tri::Any => None,
            Tri::Yes => Some("true"),
            Tri::No => Some("false"),
        }
    }
}

/// The Bodies sub-tab's filters, as set in its controls.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BodyFilters {
    pub landable: Tri,
    pub terraformable: Tri,
    pub ringed: Tri,
    pub volcanism: Tri,
    /// An atmosphere type as the server lists it (`CarbonDioxide`, `None` for none).
    pub atmosphere: Option<String>,
    /// At least this many biological / geological signals; 0 is off.
    pub min_bio: u32,
    pub min_geo: u32,
    /// Max gravity in g as typed; empty for none.
    pub max_gravity: String,
    /// Max distance in ly as typed; empty for none.
    pub max_distance: String,
}

impl BodyFilters {
    /// Whether anything but the distance narrows the search.
    fn any_set(&self) -> bool {
        [self.landable, self.terraformable, self.ringed, self.volcanism].iter().any(|t| *t != Tri::Any)
            || self.atmosphere.is_some()
            || self.min_bio > 0
            || self.min_geo > 0
            || !self.max_gravity.trim().is_empty()
    }
}

/// The query pairs for `GET /api/v1/nearest/bodies`, or what is wrong with the input.
/// `system` is the reference system (already defaulted to the current one).
pub fn body_query(choice: &BodyChoice, system: &str, f: &BodyFilters, limit: usize) -> Result<Vec<(&'static str, String)>, String> {
    let system = system.trim();
    if system.is_empty() {
        return Err("Your current system isn't known yet — type a reference system".into());
    }
    let mut q = Vec::new();
    if let Some(t) = choice.token() {
        q.push(("planet_class", t.to_string()));
    }
    q.push(("reference_system", system.to_string()));
    q.push(("limit", limit.to_string()));
    for (key, tri) in [("landable", f.landable), ("terraformable", f.terraformable), ("ringed", f.ringed), ("volcanism", f.volcanism)] {
        if let Some(v) = tri.param() {
            q.push((key, v.to_string()));
        }
    }
    if let Some(a) = f.atmosphere.as_deref() {
        q.push(("atmosphere_type", a.to_string()));
    }
    if f.min_bio > 0 {
        q.push(("min_bio", f.min_bio.to_string()));
    }
    if f.min_geo > 0 {
        q.push(("min_geo", f.min_geo.to_string()));
    }
    if let Some(g) = parse_max_gravity(&f.max_gravity)? {
        q.push(("max_gravity", g.to_string()));
    }
    if let Some(d) = parse_max_distance(&f.max_distance)? {
        q.push(("max_distance", d.to_string()));
    }
    Ok(q)
}

/// The max-gravity field in g: empty is no limit; `.` or `,` as the one decimal separator.
pub fn parse_max_gravity(text: &str) -> Result<Option<f64>, String> {
    let raw = text.trim();
    let g = raw.strip_suffix('g').unwrap_or(raw).trim();
    if g.is_empty() {
        return Ok(None);
    }
    if g.chars().filter(|c| matches!(c, '.' | ',')).count() > 1 {
        return Err(format!("Max gravity '{g}' has more than one separator"));
    }
    match g.replace(',', ".").parse::<f64>() {
        Ok(v) if v.is_finite() && v > 0.0 => Ok(Some(v)),
        _ => Err(format!("Max gravity '{g}' isn't a number of g above 0")),
    }
}

/// The search's failure as a status line, as [`star_error`]; a 503 says whether the
/// server's index is still being built or the search took too long.
pub fn body_error(e: &anyhow::Error, system: &str) -> String {
    match e.downcast_ref::<ServerMessage>() {
        Some(m) if m.status == 503 && m.text.contains("still being built") => {
            "The body search index is still being built on the server. Try again in a few minutes.".into()
        }
        Some(m) if m.status == 503 => "The search took too long. Set a max distance or loosen the filters and try again.".into(),
        _ => star_error(e, system),
    }
}

/// `n` in words for the coverage line: "32,592 known planets of this kind".
pub fn known_bodies_line(n: i64) -> String {
    match n {
        0 => "No planets of this kind are known yet".into(),
        1 => "1 known planet of this kind".into(),
        n => format!("{} known planets of this kind", edcas_common::format::group_thousands(n)),
    }
}

/// Rows the Bodies sub-tab asks for.
const BODY_ROWS: usize = 25;

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

    /// The Stars sub-tab's reference system: what the field shows there.
    star_system: String,
    /// The type groups and types for the picker, fetched once.
    star_catalog: Option<StarTypeCatalog>,
    star_catalog_slot: Slot<StarTypeCatalog>,
    star_catalog_asked: bool,
    star_choice: Option<StarChoice>,
    star_main_only: bool,
    /// Max distance in ly as typed; empty for none.
    star_max_distance: String,
    stars: Option<NearestStars>,
    stars_slot: Slot<NearestStars>,
    /// The highlighted picker entry, while no results are shown.
    star_pick: usize,

    /// The Bodies sub-tab's reference system: what the field shows there.
    body_system: String,
    /// The class groups, classes and atmosphere types for the picker, fetched once.
    body_catalog: Option<BodyClassCatalog>,
    body_catalog_slot: Slot<BodyClassCatalog>,
    body_catalog_asked: bool,
    body_choice: Option<BodyChoice>,
    body_filters: BodyFilters,
    bodies: Option<NearestBodies>,
    bodies_slot: Slot<NearestBodies>,
    /// The highlighted picker entry, while no results are shown.
    body_pick: usize,
    clipboard: Option<arboard::Clipboard>,
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
            star_system: String::new(),
            star_catalog: None,
            star_catalog_slot: slot(),
            star_catalog_asked: false,
            star_choice: None,
            star_main_only: false,
            star_max_distance: String::new(),
            stars: None,
            stars_slot: slot(),
            star_pick: 0,
            body_system: String::new(),
            body_catalog: None,
            body_catalog_slot: slot(),
            body_catalog_asked: false,
            body_choice: None,
            body_filters: BodyFilters::default(),
            bodies: None,
            bodies_slot: slot(),
            body_pick: 0,
            clipboard: None,
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
            SearchTab::Stars => self.stars_enter(api, journal),
            SearchTab::Bodies => self.bodies_enter(api, journal),
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
            SearchTab::Stars => {
                // The field is the reference system here; nothing typed elsewhere carries over.
                self.stars_enter(api, journal);
                return;
            }
            SearchTab::Bodies => {
                self.bodies_enter(api, journal);
                return;
            }
            _ => {}
        }
        if !self.query.trim().is_empty() {
            self.submit(api, journal, stations, factions);
        }
    }

    /// Opening Stars: the picker's list is fetched once, and the reference system starts
    /// as the commander's current one.
    fn stars_enter(&mut self, api: &ApiClient, journal: &JournalData) {
        if !self.star_catalog_asked {
            self.star_catalog_asked = true;
            fetch(api, &self.star_catalog_slot, |api| async move { api.star_types().await });
        }
        if self.star_system.trim().is_empty() {
            if let Some(s) = &journal.current_system {
                self.star_system = s.name.clone();
            }
        }
    }

    /// The picker's entries (see [`star_choices`]).
    pub fn star_choices(&self) -> Vec<(StarChoice, Option<i64>)> {
        star_choices(self.star_catalog.as_ref())
    }

    /// Picks what to search for; the main-star toggle takes that kind's default.
    fn pick_star(&mut self, choice: StarChoice, api: &ApiClient, journal: &JournalData) {
        self.star_main_only = choice.main_star_default();
        self.star_choice = Some(choice);
        self.search_stars(api, journal);
    }

    /// Searches the nearest stars of the picked kind from the reference system.
    fn search_stars(&mut self, api: &ApiClient, journal: &JournalData) {
        let Some(choice) = self.star_choice.clone() else {
            self.status = "Pick a kind of star first".into();
            return;
        };
        if self.star_system.trim().is_empty() {
            if let Some(s) = &journal.current_system {
                self.star_system = s.name.clone();
            }
        }
        let system = self.star_system.trim().to_string();
        let q = match star_query(&choice, &system, self.star_main_only, &self.star_max_distance, STAR_ROWS) {
            Ok(q) => q,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        self.stars = None;
        self.selected = 0;
        self.loading = true;
        self.status = format!("{} stars nearest {system}…", choice.label());
        fetch(api, &self.stars_slot, move |api| async move {
            api.nearest_stars(&q).await.map_err(|e| anyhow::anyhow!(star_error(&e, &system)))
        });
    }

    /// Copies the selected star's system name.
    fn copy_star_system(&mut self) {
        let Some(name) = self.stars.as_ref().and_then(|s| s.results.get(self.selected)).map(|r| r.system_name.trim().to_string()) else {
            return;
        };
        self.copy_system_name(name);
    }

    /// Opening Bodies: the picker's list is fetched once, and the reference system starts
    /// as the commander's current one.
    fn bodies_enter(&mut self, api: &ApiClient, journal: &JournalData) {
        if !self.body_catalog_asked {
            self.body_catalog_asked = true;
            fetch(api, &self.body_catalog_slot, |api| async move { api.body_classes().await });
        }
        if self.body_system.trim().is_empty() {
            if let Some(s) = &journal.current_system {
                self.body_system = s.name.clone();
            }
        }
    }

    /// The picker's entries (see [`body_choices`]).
    pub fn body_choices(&self) -> Vec<(BodyChoice, Option<i64>)> {
        body_choices(self.body_catalog.as_ref())
    }

    /// Picks what to search for; the filters stay as they are.
    fn pick_body(&mut self, choice: BodyChoice, api: &ApiClient, journal: &JournalData) {
        self.body_choice = Some(choice);
        self.search_bodies(api, journal);
    }

    /// Searches the nearest planets of the picked kind from the reference system.
    fn search_bodies(&mut self, api: &ApiClient, journal: &JournalData) {
        let Some(choice) = self.body_choice.clone() else {
            self.status = "Pick a kind of planet first".into();
            return;
        };
        if self.body_system.trim().is_empty() {
            if let Some(s) = &journal.current_system {
                self.body_system = s.name.clone();
            }
        }
        let system = self.body_system.trim().to_string();
        let q = match body_query(&choice, &system, &self.body_filters, BODY_ROWS) {
            Ok(q) => q,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        self.bodies = None;
        self.selected = 0;
        self.loading = true;
        self.status = format!("{} nearest {system}…", choice.label());
        fetch(api, &self.bodies_slot, move |api| async move {
            api.nearest_bodies(&q).await.map_err(|e| anyhow::anyhow!(body_error(&e, &system)))
        });
    }

    /// Copies the selected planet's system name.
    fn copy_body_system(&mut self) {
        let Some(name) = self.bodies.as_ref().and_then(|b| b.results.get(self.selected)).map(|r| r.system_name.trim().to_string()) else {
            return;
        };
        self.copy_system_name(name);
    }

    fn copy_system_name(&mut self, name: String) {
        if self.clipboard.is_none() {
            self.clipboard = arboard::Clipboard::new().ok();
        }
        self.status = match self.clipboard.as_mut().map(|cb| cb.set_text(&name)) {
            Some(Ok(())) => format!("Copied: {name}"),
            _ => "Couldn't reach the clipboard".into(),
        };
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
            SearchTab::Stars => {
                if self.star_choice.is_none() {
                    if let Some((c, _)) = self.star_choices().get(self.star_pick).cloned() {
                        self.pick_star(c, api, journal);
                        return;
                    }
                }
                self.search_stars(api, journal);
            }
            SearchTab::Bodies => {
                if self.body_choice.is_none() {
                    if let Some((c, _)) = self.body_choices().get(self.body_pick).cloned() {
                        self.pick_body(c, api, journal);
                        return;
                    }
                }
                self.search_bodies(api, journal);
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
        if let Some(r) = take(&self.star_catalog_slot) {
            match r {
                Ok(c) => self.star_catalog = Some(c),
                // The groups still work without it (see `star_choices`).
                Err(e) => tracing::warn!(error = %e, "star types unavailable"),
            }
        }
        if let Some(r) = take(&self.stars_slot) {
            self.loading = false;
            match r {
                Ok(s) => {
                    self.status = if s.results.is_empty() {
                        let label = self.star_choice.as_ref().map(StarChoice::label).unwrap_or_default();
                        let main = if self.star_main_only { " as a main star" } else { "" };
                        match self.star_max_distance.trim() {
                            "" => format!("No {label} star is known{main}"),
                            d => format!("No {label} star is known{main} within {d} ly of {}", s.reference.name.as_deref().unwrap_or("there")),
                        }
                    } else {
                        String::new()
                    };
                    self.stars = Some(s);
                }
                Err(e) => self.status = e,
            }
        }
        if let Some(r) = take(&self.body_catalog_slot) {
            match r {
                Ok(c) => self.body_catalog = Some(c),
                // Any planet and the groups still work without it (see `body_choices`).
                Err(e) => tracing::warn!(error = %e, "planet classes unavailable"),
            }
        }
        if let Some(r) = take(&self.bodies_slot) {
            self.loading = false;
            match r {
                Ok(b) => {
                    self.status = if b.results.is_empty() {
                        let noun = self.body_choice.as_ref().map(BodyChoice::noun).unwrap_or_else(|| "planet".into());
                        let filters = if self.body_filters.any_set() { " that matches these filters" } else { "" };
                        match self.body_filters.max_distance.trim() {
                            "" => format!("No {noun} is known{filters}"),
                            d => format!("No {noun} is known{filters} within {d} ly of {}", b.reference.name.as_deref().unwrap_or("there")),
                        }
                    } else {
                        String::new()
                    };
                    self.bodies = Some(b);
                }
                Err(e) => self.status = e,
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
            SearchTab::Stars => return self.stars_key(key, api, journal),
            SearchTab::Bodies => return self.bodies_key(key, api, journal),
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

    /// Stars without a focused field: w/s move through the results (or the picker while
    /// there are none), Enter shows the star's system on the galaxy map (or searches the
    /// highlighted kind), `c` copies the system name, `m` toggles "main star only".
    fn stars_key(&mut self, key: &KeyEvent, api: &ApiClient, journal: &JournalData) -> ViewEvent {
        let results = self.stars.as_ref().map_or(0, |s| s.results.len());
        let picking = results == 0;
        match key.code {
            KeyCode::Char('w') | KeyCode::Up => {
                if picking {
                    self.star_pick = self.star_pick.saturating_sub(1);
                } else {
                    self.selected = self.selected.saturating_sub(1);
                }
            }
            KeyCode::Char('s') | KeyCode::Down => {
                if picking {
                    if self.star_pick + 1 < self.star_choices().len() {
                        self.star_pick += 1;
                    }
                } else if self.selected + 1 < results {
                    self.selected += 1;
                }
            }
            KeyCode::Enter if picking => {
                if let Some((c, _)) = self.star_choices().get(self.star_pick).cloned() {
                    self.pick_star(c, api, journal);
                }
            }
            KeyCode::Enter => return self.open_star_row(self.selected),
            KeyCode::Char('c') => self.copy_star_system(),
            KeyCode::Char('m') => {
                self.star_main_only = !self.star_main_only;
                if self.star_choice.is_some() {
                    self.search_stars(api, journal);
                }
            }
            _ => return ViewEvent::None,
        }
        ViewEvent::Consumed
    }

    /// Bodies without a focused field: w/s move through the results (or the picker while
    /// there are none), Enter shows the planet's system on the galaxy map (or searches the
    /// highlighted kind), `c` copies the system name. The same keys as Stars, without `m`.
    fn bodies_key(&mut self, key: &KeyEvent, api: &ApiClient, journal: &JournalData) -> ViewEvent {
        let results = self.bodies.as_ref().map_or(0, |b| b.results.len());
        let picking = results == 0;
        match key.code {
            KeyCode::Char('w') | KeyCode::Up => {
                if picking {
                    self.body_pick = self.body_pick.saturating_sub(1);
                } else {
                    self.selected = self.selected.saturating_sub(1);
                }
            }
            KeyCode::Char('s') | KeyCode::Down => {
                if picking {
                    if self.body_pick + 1 < self.body_choices().len() {
                        self.body_pick += 1;
                    }
                } else if self.selected + 1 < results {
                    self.selected += 1;
                }
            }
            KeyCode::Enter if picking => {
                if let Some((c, _)) = self.body_choices().get(self.body_pick).cloned() {
                    self.pick_body(c, api, journal);
                }
            }
            KeyCode::Enter => return self.open_body_row(self.selected),
            KeyCode::Char('c') => self.copy_body_system(),
            _ => return ViewEvent::None,
        }
        ViewEvent::Consumed
    }

    /// Shows result `i`'s system on the galaxy map.
    fn open_body_row(&mut self, i: usize) -> ViewEvent {
        self.selected = i;
        match self.bodies.as_ref().and_then(|b| b.results.get(i)) {
            Some(r) => ViewEvent::OpenSystem { address: r.system_address, name: r.system_name.clone() },
            None => ViewEvent::Consumed,
        }
    }

    /// Shows result `i`'s system on the galaxy map.
    fn open_star_row(&mut self, i: usize) -> ViewEvent {
        self.selected = i;
        match self.stars.as_ref().and_then(|s| s.results.get(i)) {
            Some(r) => ViewEvent::OpenSystem { address: r.system_address, name: r.system_name.clone() },
            None => ViewEvent::Consumed,
        }
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
            SearchTab::Stars => &[("/", "reference system"), ("[ ]", "kind"), ("w/s", "navigate"), ("enter", "search / map"), ("c", "copy system"), ("m", "main star only")],
            SearchTab::Bodies => &[("/", "reference system"), ("[ ]", "kind"), ("w/s", "navigate"), ("enter", "search / map"), ("c", "copy system")],
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
    fn star_groups_get_readable_labels() {
        assert_eq!(group_label("scoopable"), "Scoopable (KGBFOAM)");
        assert_eq!(group_label("wolf_rayet"), "Wolf-Rayet");
        assert_eq!(group_label("s_type"), "S-type");
        assert_eq!(group_label("white_dwarf"), "White dwarf");
        // A group the server adds later still reads well.
        assert_eq!(group_label("dark_matter"), "Dark matter");
        assert_eq!(StarChoice::Type("M_RedGiant".into()).label(), "M_RedGiant");
    }

    #[test]
    fn main_star_only_is_on_for_scooping_and_boosting() {
        assert!(StarChoice::Group("scoopable".into()).main_star_default());
        assert!(StarChoice::Group("neutron".into()).main_star_default());
        for t in ["O", "B", "A", "F", "G", "K", "M", "N", "k"] {
            assert!(StarChoice::Type(t.into()).main_star_default(), "{t}");
        }
        assert!(!StarChoice::Group("carbon".into()).main_star_default());
        assert!(!StarChoice::Group("black_hole".into()).main_star_default());
        for t in ["M_RedGiant", "MS", "DA", "CN", "H"] {
            assert!(!StarChoice::Type(t.into()).main_star_default(), "{t}");
        }
    }

    #[test]
    fn the_picker_lists_groups_then_types_each_in_alphabetical_order() {
        use edcas_common::api::CountEntry;
        let catalog = StarTypeCatalog {
            groups: vec![
                StarTypeGroup { name: "carbon".into(), types: vec!["C".into()] },
                StarTypeGroup { name: "new_kind".into(), types: vec![] },
                StarTypeGroup { name: "neutron".into(), types: vec!["N".into()] },
                StarTypeGroup { name: "scoopable".into(), types: vec!["M".into()] },
            ],
            // As the server sends them: by count.
            types: vec![
                CountEntry { label: "M".into(), count: 3_291_770 },
                CountEntry { label: "N".into(), count: 540_566 },
                CountEntry { label: "AeBe".into(), count: 13_067 },
                CountEntry { label: "DA".into(), count: 8_712 },
                CountEntry { label: "A_BlueWhiteSuperGiant".into(), count: 1_328 },
                CountEntry { label: "D".into(), count: 10 },
                CountEntry { label: "A".into(), count: 309_151 },
            ],
            computed_at: None,
        };
        let tokens: Vec<(String, Option<i64>)> = star_choices(Some(&catalog)).into_iter().map(|(c, n)| (c.token().to_string(), n)).collect();
        assert_eq!(
            tokens,
            [
                // By the label shown: Carbon, Neutron, New kind, Scoopable (KGBFOAM).
                ("carbon".into(), None),
                ("neutron".into(), None),
                ("new_kind".into(), None),
                ("scoopable".into(), None),
                ("A".into(), Some(309_151)),
                ("A_BlueWhiteSuperGiant".into(), Some(1_328)),
                ("AeBe".into(), Some(13_067)),
                ("D".into(), Some(10)),
                ("DA".into(), Some(8_712)),
                ("M".into(), Some(3_291_770)),
                ("N".into(), Some(540_566)),
            ]
        );
        // Without the server's list: the known groups, nothing else.
        let fallback = star_choices(None);
        assert_eq!(fallback.len(), STAR_GROUP_LABELS.len());
        assert_eq!(fallback[0].0, StarChoice::Group("black_hole".into()));
        assert!(fallback.iter().all(|(c, n)| matches!(c, StarChoice::Group(_)) && n.is_none()));
    }

    #[test]
    fn star_query_carries_only_what_is_set() {
        let n = StarChoice::Group("neutron".into());
        let q = star_query(&n, " Sol ", true, "", 25).unwrap();
        assert_eq!(
            q,
            [("star_type", "neutron".into()), ("reference_system", "Sol".into()), ("limit", "25".into()), ("main_star", "true".into())]
        );
        let q = star_query(&StarChoice::Type("CN".into()), "Sol", false, " 500 ly", 25).unwrap();
        assert_eq!(q.last(), Some(&("max_distance", "500".to_string())));
        assert!(!q.iter().any(|(k, _)| *k == "main_star"));
        assert_eq!(star_query(&n, "Sol", false, "12,5", 25).unwrap().last(), Some(&("max_distance", "12.5".to_string())));
        assert!(star_query(&n, "Sol", false, "far", 25).unwrap_err().contains("far"));
        assert!(star_query(&n, "Sol", false, "1,000", 25).unwrap_err().contains("without separators"));
        assert!(star_query(&n, "  ", false, "", 25).unwrap_err().contains("current system"));
    }

    #[test]
    fn max_distance_takes_one_decimal_separator_and_refuses_ambiguous_ones() {
        assert_eq!(parse_max_distance("1,000").unwrap_err(), "1,000 could be 1 or 1000: write it without separators");
        for (text, v) in [("500", 500.0), ("500 ly", 500.0), ("12,5", 12.5), ("12.5", 12.5), ("0.5", 0.5), ("  ", -1.0)] {
            let want = if v < 0.0 { None } else { Some(v) };
            assert_eq!(parse_max_distance(text), Ok(want), "{text}");
        }
        for text in ["1,000", "1.000", "12,500", "1,000.5"] {
            assert!(parse_max_distance(text).unwrap_err().contains("without separators"), "{text}");
        }
        for text in ["abc", "0", "-5"] {
            assert!(parse_max_distance(text).unwrap_err().contains("isn't a number"), "{text}");
        }
    }

    #[test]
    fn server_refusals_read_as_their_own_text() {
        let e: anyhow::Error = ServerMessage::new(404, "unknown reference system").into();
        assert_eq!(star_error(&e, "Nowhere"), "Unknown reference system: 'Nowhere'");
        let e: anyhow::Error = ServerMessage::new(503, "search took too long: add max_distance or loosen the filters").into();
        assert_eq!(star_error(&e, "Sol"), "The search took too long. Set a max distance and try again.");
        let e: anyhow::Error = ServerMessage::new(503, "").into();
        assert_eq!(star_error(&e, "Sol"), "The search took too long. Set a max distance and try again.");
        let e: anyhow::Error = ServerMessage::new(400, "unknown star type \"Q\"").into();
        assert_eq!(star_error(&e, "Sol"), "Unknown star type \"Q\"");
        let e: anyhow::Error = ServerMessage::new(502, "<html>bad gateway</html>").into();
        assert_eq!(star_error(&e, "Sol"), "The server answered 502");
        assert!(star_error(&anyhow::anyhow!("connection refused"), "Sol").starts_with("Search failed: "));
    }

    #[test]
    fn coverage_reads_in_words() {
        assert_eq!(known_line(1490), "1,490 known stars of this kind");
        assert_eq!(known_line(1), "1 known star of this kind");
        assert!(known_line(0).starts_with("No stars"));
    }

    #[test]
    fn brackets_cycle_the_sub_tabs() {
        assert_eq!(SearchTab::TABS[(SearchTab::All.index() as isize - 1).rem_euclid(8) as usize], SearchTab::Bodies);
        assert_eq!(SearchTab::TABS[SearchTab::Stars.index() + 1], SearchTab::Bodies);
        assert!(!SearchTab::Stars.live() && !SearchTab::Bodies.live());
        assert!(SearchTab::Bodies.takes_reference() && !SearchTab::Commodities.takes_reference());
        assert!(!SearchTab::Stations.live() && SearchTab::Systems.live());
    }

    fn body_catalog() -> BodyClassCatalog {
        use edcas_common::api::{BodyClassGroup, CountEntry};
        let group = |name: &str, classes: &[&str]| BodyClassGroup { name: name.into(), classes: classes.iter().map(|c| c.to_string()).collect() };
        BodyClassCatalog {
            groups: vec![
                group("gas_giant", &["Sudarsky class I gas giant"]),
                group("new_group", &[]),
                group("elw", &["Earthlike body"]),
                group("terrestrial", &["Icy body", "Rocky body", "Metal rich body"]),
            ],
            classes: vec![
                CountEntry { label: "Icy body".into(), count: 13_697_041 },
                CountEntry { label: "Rocky body".into(), count: 5_428_688 },
                CountEntry { label: "Earthlike body".into(), count: 32_592 },
            ],
            atmosphere_types: vec![CountEntry { label: "None".into(), count: 15 }],
            computed_at: None,
        }
    }

    #[test]
    fn the_body_picker_lists_any_then_groups_and_classes_in_alphabetical_order() {
        let tokens: Vec<(Option<String>, Option<i64>)> =
            body_choices(Some(&body_catalog())).into_iter().map(|(c, n)| (c.token().map(str::to_string), n)).collect();
        let s = |t: &str| Some(t.to_string());
        assert_eq!(
            tokens,
            [
                (None, Some(19_158_321)),
                // Groups by the label shown: Earthlike world, Gas giant, New group, Terrestrial.
                // They count the classes the data has (Metal rich body has none here).
                (s("elw"), Some(32_592)),
                (s("gas_giant"), Some(0)),
                (s("new_group"), Some(0)),
                (s("terrestrial"), Some(19_125_729)),
                // Classes by name, not by count as the server sends them.
                (s("Earthlike body"), Some(32_592)),
                (s("Icy body"), Some(13_697_041)),
                (s("Rocky body"), Some(5_428_688)),
            ]
        );
        // Without the server's list: Any and the known groups, no counts.
        let fallback = body_choices(None);
        assert_eq!(fallback.len(), 1 + BODY_GROUP_LABELS.len());
        assert_eq!(fallback[0], (BodyChoice::Any, None));
        assert_eq!(fallback[1].0, BodyChoice::Group("aw".into()));
        assert!(fallback[1..].iter().all(|(c, n)| matches!(c, BodyChoice::Group(_)) && n.is_none()));
        // Before the server has counted: groups without numbers, not zeros.
        let mut uncounted = body_catalog();
        uncounted.classes.clear();
        assert!(body_choices(Some(&uncounted)).iter().all(|(_, n)| n.is_none()));
    }

    #[test]
    fn body_choices_read_well() {
        assert_eq!(BodyChoice::Any.label(), "Any planet");
        assert_eq!(BodyChoice::Any.token(), None);
        assert_eq!(BodyChoice::Group("elw".into()).label(), "Earthlike world");
        assert_eq!(BodyChoice::Group("odd_worlds".into()).label(), "Odd worlds");
        assert_eq!(BodyChoice::Class("Icy body".into()).token(), Some("Icy body"));
        assert_eq!(BodyChoice::Any.noun(), "planet");
    }

    #[test]
    fn body_query_carries_only_what_is_set() {
        let terrestrial = BodyChoice::Group("terrestrial".into());
        let q = body_query(&terrestrial, " Sol ", &BodyFilters::default(), 25).unwrap();
        assert_eq!(q, [("planet_class", "terrestrial".into()), ("reference_system", "Sol".into()), ("limit", "25".into())]);
        // Any planet sends no class.
        let q = body_query(&BodyChoice::Any, "Sol", &BodyFilters::default(), 25).unwrap();
        assert!(!q.iter().any(|(k, _)| *k == "planet_class"));

        let f = BodyFilters {
            landable: Tri::Yes,
            terraformable: Tri::No,
            ringed: Tri::Any,
            volcanism: Tri::Yes,
            atmosphere: Some("CarbonDioxide".into()),
            min_bio: 3,
            min_geo: 0,
            max_gravity: " 0,5 g".into(),
            max_distance: "500 ly".into(),
        };
        let q = body_query(&terrestrial, "Plooe Thoi UW-N c6-10", &f, 25).unwrap();
        assert_eq!(
            q,
            [
                ("planet_class", "terrestrial".into()),
                ("reference_system", "Plooe Thoi UW-N c6-10".into()),
                ("limit", "25".into()),
                ("landable", "true".into()),
                ("terraformable", "false".into()),
                ("volcanism", "true".into()),
                ("atmosphere_type", "CarbonDioxide".into()),
                ("min_bio", "3".into()),
                ("max_gravity", "0.5".into()),
                ("max_distance", "500".into()),
            ]
        );
        assert!(f.any_set() && !BodyFilters { max_distance: "50".into(), ..Default::default() }.any_set());

        let bad = |f: BodyFilters| body_query(&terrestrial, "Sol", &f, 25).unwrap_err();
        assert_eq!(bad(BodyFilters { max_distance: "1,000".into(), ..Default::default() }), "1,000 could be 1 or 1000: write it without separators");
        assert!(bad(BodyFilters { max_gravity: "heavy".into(), ..Default::default() }).contains("'heavy'"));
        assert!(body_query(&terrestrial, "  ", &BodyFilters::default(), 25).unwrap_err().contains("current system"));
    }

    #[test]
    fn max_gravity_takes_one_decimal_separator() {
        for (text, v) in [("0.5", 0.5), ("0,5", 0.5), ("2 g", 2.0), ("2g", 2.0), ("0,125", 0.125), ("  ", -1.0)] {
            let want = if v < 0.0 { None } else { Some(v) };
            assert_eq!(parse_max_gravity(text), Ok(want), "{text}");
        }
        assert!(parse_max_gravity("1.0.5").unwrap_err().contains("more than one separator"));
        for text in ["abc", "0", "-1", "inf"] {
            assert!(parse_max_gravity(text).unwrap_err().contains("isn't a number"), "{text}");
        }
    }

    #[test]
    fn body_refusals_read_as_their_own_text() {
        let e: anyhow::Error = ServerMessage::new(404, "unknown reference system").into();
        assert_eq!(body_error(&e, "Nowhere"), "Unknown reference system: 'Nowhere'");
        let e: anyhow::Error = ServerMessage::new(400, "unknown atmosphere type \"Lava\"").into();
        assert_eq!(body_error(&e, "Sol"), "Unknown atmosphere type \"Lava\"");
        let e: anyhow::Error = ServerMessage::new(503, "search took too long: add max_distance, a narrower planet_class or looser filters").into();
        assert_eq!(body_error(&e, "Sol"), "The search took too long. Set a max distance or loosen the filters and try again.");
        let e: anyhow::Error = ServerMessage::new(503, "the body search index is still being built, try again in a few minutes").into();
        assert_eq!(body_error(&e, "Sol"), "The body search index is still being built on the server. Try again in a few minutes.");
        let e: anyhow::Error = ServerMessage::new(503, "").into();
        assert!(body_error(&e, "Sol").starts_with("The search took too long"));
        assert!(body_error(&anyhow::anyhow!("connection refused"), "Sol").starts_with("Search failed: "));
        assert_eq!(known_bodies_line(32_592), "32,592 known planets of this kind");
        assert_eq!(known_bodies_line(1), "1 known planet of this kind");
    }
}
