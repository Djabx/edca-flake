mod gui;

use crate::event_shim::{KeyCode, KeyEvent};
use edcas_common::api::{FactionQuery, FactionResponse, InfluencePoint};
use ratatui::style::Color;
use std::cmp::Ordering;
use std::collections::HashSet;

use crate::api_client::ApiClient;
use crate::views::util::{FocusArea, SearchState};
use crate::views::ViewEvent;

use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq)]
enum DetailTab {
    Info,
    Systems,
    History,
}

#[derive(Clone, Copy, PartialEq)]
enum SystemSort {
    Name,
    Influence,
    ActiveState,
    Pending,
    Updated,
}

impl SystemSort {
    fn default_dir(self) -> SortDir {
        match self {
            Self::Influence | Self::Updated => SortDir::Desc,
            _ => SortDir::Asc,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum SortDir { Asc, Desc }

impl DetailTab {
    fn next(self) -> Option<Self> {
        match self {
            Self::Info    => Some(Self::Systems),
            Self::Systems => Some(Self::History),
            Self::History => None,
        }
    }
    fn prev(self) -> Option<Self> {
        match self {
            Self::Info    => None,
            Self::Systems => Some(Self::Info),
            Self::History => Some(Self::Systems),
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Info    => "Info",
            Self::Systems => "Systems",
            Self::History => "History",
        }
    }
}

pub struct FactionsView {
    /// Shown inside the Search tab, whose field does the searching: no field of its own.
    pub embedded: bool,
    search_query: String,
    search_state: SearchState,
    results: Vec<FactionResponse>,
    pinned_names: HashSet<String>,
    pinned_results: Vec<FactionResponse>,
    selected_idx: usize,
    selected_system: usize,
    detail_scroll: usize,
    status_msg: String,
    focus: FocusArea,
    detail_tab: DetailTab,
    loading: bool,
    system_sort: SystemSort,
    system_sort_dir: SortDir,
    clipboard: Option<arboard::Clipboard>,
    loading_pins: bool,
    pending_pins: Arc<Mutex<Option<Result<Vec<FactionResponse>, String>>>>,
    pending_search: Arc<Mutex<Option<Result<Vec<FactionResponse>, String>>>>,

    // History tab state
    history_faction_name: String,
    history_system_name: String,
    /// Pre-computed chart points: (days_since_first, influence_percent)
    history_data: Vec<(f64, f64)>,
    history_time_bounds: [f64; 2],
    /// Unix time of the first chart point (x = 0), for dating points in the window's chart.
    history_start_ts: i64,
    history_y_bounds: [f64; 2],
    history_x_labels: Vec<String>,
    loading_history: bool,
    pending_history: Arc<Mutex<Option<Result<Vec<InfluencePoint>, String>>>>,
}

impl FactionsView {
    pub fn new() -> Self {
        let pins = crate::pins::Pins::load();
        Self {
            embedded: false,
            search_query: String::new(),
            search_state: SearchState::Idle,
            results: Vec::new(),
            pinned_names: pins.factions,
            pinned_results: Vec::new(),
            selected_idx: 0,
            selected_system: 0,
            detail_scroll: 0,
            status_msg: "/ or f: search  |  p: pin/unpin".into(),
            focus: FocusArea::List,
            detail_tab: DetailTab::Info,
            loading: false,
            system_sort: SystemSort::Influence,
            system_sort_dir: SortDir::Desc,
            // Kept alive for the duration of the view so the X11/Wayland
            // background clipboard-serving thread keeps running after set_text.
            clipboard: arboard::Clipboard::new().ok(),
            loading_pins: false,
            pending_pins: Arc::new(Mutex::new(None)),
            pending_search: Arc::new(Mutex::new(None)),
            history_faction_name: String::new(),
            history_system_name: String::new(),
            history_data: Vec::new(),
            history_time_bounds: [0.0, 1.0],
            history_start_ts: 0,
            history_y_bounds: [0.0, 100.0],
            history_x_labels: Vec::new(),
            loading_history: false,
            pending_history: Arc::new(Mutex::new(None)),
        }
    }

    pub fn poll_search(&mut self) {
        if let Some(result) = self.pending_pins.lock().unwrap().take() {
            self.loading_pins = false;
            match result {
                Ok(results) => {
                    self.pinned_results = results;
                    if self.status_msg.starts_with("Loading") {
                        self.status_msg = "/ or f: search  |  p: pin/unpin".into();
                    }
                }
                Err(e) => { self.status_msg = format!("API error loading pins: {e}"); }
            }
        }
        if let Some(result) = self.pending_search.lock().unwrap().take() {
            self.loading = false;
            match result {
                Ok(results) => {
                    let count = results.len();
                    self.results = results;
                    self.selected_idx = 0;
                    self.selected_system = 0;
                    self.detail_scroll = 0;
                    self.focus = FocusArea::List;
                    self.detail_tab = DetailTab::Info;
                    self.status_msg = if count == 0 {
                        format!("No factions found for '{}'", self.search_query)
                    } else {
                        format!("{count} faction(s) found")
                    };
                }
                Err(e) => { self.status_msg = format!("API error: {e}"); }
            }
        }

        let history_result = self.pending_history.lock().unwrap().take();
        if let Some(result) = history_result {
            self.loading_history = false;
            match result {
                Ok(points) => self.set_history_data(points),
                Err(e) => {
                    self.history_data.clear();
                    self.history_x_labels = vec![format!("Error: {e}")];
                }
            }
        }
    }

    pub fn on_enter(&mut self, api: &ApiClient) {
        if !self.pinned_names.is_empty() && self.pinned_results.is_empty() && !self.loading_pins {
            self.refresh_pins(api);
        }
    }

    fn refresh_pins(&mut self, api: &ApiClient) {
        self.loading_pins = true;
        self.pinned_results.clear();
        self.status_msg = "Loading pinned factions…".into();
        let pending = Arc::clone(&self.pending_pins);
        let api_owned = api.clone();
        let names: Vec<String> = self.pinned_names.iter().cloned().collect();
        api.spawn(async move {
            let mut results = Vec::new();
            for name in names {
                let query = FactionQuery { name: Some(name.clone()), limit: Some(10) };
                if let Ok(factions) = api_owned.search_factions(&query).await {
                    if let Some(f) = factions.into_iter().find(|f| f.name == name) {
                        results.push(f);
                    }
                }
            }
            results.sort_by(|a, b| a.name.cmp(&b.name));
            *pending.lock().unwrap() = Some(Ok(results));
        });
    }

    fn save_pins(&self) {
        let mut pins = crate::pins::Pins::load();
        pins.factions = self.pinned_names.clone();
        pins.save();
    }

    fn display_count(&self) -> usize {
        let n_search = self.results.iter()
            .filter(|f| !self.pinned_names.contains(&f.name))
            .count();
        self.pinned_results.len() + n_search
    }

    fn selected_faction(&self) -> Option<&FactionResponse> {
        let n = self.pinned_results.len();
        if self.selected_idx < n {
            self.pinned_results.get(self.selected_idx)
        } else {
            let j = self.selected_idx - n;
            self.results.iter()
                .filter(|f| !self.pinned_names.contains(&f.name))
                .nth(j)
        }
    }

    fn toggle_pin(&mut self, api: &ApiClient) {
        let n = self.pinned_results.len();
        if self.selected_idx < n {
            // Unpin
            let name = self.pinned_results[self.selected_idx].name.clone();
            self.pinned_names.remove(&name);
            self.pinned_results.remove(self.selected_idx);
            let total = self.display_count();
            if total > 0 {
                self.selected_idx = self.selected_idx.min(total - 1);
            } else {
                self.selected_idx = 0;
            }
        } else {
            // Pin
            let j = self.selected_idx - n;
            if let Some(faction) = self.results.iter()
                .filter(|f| !self.pinned_names.contains(&f.name))
                .nth(j)
                .cloned()
            {
                let name = faction.name.clone();
                self.pinned_names.insert(name.clone());
                self.pinned_results.push(faction);
                self.pinned_results.sort_by(|a, b| a.name.cmp(&b.name));
                if let Some(pos) = self.pinned_results.iter().position(|f| f.name == name) {
                    self.selected_idx = pos;
                }
            } else if !self.pinned_names.is_empty() {
                self.refresh_pins(api);
            }
        }
        self.save_pins();
    }

    fn do_search(&mut self, api: &ApiClient) {
        if self.search_query.is_empty() {
            self.status_msg = "Enter a search term first".into();
            return;
        }
        self.loading = true;
        self.status_msg = format!("Searching for '{}'…", self.search_query);
        let pending = Arc::clone(&self.pending_search);
        let api_owned = api.clone();
        let query = FactionQuery { name: Some(self.search_query.clone()), limit: Some(100) };
        api.spawn(async move {
            let result = api_owned.search_factions(&query).await.map_err(|e| e.to_string());
            *pending.lock().unwrap() = Some(result);
        });
    }

    fn fetch_history(&mut self, api: &ApiClient) {
        // Collect needed data while self is borrowed immutably, then release the borrow.
        let info = {
            let faction = match self.selected_faction() {
                Some(f) => f,
                None => return,
            };
            let indices = self.sorted_system_indices(faction);
            let sys_i = match indices.get(self.selected_system).copied() {
                Some(i) => i,
                None => return,
            };
            match faction.presences.get(sys_i) {
                Some(p) => (faction.name.clone(), p.system_name.clone(), p.system_address),
                None => return,
            }
        };
        let (faction_name, system_name, system_address) = info;

        self.history_faction_name = faction_name.clone();
        self.history_system_name = system_name;
        self.loading_history = true;
        self.history_data.clear();
        self.history_x_labels.clear();

        let pending = Arc::clone(&self.pending_history);
        let api_clone = api.clone();

        api.spawn(async move {
            let result = api_clone
                .fetch_faction_influence_history(&faction_name, system_address, 90)
                .await
                .map_err(|e| e.to_string());
            *pending.lock().unwrap() = Some(result);
        });
    }

    fn set_history_data(&mut self, mut points: Vec<InfluencePoint>) {
        if points.is_empty() {
            self.history_data.clear();
            self.history_x_labels = vec!["No data yet".to_string()];
            return;
        }

        // Deduplicate: keep one point per hour (last one wins) to reduce noise.
        points.dedup_by(|a, b| {
            let ta = a.timestamp.timestamp() / 3600;
            let tb = b.timestamp.timestamp() / 3600;
            ta == tb
        });

        self.history_start_ts = points[0].timestamp.timestamp();
        let min_ts = self.history_start_ts as f64;
        let max_ts = points.last().unwrap().timestamp.timestamp() as f64;
        let total_days = ((max_ts - min_ts) / 86400.0).max(0.5);

        self.history_data = points
            .iter()
            .map(|p| {
                let x = (p.timestamp.timestamp() as f64 - min_ts) / 86400.0;
                let y = (p.influence * 100.0) as f64;
                (x, y)
            })
            .collect();

        self.history_time_bounds = [0.0, total_days];

        let values: Vec<f64> = self.history_data.iter().map(|(_, y)| *y).collect();
        let min_y = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_y = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let padding = ((max_y - min_y) * 0.15).max(2.0);
        self.history_y_bounds = [(min_y - padding).max(0.0), (max_y + padding).min(100.0)];

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

    fn sorted_system_indices(&self, faction: &FactionResponse) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..faction.presences.len()).collect();
        indices.sort_by(|&a, &b| {
            let pa = &faction.presences[a];
            let pb = &faction.presences[b];
            let ord = match self.system_sort {
                SystemSort::Name => pa.system_name.cmp(&pb.system_name),
                SystemSort::Influence => pa.influence
                    .partial_cmp(&pb.influence)
                    .unwrap_or(Ordering::Equal),
                SystemSort::ActiveState => pa.active_states.first().map(String::as_str).unwrap_or("")
                    .cmp(pb.active_states.first().map(String::as_str).unwrap_or("")),
                SystemSort::Pending => pa.pending_states.first().map(String::as_str).unwrap_or("")
                    .cmp(pb.pending_states.first().map(String::as_str).unwrap_or("")),
                SystemSort::Updated => pa.updated_at.cmp(&pb.updated_at),
            };
            if self.system_sort_dir == SortDir::Desc { ord.reverse() } else { ord }
        });
        indices
    }

    /// Keep detail_scroll so the selected system row stays visible.
    /// Body layout: line 0 = column header, line 1 = separator, line N+2 = presence[N].
    fn sync_system_scroll(&mut self) {
        self.detail_scroll = self.selected_system.saturating_sub(1);
    }

    pub fn prefill_search(&mut self, name: &str, api: &ApiClient) {
        self.search_query = name.to_string();
        self.search_state = SearchState::Idle;
        self.do_search(api);
    }

    pub fn handle_key(&mut self, key: &KeyEvent, api: &ApiClient) -> ViewEvent {
        if matches!(self.search_state, SearchState::Typing) {
            match key.code {
                KeyCode::Esc => {
                    self.search_state = SearchState::Idle;
                    self.status_msg = "Search cancelled".into();
                }
                KeyCode::Enter => {
                    self.search_state = SearchState::Idle;
                    self.do_search(api);
                }
                KeyCode::Backspace => { self.search_query.pop(); }
                KeyCode::Char(c)   => { self.search_query.push(c); }
                _ => {}
            }
            return ViewEvent::Consumed;
        }

        match key.code {
            KeyCode::Char('/') | KeyCode::Char('f') if self.embedded => return ViewEvent::FocusSearch,
            KeyCode::Char('/') | KeyCode::Char('f') => {
                self.search_query.clear();
                self.search_state = SearchState::Typing;
                self.status_msg = "Typing… (Enter to search, Esc to cancel)".into();
            }
            KeyCode::Char('p') => {
                if self.display_count() > 0 {
                    self.toggle_pin(api);
                }
                return ViewEvent::Consumed;
            }
            KeyCode::Char('w') | KeyCode::Up => match self.focus {
                FocusArea::List => {
                    if self.selected_idx > 0 {
                        self.selected_idx -= 1;
                        self.selected_system = 0;
                        self.detail_scroll = 0;
                        self.detail_tab = DetailTab::Info;
                    }
                }
                FocusArea::Detail if self.detail_tab == DetailTab::Systems => {
                    self.selected_system = self.selected_system.saturating_sub(1);
                    self.sync_system_scroll();
                }
                FocusArea::Detail => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                }
            },
            KeyCode::Char('s') | KeyCode::Down => match self.focus {
                FocusArea::List => {
                    if self.selected_idx + 1 < self.display_count() {
                        self.selected_idx += 1;
                        self.selected_system = 0;
                        self.detail_scroll = 0;
                        self.detail_tab = DetailTab::Info;
                    }
                }
                FocusArea::Detail if self.detail_tab == DetailTab::Systems => {
                    let max = self.selected_faction()
                        .map(|f| f.presences.len().saturating_sub(1))
                        .unwrap_or(0);
                    self.selected_system = (self.selected_system + 1).min(max);
                    self.sync_system_scroll();
                }
                FocusArea::Detail => {
                    self.detail_scroll += 1;
                }
            },
            KeyCode::PageUp => match self.focus {
                FocusArea::List => { self.selected_idx = self.selected_idx.saturating_sub(10); }
                FocusArea::Detail => { self.detail_scroll = self.detail_scroll.saturating_sub(10); }
            },
            KeyCode::PageDown => match self.focus {
                FocusArea::List => {
                    let max = self.display_count().saturating_sub(1);
                    self.selected_idx = (self.selected_idx + 10).min(max);
                }
                FocusArea::Detail => { self.detail_scroll += 10; }
            },
            KeyCode::Char('c') => {
                if self.focus == FocusArea::Detail && self.detail_tab == DetailTab::Systems {
                    // Resolve the highlighted row through the active sort, matching fetch_history.
                    let name = self.selected_faction().and_then(|faction| {
                        let indices = self.sorted_system_indices(faction);
                        let sys_i = indices.get(self.selected_system).copied()?;
                        faction.presences.get(sys_i).map(|p| p.system_name.trim().to_string())
                    });
                    if let Some(name) = name {
                        if let Some(cb) = self.clipboard.as_mut() {
                            let _ = cb.set_text(&name);
                        }
                    }
                    return ViewEvent::Consumed;
                }
            }
            KeyCode::Tab => {
                match self.focus {
                    FocusArea::List => {
                        if self.display_count() > 0 {
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
                        if next == DetailTab::History {
                            self.fetch_history(api);
                        }
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
            KeyCode::Char(c @ '1'..='5')
                if self.focus == FocusArea::Detail && self.detail_tab == DetailTab::Systems =>
            {
                let col = match c {
                    '1' => SystemSort::Name,
                    '2' => SystemSort::Influence,
                    '3' => SystemSort::ActiveState,
                    '4' => SystemSort::Pending,
                    '5' => SystemSort::Updated,
                    _ => unreachable!(),
                };
                if col == self.system_sort {
                    self.system_sort_dir = match self.system_sort_dir {
                        SortDir::Asc => SortDir::Desc,
                        SortDir::Desc => SortDir::Asc,
                    };
                } else {
                    self.system_sort = col;
                    self.system_sort_dir = col.default_dir();
                }
                self.selected_system = 0;
                self.detail_scroll = 0;
            }
            _ => {}
        }

        ViewEvent::None
    }
}

// ── Info tab ─────────────────────────────────────────────────────────────────

/// One law/security row of a government description: label, value and its colour.
type GovRow = (&'static str, &'static str, Color);

/// Description and law/security rows for a government type, or `None` when there is nothing
/// on file for it.
fn government_info(gov: &str) -> Option<(&'static [&'static str], &'static [GovRow])> {
    Some(match gov {
        "Anarchy" => (
            &[
                "No formal laws or enforcement. Individual strength determines survival.",
                "Security forces are absent or entirely corrupt.",
            ],
            &[
                ("Illegal:", "Nothing — no laws are enforced", Color::Green),
                ("Legal:", "Everything, including narcotics, weapons, slaves", Color::Cyan),
                ("Security:", "None — pirates thrive here", Color::Red),
                ("Trade risk:", "Very high — no protection for traders", Color::Red),
            ],
        ),
        "Communism" => (
            &[
                "State ownership of all production. Resources are allocated centrally",
                "for the common good. Equality is prioritised over individual freedom.",
            ],
            &[
                ("Illegal:", "Narcotics, combat stabilisers, civilian weapons", Color::Yellow),
                ("Legal:", "Most standard goods and commodities", Color::Cyan),
                ("Security:", "Moderate to high", Color::Green),
                ("Trade:", "State-controlled; black markets common", Color::DarkGray),
            ],
        ),
        "Confederacy" => (
            &[
                "A loose alliance of autonomous member states sharing common defence",
                "and trade agreements. Local authorities retain significant autonomy.",
            ],
            &[
                ("Illegal:", "Narcotics, some weapons (varies by station)", Color::Yellow),
                ("Legal:", "Most goods; weapon laws vary per port", Color::Cyan),
                ("Security:", "Moderate", Color::Green),
            ],
        ),
        "Corporate" => (
            &[
                "A megacorporation exercises governmental authority. Every policy",
                "decision is driven by profit. Workers are assets; shareholders rule.",
            ],
            &[
                ("Illegal:", "Narcotics (reduce productivity), unsanctioned weapons", Color::Yellow),
                ("Legal:", "Most goods if profitable; grey-market tolerant", Color::Cyan),
                ("Security:", "Moderate — corporate security forces", Color::Green),
                ("Trade:", "Strong markets; corporations control supply", Color::Green),
            ],
        ),
        "Cooperative" => (
            &[
                "Collectively owned and democratically managed. Members share profits",
                "and governance equally. Community wellbeing is the primary goal.",
            ],
            &[
                ("Illegal:", "Narcotics, unregulated weapons", Color::Yellow),
                ("Legal:", "Most standard goods and services", Color::Cyan),
                ("Security:", "Moderate — community watch", Color::Green),
            ],
        ),
        "Democracy" => (
            &[
                "Elected representatives govern in the name of the people. Laws",
                "balance individual freedoms with collective security.",
            ],
            &[
                ("Illegal:", "Narcotics, unregulated weapons, combat stabilisers", Color::Yellow),
                ("Legal:", "Most commercial goods; personal sidearms (licensed)", Color::Cyan),
                ("Security:", "Moderate to high", Color::Green),
            ],
        ),
        "Dictatorship" => (
            &[
                "A single autocrat holds absolute power. Order is maintained through",
                "strict law enforcement and loyal security forces. Dissent is crushed.",
            ],
            &[
                ("Illegal:", "Narcotics, weapons, opposition materials, many goods", Color::Red),
                ("Legal:", "State-approved goods only", Color::Cyan),
                ("Security:", "Very high — harsh penalties", Color::Red),
                ("Trade:", "Restricted; state has monopoly on key goods", Color::Yellow),
            ],
        ),
        "Feudal" => (
            &[
                "Noble hierarchy governs by hereditary right. Vassals owe loyalty and",
                "service to their liege lords. Might and lineage determine status.",
            ],
            &[
                ("Illegal:", "Weapons for commoners, narcotics", Color::Yellow),
                ("Legal:", "Slaves (in many feudal systems), agricultural goods", Color::Cyan),
                ("Security:", "High for nobility; low for common areas", Color::Yellow),
            ],
        ),
        "Imperial" => (
            &[
                "Imperial hierarchy based on rank, honour and duty. Service to the",
                "Empire is paramount. Social mobility exists through demonstrated loyalty.",
            ],
            &[
                ("Illegal:", "Chattel slaves, narcotics, advanced personal weapons", Color::Red),
                ("Legal:", "Imperial Slaves (bond-servants with rights and a contract)", Color::Cyan),
                ("Security:", "High — Imperial Navy patrols", Color::Green),
                ("Note:", "Imperial Slaves ≠ chattel slavery; bond ends after term", Color::DarkGray),
            ],
        ),
        "Patronage" => (
            &[
                "Powerful patrons extend protection in exchange for loyalty and service.",
                "Political power flows through personal relationships, not institutions.",
            ],
            &[
                ("Illegal:", "Narcotics, weapons (varies by patron)", Color::Yellow),
                ("Legal:", "Goods approved by the ruling patron", Color::Cyan),
                ("Security:", "Moderate — patron's private forces", Color::Yellow),
            ],
        ),
        "Prison Colony" => (
            &[
                "A penal authority administers this system. Security forces are wardens.",
                "Residents are either prisoners, guards, or support staff.",
            ],
            &[
                ("Illegal:", "Almost everything beyond basic necessities", Color::Red),
                ("Legal:", "Heavily regulated basic goods only", Color::Cyan),
                ("Security:", "Maximum — entire system is a prison", Color::Red),
                ("Trade:", "Extremely restricted; smuggling very risky", Color::Red),
            ],
        ),
        "Theocracy" => (
            &[
                "Sacred law governs all aspects of life. Religious authorities hold",
                "ultimate temporal power. Deviation from doctrine is a criminal offence.",
            ],
            &[
                ("Illegal:", "Narcotics, personal weapons, goods deemed heretical", Color::Red),
                ("Legal:", "Goods and services aligned with religious doctrine", Color::Cyan),
                ("Security:", "Very high — zealous enforcement", Color::Red),
                ("Trade:", "Restricted to approved goods", Color::Yellow),
            ],
        ),
        "None" => (
            &[
                "No established government. Laws and security vary unpredictably.",
            ],
            &[],
        ),
        _ => return None,
    })
}

/// Description and notable facts for an allegiance, or `None` when there is nothing on file.
fn allegiance_info(alleg: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    Some(match alleg {
        "Federation" => (
            &[
                "Member of the Galactic Federation. Values democracy, individual rights",
                "and rule of law. Bureaucratic governance; strong Federal Navy.",
            ],
            &[
                "Federal credits and rank can be earned through missions",
                "Anti-slavery: chattel and Imperial slaves are illegal in Fed space",
                "Strong presence in the Sol neighbourhood and core worlds",
            ],
        ),
        "Empire" => (
            &[
                "Subject of the Galactic Empire. Values honour, rank, duty and tradition.",
                "Social hierarchy is rigid but meritocratic through demonstrated service.",
            ],
            &[
                "Imperial rank earned through trade, combat and patron missions",
                "Imperial Slaves are legal — a regulated bond-servant system",
                "Narcotics and chattel slaves are illegal in Imperial space",
            ],
        ),
        "Alliance" => (
            &[
                "Member of the Alliance of Independent Systems. Values sovereignty,",
                "cooperation and mutual defence without imposing cultural uniformity.",
            ],
            &[
                "Loosely governed — member systems retain local laws",
                "Focused on exploration, trade and defence",
                "No unified stance on slaves or narcotics; local laws apply",
            ],
        ),
        "Independent" => (
            &[
                "No allegiance to a major power. Laws and policies are set entirely",
                "by the local governing faction. Expect wide variation between systems.",
            ],
            &[
                "Check local station laws before trading sensitive goods",
                "Often targets of powerplay expansion from the major factions",
            ],
        ),
        "Pilots Federation" | "PilotsFederation" => (
            &[
                "Governed by the Pilots Federation — the guild of licensed commanders.",
                "Mostly neutral; these systems serve as training grounds and safe zones.",
            ],
            &[
                "Rebuy penalties do not apply in Pilots Federation space",
                "Combat is illegal — these are protected starter systems",
            ],
        ),
        _ => return None,
    })
}
