mod gui;
mod influence_chart;

pub(crate) use gui::pretty;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use edcas_common::exobiology::BodyFacts;
use edcas_common::api::{FactionInfluencePoint, SystemTickPrediction};

use crate::event_shim::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style};
use tracing::debug;

use crate::journal_reader::{
    BodyRing, BodyScan, BodySignal, DiscoveredSignal,
    JournalData, SaaBodyData, StationData, SystemData, SystemScans, TreeNode, build_body_tree,
    load_system_scans,
};
use crate::api_client::ApiClient;
use crate::body_finds::BodyFinds;
use crate::cartography::UnsoldData;
use crate::settings::Settings;
use crate::views::ViewEvent;

/// Days of faction influence the system panel charts.
const INFLUENCE_DAYS: u32 = 30;
/// How long a system's server history is shown before it is fetched again (the tick
/// prediction moves on once a tick has passed).
const HISTORY_REFRESH: Duration = Duration::from_secs(15 * 60);

/// What the server knows of a system's past for the system panel: faction influence over
/// time and its BGS ticks.
#[derive(Default)]
pub(crate) struct SystemHistory {
    /// Every faction's influence readings, ordered by faction then time.
    influence: Vec<FactionInfluencePoint>,
    /// The influence request failed (as opposed to the server having no readings).
    influence_failed: bool,
    next_tick: Option<SystemTickPrediction>,
    last_tick: Option<DateTime<Utc>>,
}

/// One system's history request: `data` is `None` until the first answer arrives and keeps
/// the previous answer while a refresh is under way.
struct HistoryEntry {
    requested: Instant,
    data: Option<SystemHistory>,
}

#[derive(PartialEq)]
enum ExplorerFocus {
    SystemInfo,
    Tree,
    Detail,
}

pub struct ExplorerView {
    // Body tree state
    tree: Vec<TreeNode>,
    selected_idx: usize,
    flat_nodes: Vec<FlatNode>,
    detail_scroll: usize,
    system_name: String,
    fss_signals: HashMap<i32, Vec<BodySignal>>,
    saa_data: HashMap<i32, SaaBodyData>,
    stations: Vec<StationData>,
    discovered_signals: Vec<DiscoveredSignal>,
    fss_body_count: Option<u32>,
    fss_non_body_count: Option<u32>,
    fss_all_bodies_found: bool,
    nav_beacon_bodies: Option<u32>,
    // System history / info state
    visited_systems: Vec<SystemData>,
    selected_system_idx: usize,
    selected_faction: usize,
    focus: ExplorerFocus,
    // Bodies cached per system address for history navigation
    scans_cache: HashMap<i64, SystemScans>,
    /// Past systems whose scans are being read back from the journals, and the reads that
    /// came back (see [`ExplorerView::poll_scans`]).
    scans_loading: HashSet<i64>,
    scans_rx: Arc<Mutex<Vec<(i64, SystemScans)>>>,
    /// Stations the server knows per system address (see [`ExplorerView::set_server_stations`]).
    server_stations: HashMap<i64, Vec<StationData>>,
    /// Influence and tick history per system address (see [`ExplorerView::poll_history`]).
    history: HashMap<i64, HistoryEntry>,
    history_rx: Arc<Mutex<Vec<(i64, SystemHistory)>>>,
    /// Bodies whose subtree is folded away in the tree (window only; cleared on system change).
    collapsed: HashSet<i32>,
    /// Showing a system from the server rather than the journal (the galaxy map's pick, see
    /// [`ExplorerView::show_remote_system`]): only the tree and details exist then.
    remote: bool,
    /// The count of the newest body focus the selection followed
    /// ([`JournalData::body_in_focus`]).
    followed_focus: u64,
    /// Exploration data not yet sold (see [`crate::cartography`]), to tell sold firsts from
    /// unsold ones.
    pub(crate) unsold: Arc<UnsoldData>,
    /// Organic and geological finds per body, from every journal.
    pub(crate) finds: Arc<BodyFinds>,
    gui: gui::GuiState,
}

enum NodeType {
    Body,
    SectionHeader,
    Station(StationData),
    Signal(DiscoveredSignal),
    /// A bio/geo signal attached to a body; FlatNode.body_id is the parent body.
    BodySignal(BodySignal),
    /// One of a body's rings or belts; FlatNode.body_id is the parent body.
    Ring(BodyRing),
}

struct FlatNode {
    tree_prefix: String,
    short_name: String,
    body_id: i32,
    distance_ls: f32,
    has_rings: bool,
    landable: bool,
    planet_class: String,
    star_type: String,
    is_barycentre: bool,
    bio_signal_count: i32,
    geo_signal_count: i32,
    node_type: NodeType,
    radius: f32,    // meters; 0 if body not yet scanned
    /// Rows folded away below this one because its body is collapsed.
    hidden: usize,
}

impl ExplorerView {
    pub fn new() -> Self {
        Self {
            tree: Vec::new(),
            selected_idx: 0,
            flat_nodes: Vec::new(),
            detail_scroll: 0,
            system_name: String::new(),
            fss_signals: HashMap::new(),
            saa_data: HashMap::new(),
            stations: Vec::new(),
            discovered_signals: Vec::new(),
            fss_body_count: None,
            fss_non_body_count: None,
            fss_all_bodies_found: false,
            nav_beacon_bodies: None,
            visited_systems: Vec::new(),
            selected_system_idx: 0,
            selected_faction: 0,
            focus: ExplorerFocus::Tree,
            scans_cache: HashMap::new(),
            scans_loading: HashSet::new(),
            scans_rx: Arc::default(),
            server_stations: HashMap::new(),
            history: HashMap::new(),
            history_rx: Arc::default(),
            collapsed: HashSet::new(),
            remote: false,
            followed_focus: 0,
            unsold: Arc::default(),
            finds: Arc::default(),
            gui: gui::GuiState::default(),
        }
    }

    /// Shows a system as the server knows it — bodies and stations, no signals or FSS
    /// progress — for the galaxy map. Only the tree and detail panels are used then
    /// ([`ExplorerView::system_map_ui`]).
    pub(crate) fn show_remote_system(&mut self, name: &str, bodies: &[BodyScan], stations: Vec<StationData>) {
        self.remote = true;
        self.system_name = name.to_string();
        self.tree = build_body_tree(bodies);
        self.stations = stations;
        self.fss_signals.clear();
        self.saa_data.clear();
        self.discovered_signals.clear();
        self.fss_body_count = None;
        self.fss_non_body_count = None;
        self.fss_all_bodies_found = false;
        self.nav_beacon_bodies = None;
        self.collapsed.clear();
        self.focus = ExplorerFocus::Tree;
        self.rebuild_flat_nodes();
        self.selected_idx = self.nearest_selectable(0);
        self.detail_scroll = 0;
    }

    /// Stations the server knows for a system. Every commander's docks report how far out
    /// each one is, so the tree can hang them (and their FSS signals) on their bodies after
    /// a jump back, not only once this commander docks there again.
    pub fn set_server_stations(&mut self, system_address: i64, stations: &[StationSearchResult]) {
        self.server_stations.insert(system_address, stations.iter().map(station_from_api).collect());
    }

    /// Picks up finished history requests and asks the server for the shown system's
    /// influence and tick history when it has none or it is older than [`HISTORY_REFRESH`].
    pub fn poll_history(&mut self, api: &ApiClient) {
        for (addr, data) in self.history_rx.lock().unwrap().drain(..) {
            if let Some(entry) = self.history.get_mut(&addr) {
                entry.data = Some(data);
            }
        }
        let Some(addr) = self.visited_systems.get(self.selected_system_idx).map(|s| s.system_address) else { return };
        let due = self.history.get(&addr).is_none_or(|e| e.data.is_some() && e.requested.elapsed() > HISTORY_REFRESH);
        if addr == 0 || !due {
            return;
        }
        let entry = self.history.entry(addr).or_insert(HistoryEntry { requested: Instant::now(), data: None });
        entry.requested = Instant::now();
        let slot = Arc::clone(&self.history_rx);
        let api_owned = api.clone();
        api.spawn(async move {
            let (influence, prediction, ticks) = tokio::join!(
                api_owned.get_system_influence_history(addr, INFLUENCE_DAYS),
                api_owned.get_tick_prediction(addr),
                api_owned.get_system_ticks(addr),
            );
            let next_tick = prediction.ok().flatten();
            let last_tick = next_tick.as_ref().and_then(|p| p.last_tick)
                .or_else(|| ticks.ok().and_then(|t| t.first().map(|t| t.tick_upper)));
            let data = SystemHistory {
                influence_failed: influence.is_err(),
                influence: influence.unwrap_or_default(),
                next_tick,
                last_tick,
            };
            slot.lock().unwrap().push((addr, data));
        });
    }

    /// The server history of a system, once it has arrived.
    fn history_of(&self, system_address: i64) -> Option<&SystemHistory> {
        self.history.get(&system_address).and_then(|e| e.data.as_ref())
    }

    pub fn update(&mut self, journal: &JournalData) {
        // Keep the current system's scans so the jump history can show them later.
        let current_addr = journal.current_system.as_ref().map(|s| s.system_address);
        if let Some(addr) = current_addr.filter(|_| !journal.bodies.is_empty()) {
            self.scans_cache.insert(addr, SystemScans::of(journal));
        }

        // Update visited systems; reset to current when the active system changes.
        let prev_addr = self.visited_systems.first().map(|s| s.system_address);
        self.visited_systems = journal.visited_systems.clone();
        let new_addr = self.visited_systems.first().map(|s| s.system_address);
        if prev_addr != new_addr {
            self.selected_system_idx = 0;
            self.selected_faction = 0;
            self.collapsed.clear();
        }
        if self.selected_system_idx >= self.visited_systems.len() {
            self.selected_system_idx = 0;
        }

        // The current system as the journal has it (with the bodies of an earlier visit
        // until it scans them again), a past one as cached or read back from the journals.
        let current = self.selected_system_idx == 0;
        let shown_addr = if current { current_addr } else { self.visited_systems.get(self.selected_system_idx).map(|s| s.system_address) };
        let scans = if current {
            let mut live = SystemScans::of(journal);
            if live.bodies.is_empty() {
                if let Some(cached) = current_addr.and_then(|a| self.scans_cache.get(&a)) {
                    live.bodies = cached.bodies.clone();
                }
            }
            live
        } else {
            shown_addr.and_then(|a| self.scans_cache.get(&a)).cloned().unwrap_or_default()
        };
        self.system_name = if current {
            journal.current_system.as_ref().map(|s| s.name.clone()).unwrap_or_default()
        } else {
            self.visited_systems.get(self.selected_system_idx).map(|s| s.name.clone()).unwrap_or_default()
        };
        self.show_scans(&scans);
        debug!("Explorer updated with {} bodies, {} tree nodes", scans.bodies.len(), self.tree.len());

        // This session's docks for the current system, then what the server knows of the
        // shown system. A carrier from the server counts only while the FSS still sees it.
        let mut stations = if current { journal.stations.clone() } else { Vec::new() };
        for s in shown_addr.and_then(|a| self.server_stations.get(&a)).into_iter().flatten() {
            let present = s.station_type != "FleetCarrier"
                || self.discovered_signals.iter().any(|sig| signal_names_station(sig, s));
            if present && !stations.iter().any(|j| j.market_id == s.market_id) {
                stations.push(s.clone());
            }
        }
        self.stations = stations;
        // A newly scanned body moves the rows below it: stay on the selected body.
        let kept = self.flat_nodes.get(self.selected_idx).filter(|n| matches!(n.node_type, NodeType::Body)).map(|n| n.body_id);
        self.rebuild_flat_nodes();
        if let Some(i) = kept.and_then(|id| self.body_row(id)) {
            self.selected_idx = i;
        }
        if self.selected_idx >= self.flat_nodes.len() {
            self.selected_idx = self.flat_nodes.len().saturating_sub(1);
        }

        // Follow the body the commander turned to in the game (targeted, approached, dropped
        // out at, or scanned in the FSS or by surface map) in the current system.
        if let Some((n, body_id)) = journal.body_in_focus.filter(|&(n, _)| n != self.followed_focus) {
            if current && !self.remote {
                self.followed_focus = n;
                self.select_body(body_id);
            }
        }
    }

    /// The row of a body in the tree, if shown.
    fn body_row(&self, body_id: i32) -> Option<usize> {
        self.flat_nodes.iter().position(|n| matches!(n.node_type, NodeType::Body) && n.body_id == body_id)
    }

    /// Selects a body's row, unfolding the bodies it is folded under.
    fn select_body(&mut self, body_id: i32) {
        let mut path = Vec::new();
        if body_path(&self.tree, body_id, &mut path) {
            let mut unfolded = false;
            for id in &path {
                unfolded |= self.collapsed.remove(id);
            }
            if unfolded {
                self.rebuild_flat_nodes();
            }
        }
        if let Some(i) = self.body_row(body_id).filter(|&i| i != self.selected_idx) {
            self.selected_idx = i;
            self.detail_scroll = 0;
        }
    }

    fn rebuild_flat_nodes(&mut self) {
        self.flat_nodes.clear();
        let (stations, signals) = place_in_tree(&self.tree, &self.stations, &self.discovered_signals);
        let mut placed_stations: HashSet<i64> = HashSet::new();
        let n = self.tree.len();
        for (i, node) in self.tree.iter().enumerate() {
            flatten_node(
                node,
                "",
                i == n - 1,
                &mut self.flat_nodes,
                &self.system_name,
                &self.fss_signals,
                &self.saa_data,
                &signals,
                &stations,
                &mut placed_stations,
            );
        }
        if !self.collapsed.is_empty() {
            self.flat_nodes = fold_collapsed(std::mem::take(&mut self.flat_nodes), &self.collapsed);
        }
        let unplaced: Vec<&StationData> = stations.iter()
            .filter(|s| !placed_stations.contains(&s.market_id))
            .collect();
        if !unplaced.is_empty() {
            self.flat_nodes.push(FlatNode {
                tree_prefix: String::new(),
                short_name: "Stations".into(),
                body_id: -1,
                distance_ls: 0.0,
                has_rings: false,
                landable: false,
                planet_class: String::new(),
                star_type: String::new(),
                is_barycentre: false,
                bio_signal_count: 0,
                geo_signal_count: 0,
                node_type: NodeType::SectionHeader,
                radius: 0.0,
                hidden: 0,
            });
            for station in unplaced {
                self.flat_nodes.push(FlatNode {
                    tree_prefix: String::new(),
                    short_name: carrier_display_name(station, &signals),
                    body_id: -1,
                    distance_ls: station.dist_from_star_ls,
                    has_rings: false,
                    landable: false,
                    planet_class: String::new(),
                    star_type: String::new(),
                    is_barycentre: false,
                    bio_signal_count: 0,
                    geo_signal_count: 0,
                    node_type: NodeType::Station(station.clone()),
                    radius: 0.0,
                    hidden: 0,
                });
            }
        }
        // Only show signals that have no body association in the flat section.
        // Signals with a body_id are already rendered under their body in the tree.
        // Also hide fleet carrier signals for carriers already shown as docked stations.
        let unassociated: Vec<&DiscoveredSignal> = signals.iter()
            .filter(|s| s.body_id.is_none())
            .filter(|s| !is_known_station_signal(s, &stations))
            .collect();
        if !unassociated.is_empty() {
            self.flat_nodes.push(FlatNode {
                tree_prefix: String::new(),
                short_name: format!("System Signals ({})", unassociated.len()),
                body_id: -1,
                distance_ls: 0.0,
                has_rings: false,
                landable: false,
                planet_class: String::new(),
                star_type: String::new(),
                is_barycentre: false,
                bio_signal_count: 0,
                geo_signal_count: 0,
                node_type: NodeType::SectionHeader,
                radius: 0.0,
                hidden: 0,
            });
            for sig in unassociated {
                self.flat_nodes.push(FlatNode {
                    tree_prefix: String::new(),
                    short_name: sig.display_name.clone(),
                    body_id: -1,
                    distance_ls: 0.0,
                    has_rings: false,
                    landable: false,
                    planet_class: String::new(),
                    star_type: String::new(),
                    is_barycentre: false,
                    bio_signal_count: 0,
                    geo_signal_count: 0,
                    node_type: NodeType::Signal(sig.clone()),
                    radius: 0.0,
                    hidden: 0,
                });
            }
        }
    }

    /// Shows the system at `idx` in the history (0 = current), as `a`/`d` step there.
    fn select_system(&mut self, idx: usize) {
        if idx >= self.visited_systems.len() || idx == self.selected_system_idx {
            return;
        }
        self.selected_system_idx = idx;
        self.selected_faction = 0;
        self.collapsed.clear();
        self.rebuild_tree_for_selected_system();
    }

    /// Folds or unfolds the subtree under the body row at `idx`, keeping the selection on the
    /// same row (or on `idx` when the selected row is folded away).
    fn toggle_collapsed(&mut self, idx: usize) {
        let Some(node) = self.flat_nodes.get(idx) else { return };
        if !matches!(node.node_type, NodeType::Body) {
            return;
        }
        let body_id = node.body_id;
        if !self.collapsed.remove(&body_id) {
            self.collapsed.insert(body_id);
        }
        let before = self.flat_nodes.len() as isize;
        self.rebuild_flat_nodes();
        let delta = self.flat_nodes.len() as isize - before;
        if self.selected_idx > idx {
            if delta < 0 && self.selected_idx <= idx + (-delta) as usize {
                self.selected_idx = idx;
                self.detail_scroll = 0;
            } else {
                self.selected_idx = (self.selected_idx as isize + delta).max(0) as usize;
            }
        }
        self.selected_idx = self.selected_idx.min(self.flat_nodes.len().saturating_sub(1));
    }

    /// Rebuilds the body tree from cached bodies for `selected_system_idx`.
    /// Called after history navigation (a/d) to update the tree without a full journal update.
    fn rebuild_tree_for_selected_system(&mut self) {
        let addr = self.visited_systems
            .get(self.selected_system_idx)
            .map(|s| s.system_address);
        if let Some(addr) = addr {
            self.system_name = self.visited_systems
                .get(self.selected_system_idx)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            let scans = self.scans_cache.get(&addr).cloned().unwrap_or_default();
            self.show_scans(&scans);
            self.selected_idx = 0;
            self.rebuild_flat_nodes();
        }
    }

    /// Shows one system's scans: the body tree, its signals and the FSS progress.
    fn show_scans(&mut self, scans: &SystemScans) {
        self.tree = build_body_tree(&scans.bodies);
        self.fss_signals = scans.fss_signals.clone();
        self.saa_data = scans.saa_data.clone();
        self.fss_body_count = scans.fss_body_count;
        self.fss_non_body_count = scans.fss_non_body_count;
        self.fss_all_bodies_found = scans.fss_all_bodies_found;
        self.nav_beacon_bodies = scans.nav_beacon_bodies;
        self.discovered_signals = scans.discovered_signals.clone();
    }

    /// Reads a past system's scans back from the journals when the jump history shows one
    /// the client wasn't running for, in the background; shows them when they arrive.
    pub fn poll_scans(&mut self, dir: &str) {
        let arrived: Vec<(i64, SystemScans)> = self.scans_rx.lock().unwrap().drain(..).collect();
        for (addr, scans) in arrived {
            self.scans_loading.remove(&addr);
            self.scans_cache.entry(addr).or_insert(scans);
            if self.selected_system_idx > 0 && self.visited_systems.get(self.selected_system_idx).map(|s| s.system_address) == Some(addr) {
                self.rebuild_tree_for_selected_system();
            }
        }
        let dir = dir.trim();
        if self.remote || self.selected_system_idx == 0 || dir.is_empty() {
            return;
        }
        let Some(addr) = self.visited_systems.get(self.selected_system_idx).map(|s| s.system_address) else { return };
        if self.scans_cache.contains_key(&addr) || !self.scans_loading.insert(addr) {
            return;
        }
        let (rx, dir) = (Arc::clone(&self.scans_rx), std::path::PathBuf::from(dir));
        std::thread::spawn(move || {
            let scans = load_system_scans(&dir, addr);
            rx.lock().unwrap().push((addr, scans));
        });
    }

    /// Whether the shown past system's scans are still being read from the journals.
    pub(crate) fn loading_scans(&self) -> bool {
        self.selected_system_idx > 0
            && self.visited_systems.get(self.selected_system_idx).is_some_and(|s| self.scans_loading.contains(&s.system_address))
    }

    fn get_selected_body(&self) -> Option<&BodyScan> {
        let node = self.flat_nodes.get(self.selected_idx)?;
        // Body, BodySignal and Ring nodes store a valid body_id
        if !matches!(node.node_type, NodeType::Body | NodeType::BodySignal(_) | NodeType::Ring(_)) {
            return None;
        }
        find_body_in_tree(&self.tree, node.body_id)
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> ViewEvent {
        let event = self.handle_focus_key(key);
        // A system from the galaxy map has no system panel to move to.
        if self.remote && self.focus == ExplorerFocus::SystemInfo {
            self.focus = ExplorerFocus::Tree;
        }
        event
    }

    fn handle_focus_key(&mut self, key: &KeyEvent) -> ViewEvent {
        match self.focus {
            ExplorerFocus::SystemInfo => match key.code {
                KeyCode::Char('w') | KeyCode::Up => {
                    // Navigate factions upward
                    self.selected_faction = self.selected_faction.saturating_sub(1);
                    ViewEvent::Consumed
                }
                KeyCode::Char('s') | KeyCode::Down => {
                    // Navigate factions downward
                    let faction_count = self.visited_systems
                        .get(self.selected_system_idx)
                        .map(|s| s.factions.len())
                        .unwrap_or(0);
                    if faction_count > 0 {
                        self.selected_faction = (self.selected_faction + 1).min(faction_count - 1);
                    }
                    ViewEvent::Consumed
                }
                KeyCode::Char('a') | KeyCode::Left => {
                    // Browse to older system (higher index = further back in history)
                    self.select_system(self.selected_system_idx + 1);
                    ViewEvent::Consumed
                }
                KeyCode::Char('d') | KeyCode::Right => {
                    // Browse to newer system (lower index = more recent)
                    if self.selected_system_idx > 0 {
                        self.select_system(self.selected_system_idx - 1);
                    }
                    ViewEvent::Consumed
                }
                KeyCode::Tab => {
                    self.focus = ExplorerFocus::Tree;
                    ViewEvent::Consumed
                }
                KeyCode::Char(' ') => {
                    if let Some(system) = self.visited_systems.get(self.selected_system_idx) {
                        if !system.factions.is_empty() {
                            let mut sorted = system.factions.clone();
                            sorted.sort_by(|a, b| {
                                b.influence.partial_cmp(&a.influence).unwrap_or(Ordering::Equal)
                            });
                            let idx = self.selected_faction.min(sorted.len() - 1);
                            return ViewEvent::OpenFactions(sorted[idx].name.clone());
                        }
                    }
                    ViewEvent::None
                }
                _ => ViewEvent::None,
            },
            ExplorerFocus::Tree => {
                let n = self.flat_nodes.len();
                match key.code {
                    KeyCode::Char('w') | KeyCode::Up => {
                        self.selected_idx = self.prev_selectable(self.selected_idx);
                        self.detail_scroll = 0;
                        ViewEvent::Consumed
                    }
                    KeyCode::Char('s') | KeyCode::Down => {
                        self.selected_idx = self.next_selectable(self.selected_idx);
                        self.detail_scroll = 0;
                        ViewEvent::Consumed
                    }
                    KeyCode::PageUp => {
                        let target = self.selected_idx.saturating_sub(10);
                        self.selected_idx = self.nearest_selectable(target);
                        self.detail_scroll = 0;
                        ViewEvent::Consumed
                    }
                    KeyCode::PageDown => {
                        let target = (self.selected_idx + 10).min(n.saturating_sub(1));
                        self.selected_idx = self.nearest_selectable(target);
                        self.detail_scroll = 0;
                        ViewEvent::Consumed
                    }
                    KeyCode::Home => {
                        self.selected_idx = self.nearest_selectable(0);
                        self.detail_scroll = 0;
                        ViewEvent::Consumed
                    }
                    KeyCode::End => {
                        self.selected_idx = self.nearest_selectable(n.saturating_sub(1));
                        self.detail_scroll = 0;
                        ViewEvent::Consumed
                    }
                    KeyCode::Tab | KeyCode::Char('d') | KeyCode::Right => {
                        self.focus = ExplorerFocus::Detail;
                        ViewEvent::Consumed
                    }
                    KeyCode::Char('a') | KeyCode::Left => {
                        self.focus = ExplorerFocus::SystemInfo;
                        ViewEvent::Consumed
                    }
                    _ => ViewEvent::None,
                }
            }
            ExplorerFocus::Detail => match key.code {
                KeyCode::Char('w') | KeyCode::Up => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(1);
                    ViewEvent::Consumed
                }
                KeyCode::Char('s') | KeyCode::Down => {
                    self.detail_scroll = self.detail_scroll.saturating_add(1);
                    ViewEvent::Consumed
                }
                KeyCode::PageUp => {
                    self.detail_scroll = self.detail_scroll.saturating_sub(10);
                    ViewEvent::Consumed
                }
                KeyCode::PageDown => {
                    self.detail_scroll = self.detail_scroll.saturating_add(10);
                    ViewEvent::Consumed
                }
                KeyCode::Home => {
                    self.detail_scroll = 0;
                    ViewEvent::Consumed
                }
                KeyCode::End => {
                    self.detail_scroll = usize::MAX;
                    ViewEvent::Consumed
                }
                KeyCode::Tab => {
                    self.focus = ExplorerFocus::SystemInfo;
                    ViewEvent::Consumed
                }
                KeyCode::Char('a') | KeyCode::Left | KeyCode::Esc => {
                    self.focus = ExplorerFocus::Tree;
                    ViewEvent::Consumed
                }
                _ => ViewEvent::None,
            },
        }
    }

    // Navigate to the previous non-header node; stay if none exists.
    fn prev_selectable(&self, from: usize) -> usize {
        let mut i = from;
        while i > 0 {
            i -= 1;
            if !matches!(self.flat_nodes[i].node_type, NodeType::SectionHeader) {
                return i;
            }
        }
        from
    }

    // Navigate to the next non-header node; stay if none exists.
    fn next_selectable(&self, from: usize) -> usize {
        let mut i = from + 1;
        while i < self.flat_nodes.len() {
            if !matches!(self.flat_nodes[i].node_type, NodeType::SectionHeader) {
                return i;
            }
            i += 1;
        }
        from
    }

    // Nearest non-header node to `target`: try forward then backward.
    fn nearest_selectable(&self, target: usize) -> usize {
        if self.flat_nodes.is_empty() {
            return 0;
        }
        let target = target.min(self.flat_nodes.len() - 1);
        if !matches!(self.flat_nodes[target].node_type, NodeType::SectionHeader) {
            return target;
        }
        // Forward
        let mut i = target + 1;
        while i < self.flat_nodes.len() {
            if !matches!(self.flat_nodes[i].node_type, NodeType::SectionHeader) {
                return i;
            }
            i += 1;
        }
        // Backward
        let mut i = target;
        while i > 0 {
            i -= 1;
            if !matches!(self.flat_nodes[i].node_type, NodeType::SectionHeader) {
                return i;
            }
        }
        self.selected_idx
    }
}

// ── Tree flattening ──────────────────────────────────────────────────────────

/// A planetary mining site: the `$PlanetaryMiningLocation_Name;` signal.
fn is_mining_site(s: &BodySignal) -> bool {
    s.signal_type.contains("PlanetaryMiningLocation")
}

fn flatten_node(
    node: &TreeNode,
    parent_prefix: &str,
    is_last: bool,
    result: &mut Vec<FlatNode>,
    system_name: &str,
    fss_signals: &HashMap<i32, Vec<BodySignal>>,
    saa_data: &HashMap<i32, SaaBodyData>,
    discovered_signals: &[DiscoveredSignal],
    stations: &[StationData],
    placed_stations: &mut HashSet<i64>,
) {
    let connector = if is_last { "└─ " } else { "├─ " };
    let continuation = if is_last { "   " } else { "│  " };
    let tree_prefix = format!("{}{}", parent_prefix, connector);
    let child_prefix = format!("{}{}", parent_prefix, continuation);

    let body = node.data.as_ref();
    let is_barycentre = node.name.to_ascii_lowercase().contains("barycentre");
    let is_star = body.map(|b| !b.star_type.is_empty()).unwrap_or(false);
    let short_name = strip_system_prefix(&node.name, system_name);
    let body_dist = body.map(|b| b.distance_from_arrival_ls).unwrap_or(0.0);

    let (bio_signal_count, geo_signal_count) = if let Some(saa) = saa_data.get(&node.body_id) {
        let bio = saa.signals.iter().filter(|s| s.is_biological()).map(|s| s.count).sum();
        let geo = saa.signals.iter().filter(|s| s.is_geological()).map(|s| s.count).sum();
        (bio, geo)
    } else if let Some(fss) = fss_signals.get(&node.body_id) {
        let bio = fss.iter().filter(|s| s.is_biological()).map(|s| s.count).sum();
        let geo = fss.iter().filter(|s| s.is_geological()).map(|s| s.count).sum();
        (bio, geo)
    } else {
        (0, 0)
    };

    result.push(FlatNode {
        tree_prefix,
        short_name,
        body_id: node.body_id,
        distance_ls: body_dist,
        has_rings: body.map(|b| !b.rings.is_empty()).unwrap_or(false),
        landable: body.map(|b| b.landable).unwrap_or(false),
        planet_class: body.map(|b| b.planet_class.clone()).unwrap_or_default(),
        star_type: body.map(|b| b.star_type.clone()).unwrap_or_default(),
        is_barycentre,
        bio_signal_count,
        geo_signal_count,
        node_type: NodeType::Body,
        radius: body.map(|b| b.radius).unwrap_or(0.0),
        hidden: 0,
    });

    // Stations orbiting this body, as `place_in_tree` resolved them.
    let orbiting: Vec<&StationData> = if !is_star && !is_barycentre {
        stations.iter()
            .filter(|s| !placed_stations.contains(&s.market_id) && s.host_body_id == Some(node.body_id))
            .collect()
    } else {
        Vec::new()
    };
    // Mark placed BEFORE recursing so children don't pick up these stations.
    for s in &orbiting {
        placed_stations.insert(s.market_id);
    }
    let n_orbiting = orbiting.len();

    // Collect signals for this body (prefer SAA over FSS)
    let body_signals: Vec<BodySignal> = if let Some(saa) = saa_data.get(&node.body_id) {
        saa.signals.clone()
    } else if let Some(sigs) = fss_signals.get(&node.body_id) {
        sigs.clone()
    } else {
        Vec::new()
    };
    // Mining sites crowd the tree (17 on one body at Hegoea EN-F b39-0); the body's
    // details still list them.
    let body_signals: Vec<BodySignal> = body_signals.into_iter().filter(|s| !is_mining_site(s)).collect();

    let n_sigs = body_signals.len();
    let disc_sigs: Vec<&DiscoveredSignal> = discovered_signals
        .iter()
        .filter(|s| s.body_id == Some(node.body_id))
        .collect();
    let n_disc = disc_sigs.len();

    // Rings and belts, innermost first as the scan lists them — unless one was scanned
    // itself and is already a child.
    let rings: Vec<&BodyRing> = body
        .map(|b| b.rings.iter().filter(|r| !node.children.iter().any(|c| c.name == r.name)).collect())
        .unwrap_or_default();
    // A belt's clusters ("… A Belt Cluster 3") hang under their belt.
    let in_ring = |c: &TreeNode, r: &BodyRing| c.name.strip_prefix(r.name.as_str()).is_some_and(|rest| rest.starts_with(' '));
    let children: Vec<&TreeNode> = node.children.iter().filter(|c| !rings.iter().any(|r| in_ring(c, r))).collect();
    let n_children = children.len();
    let n_rings = rings.len();
    for (i, ring) in rings.iter().enumerate() {
        let is_last = i == n_rings - 1 && n_children == 0 && n_sigs == 0 && n_disc == 0 && n_orbiting == 0;
        let conn = if is_last { "└─ " } else { "├─ " };
        result.push(FlatNode {
            tree_prefix: format!("{}{}", child_prefix, conn),
            short_name: ring.name.strip_prefix(&node.name).unwrap_or(&ring.name).trim().to_string(),
            body_id: node.body_id,
            distance_ls: 0.0,
            has_rings: false,
            landable: false,
            planet_class: String::new(),
            star_type: String::new(),
            is_barycentre: false,
            bio_signal_count: 0,
            geo_signal_count: 0,
            node_type: NodeType::Ring((*ring).clone()),
            radius: 0.0,
            hidden: 0,
        });
        let ring_prefix = format!("{}{}", child_prefix, if is_last { "   " } else { "│  " });
        let clusters: Vec<&TreeNode> = node.children.iter().filter(|c| in_ring(c, ring)).collect();
        for (j, cluster) in clusters.iter().enumerate() {
            let row = result.len();
            flatten_node(cluster, &ring_prefix, j == clusters.len() - 1, result, system_name, fss_signals, saa_data, discovered_signals, stations, placed_stations);
            // "Cluster 3" under its belt row rather than "A A Belt Cluster 3".
            result[row].short_name = cluster.name[ring.name.len()..].trim().to_string();
        }
    }

    // Orbital children
    for (i, child) in children.iter().enumerate() {
        let is_last = i == n_children - 1 && n_sigs == 0 && n_disc == 0 && n_orbiting == 0;
        flatten_node(child, &child_prefix, is_last, result, system_name, fss_signals, saa_data, discovered_signals, stations, placed_stations);
    }

    // Bio/geo signal children
    for (j, sig) in body_signals.iter().enumerate() {
        let is_last = j == n_sigs - 1 && n_disc == 0 && n_orbiting == 0;
        let sig_connector = if is_last { "└─ " } else { "├─ " };
        let sig_prefix = format!("{}{}", child_prefix, sig_connector);
        let label = format!("{} ×{}", sig.display_type(), sig.count);
        result.push(FlatNode {
            tree_prefix: sig_prefix,
            short_name: label,
            body_id: node.body_id,
            distance_ls: 0.0,
            has_rings: false,
            landable: false,
            planet_class: String::new(),
            star_type: String::new(),
            is_barycentre: false,
            bio_signal_count: 0,
            geo_signal_count: 0,
            node_type: NodeType::BodySignal(sig.clone()),
            radius: 0.0,
            hidden: 0,
        });
    }

    // Body-associated system signals (FssSignalDiscovered with BodyID)
    for (k, sig) in disc_sigs.iter().enumerate() {
        let is_last = k == n_disc - 1 && n_orbiting == 0;
        let sig_connector = if is_last { "└─ " } else { "├─ " };
        let sig_prefix = format!("{}{}", child_prefix, sig_connector);
        result.push(FlatNode {
            tree_prefix: sig_prefix,
            short_name: sig.display_name.clone(),
            body_id: node.body_id,
            distance_ls: 0.0,
            has_rings: false,
            landable: false,
            planet_class: String::new(),
            star_type: String::new(),
            is_barycentre: false,
            bio_signal_count: 0,
            geo_signal_count: 0,
            node_type: NodeType::Signal((*sig).clone()),
            radius: 0.0,
            hidden: 0,
        });
    }

    // Orbiting stations as children of this body
    for (i, station) in orbiting.iter().enumerate() {
        let conn = if i == n_orbiting - 1 { "└─ " } else { "├─ " };
        let prefix = format!("{}{}", child_prefix, conn);
        result.push(FlatNode {
            tree_prefix: prefix,
            short_name: carrier_display_name(station, discovered_signals),
            body_id: -1,
            distance_ls: station.dist_from_star_ls,
            has_rings: false,
            landable: false,
            planet_class: String::new(),
            star_type: String::new(),
            is_barycentre: false,
            bio_signal_count: 0,
            geo_signal_count: 0,
            node_type: NodeType::Station((*station).clone()),
            radius: 0.0,
            hidden: 0,
        });
    }
}

/// Nesting level of a tree row from its `├─ `/`│  ` prefix (0 for rows outside the tree).
fn tree_depth(prefix: &str) -> usize {
    prefix.chars().count() / 3
}

/// Drops the rows below each collapsed body that sit deeper than it, recording how many
/// were hidden on the body's row.
fn fold_collapsed(nodes: Vec<FlatNode>, collapsed: &HashSet<i32>) -> Vec<FlatNode> {
    let mut out: Vec<FlatNode> = Vec::with_capacity(nodes.len());
    let mut fold: Option<(usize, usize)> = None; // (row in `out`, its depth)
    for node in nodes {
        let depth = tree_depth(&node.tree_prefix);
        if let Some((row, d)) = fold {
            if depth > d {
                out[row].hidden += 1;
                continue;
            }
            fold = None;
        }
        if matches!(node.node_type, NodeType::Body) && collapsed.contains(&node.body_id) {
            fold = Some((out.len(), depth));
        }
        out.push(node);
    }
    out
}

/// Collects the ids of the bodies above `body_id` in the tree into `path`; whether it is there.
fn body_path(nodes: &[TreeNode], body_id: i32, path: &mut Vec<i32>) -> bool {
    for node in nodes {
        if node.body_id == body_id {
            return true;
        }
        path.push(node.body_id);
        if body_path(&node.children, body_id, path) {
            return true;
        }
        path.pop();
    }
    false
}

/// Every scanned body in the tree.
fn tree_bodies<'a>(nodes: &'a [TreeNode], out: &mut Vec<&'a BodyScan>) {
    for node in nodes {
        out.extend(node.data.as_ref());
        tree_bodies(&node.children, out);
    }
}

/// What the exobiology prediction needs to know of `body` in the system `tree` at
/// `position`: `None` for a body nobody can land on, or one the commander didn't scan.
pub(crate) fn exobiology_facts(tree: &[TreeNode], position: Option<[f32; 3]>, body: &BodyScan) -> Option<BodyFacts> {
    let exo = body.exo.as_ref().filter(|_| body.landable)?;
    let mut bodies = Vec::new();
    tree_bodies(tree, &mut bodies);
    let stars: Vec<&BodyScan> = bodies.into_iter().filter(|b| !b.star_type.is_empty()).collect();
    // The main star has the lowest body ID; the star a body orbits names it ("X B 3" orbits
    // "X B"), else it's the main star.
    let main = stars.iter().min_by_key(|b| b.body_id);
    let parent = stars
        .iter()
        .filter(|s| body.body_name.strip_prefix(s.body_name.as_str()).is_some_and(|rest| rest.starts_with(' ')))
        .max_by_key(|s| s.body_name.len())
        .or(main);
    let star_type = |s: Option<&&BodyScan>| s.map(|s| s.star_type.clone()).unwrap_or_default();
    Some(BodyFacts {
        planet_class: body.planet_class.clone(),
        atmosphere_type: exo.atmosphere_type.clone(),
        so2_percent: exo.so2_percent,
        surface_gravity: body.surface_gravity,
        surface_temperature: body.surface_temperature,
        surface_pressure: exo.surface_pressure,
        volcanism: body.volcanism.clone(),
        parent_star: star_type(parent),
        main_star: star_type(main),
        position,
    })
}

fn find_body_in_tree<'a>(nodes: &'a [TreeNode], body_id: i32) -> Option<&'a BodyScan> {
    for node in nodes {
        if node.body_id == body_id {
            return node.data.as_ref();
        }
        if let Some(found) = find_body_in_tree(&node.children, body_id) {
            return Some(found);
        }
    }
    None
}

// ── Icons & styling ──────────────────────────────────────────────────────────

fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "white"        => Color::White,
        "black"        => Color::Black,
        "red"          => Color::Red,
        "green"        => Color::Green,
        "blue"         => Color::Blue,
        "yellow"       => Color::Yellow,
        "cyan"         => Color::Cyan,
        "magenta"      => Color::Magenta,
        "gray" | "grey"             => Color::Gray,
        "darkgray" | "darkgrey"     => Color::DarkGray,
        "lightred"     => Color::LightRed,
        "lightgreen"   => Color::LightGreen,
        "lightyellow"  => Color::LightYellow,
        "lightblue"    => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan"    => Color::LightCyan,
        s if s.starts_with('#') && s.len() == 7 => {
            let r = u8::from_str_radix(&s[1..3], 16).unwrap_or(255);
            let g = u8::from_str_radix(&s[3..5], 16).unwrap_or(255);
            let b = u8::from_str_radix(&s[5..7], 16).unwrap_or(255);
            Color::Rgb(r, g, b)
        }
        _ => Color::White,
    }
}

/// The colour a body is drawn in, from the star / planet / icon settings.
fn node_style(node: &FlatNode, settings: &Settings) -> Style {
    if node.planet_class.is_empty() {
        if node.is_barycentre {
            return Style::default().fg(Color::DarkGray);
        }
        if is_belt_cluster(node) {
            let color = settings.icons.get("belt_cluster").filter(|i| i.enabled)
                .map(|i| parse_color(&i.color))
                .unwrap_or(Color::Rgb(160, 120, 80));
            return Style::default().fg(color);
        }
        let color = settings.stars.get(&node.star_type).filter(|i| i.enabled)
            .map(|i| parse_color(&i.color))
            .unwrap_or(crate::theme::accent());
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        body_class_style(&node.planet_class, settings)
    }
}

fn body_class_style(planet_class: &str, settings: &Settings) -> Style {
    let lower = planet_class.to_lowercase();
    let is_gas = lower.contains("gas") || lower.contains("sudarsky") || lower.contains("giant");

    // Color: settings → hardcoded class default
    let (color, bold) = settings.planets.get(planet_class).filter(|i| i.enabled)
        .map(|i| (parse_color(&i.color), false))
        .unwrap_or_else(|| {
            if lower.contains("earthlike")                                          { (Color::Green, true) }
            else if lower.contains("water world") || lower.contains("water giant") { (Color::Blue, false) }
            else if lower.contains("ammonia")                                       { (Color::LightMagenta, false) }
            else if lower.contains("metal rich")                                    { (Color::Red, false) }
            else if lower.contains("high metal")                                    { (Color::LightRed, false) }
            else if lower.contains("icy")                                           { (Color::Cyan, false) }
            else if lower.contains("rocky")                                         { (Color::Gray, false) }
            else if is_gas                                                           { (Color::Rgb(255, 165, 0), false) }
            else                                                                    { (Color::White, false) }
        });

    if bold { Style::default().fg(color).add_modifier(Modifier::BOLD) } else { Style::default().fg(color) }
}

/// Icon and colour for a bio/geo/other signal on a body, from the icon settings.
fn body_signal_icon(sig: &BodySignal, settings: &Settings) -> (String, Color) {
    kind_icon(sig.is_biological(), sig.is_geological(), settings)
}

/// Icon and colour for something biological, geological or else, from the icon settings.
fn kind_icon(biological: bool, geological: bool, settings: &Settings) -> (String, Color) {
    let (key, fallback, color) = if biological {
        ("bio_signal", "🌿", Color::Green)
    } else if geological {
        ("geo_signal", "🌋", Color::Yellow)
    } else {
        ("unknown_signal", "◆", Color::White)
    };
    settings.icons.get(key).filter(|i| i.enabled)
        .map(|i| (i.char.clone(), parse_color(&i.color)))
        .unwrap_or_else(|| (fallback.to_string(), color))
}

fn signal_icon(sig: &DiscoveredSignal, settings: &Settings) -> (String, Color) {
    if sig.uss_type.is_some() {
        let threat = sig.threat_level.unwrap_or(0);
        if threat >= 4 {
            ("⚠".to_string(), Color::Red)
        } else if threat >= 2 {
            ("⚠".to_string(), crate::theme::accent())
        } else {
            ("⚠".to_string(), Color::Yellow)
        }
    } else if sig.is_station {
        ("◉".to_string(), Color::Cyan)
    } else {
        let lower = sig.display_name.to_lowercase();
        if lower.contains("war") || lower.contains("combat") || lower.contains("conflict") {
            settings.icons.get("human_signal").filter(|i| i.enabled)
                .map(|i| (i.char.clone(), parse_color(&i.color)))
                .unwrap_or_else(|| ("⚔".to_string(), Color::Red))
        } else {
            settings.icons.get("unknown_signal").filter(|i| i.enabled)
                .map(|i| (i.char.clone(), parse_color(&i.color)))
                .unwrap_or_else(|| ("◆".to_string(), Color::White))
        }
    }
}

// ── System info helpers ──────────────────────────────────────────────────────

use edcas_common::format::abbreviate as format_population;

// ── Utilities ────────────────────────────────────────────────────────────────

fn strip_system_prefix(name: &str, system_name: &str) -> String {
    if system_name.is_empty() || name == system_name {
        return name.to_string();
    }
    if let Some(rest) = name.strip_prefix(system_name) {
        let trimmed = rest.trim_start();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    name.to_string()
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Whether a row is a belt cluster: "A Belt Cluster 2", or "Cluster 2" under its belt row.
fn is_belt_cluster(node: &FlatNode) -> bool {
    let lower = node.short_name.to_ascii_lowercase();
    lower.contains("belt cluster") || lower.starts_with("cluster ")
}

/// A ring class as players read it ("eRingClass_MetalRich" → "MetalRich").
fn ring_class_name(ring_class: &str) -> String {
    ring_class.strip_prefix("eRingClass_").unwrap_or(ring_class).replace('_', " ")
}

fn is_valuable_ring(ring_class: &str) -> bool {
    let lower = ring_class.to_lowercase();
    lower.contains("metal") || lower.contains("rocky")
}

use edcas_common::api::StationSearchResult;
use edcas_common::format::group_thousands as format_thousands;

/// A server station row as the tree shows stations (placed by its distance from the star).
pub(crate) fn station_from_api(s: &StationSearchResult) -> StationData {
    StationData {
        name: s.name.clone(),
        station_type: s.station_type.clone().unwrap_or_default(),
        system_name: s.system_name.clone(),
        dist_from_star_ls: s.dist_from_star_ls.unwrap_or(0.0),
        services: Vec::new(),
        economy: String::new(),
        secondary_economies: Vec::new(),
        faction: String::new(),
        government: String::new(),
        allegiance: String::new(),
        landing_pads: None,
        market_id: s.market_id,
        host_body_id: None,
        commodities: Vec::new(),
    }
}

/// Whether an FSS signal is a station the tree already shows as a station row.
fn is_known_station_signal(sig: &DiscoveredSignal, stations: &[StationData]) -> bool {
    stations.iter().any(|s| signal_names_station(sig, s))
}

/// Whether an FSS signal is this station: the same name, or for a fleet carrier the
/// signal's "NAME CALLSIGN" ending in the station's callsign.
fn signal_names_station(sig: &DiscoveredSignal, station: &StationData) -> bool {
    sig.display_name == station.name
        || (station.station_type == "FleetCarrier"
            && sig.signal_type.as_deref() == Some("FleetCarrier")
            && sig.display_name.ends_with(&station.name))
}

/// The name a colonisation construction site builds ("Orbital Construction Site: Thirsk
/// City" → "Thirsk City").
fn construction_site_target(station: &StationData) -> Option<&str> {
    if !station.station_type.contains("ConstructionDepot") {
        return None;
    }
    station.name.split_once(": ").map(|(_, name)| name)
}

/// Planets and moons a station can orbit or sit on: (body id, distance from the arrival
/// star, landable).
fn orbitable_bodies(nodes: &[TreeNode], out: &mut Vec<(i32, f32, bool)>) {
    for node in nodes {
        if let Some(b) = &node.data {
            let barycentre = node.name.to_ascii_lowercase().contains("barycentre");
            if b.star_type.is_empty() && !barycentre && b.distance_from_arrival_ls > 0.0 {
                out.push((node.body_id, b.distance_from_arrival_ls, b.landable));
            }
        }
        orbitable_bodies(&node.children, out);
    }
}

/// The body a station orbits. The journal names it only when the commander dropped at the
/// body before docking (`host_body_id`); otherwise it is the body whose distance from the
/// arrival star is closest to the station's, within 2 Ls or 0.5 % — a station sits within
/// a few hundredths of a light second of its body, which only drifts by its orbit since
/// the scan. Settlements and surface ports only sit on landable bodies; a gas giant's moons
/// are often nearer to the giant's distance than to each other's.
fn station_host(station: &StationData, bodies: &[(i32, f32, bool)]) -> Option<i32> {
    if station.host_body_id.is_some() {
        return station.host_body_id;
    }
    if station.dist_from_star_ls <= 0.0 {
        return None;
    }
    let on_surface = matches!(
        station.station_type.as_str(),
        "OnFootSettlement" | "CraterOutpost" | "CraterPort" | "PlanetaryConstructionDepot"
    );
    bodies
        .iter()
        .filter(|&&(_, _, landable)| landable || !on_surface)
        .map(|&(id, d, _)| (id, (station.dist_from_star_ls - d).abs(), d))
        .filter(|&(_, diff, d)| diff < (d * 0.005).max(2.0))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, ..)| id)
}

/// Resolves where stations and station signals go in the body tree. `FSSSignalDiscovered`
/// carries no body, so a signal is placed through the station it names: that station's row
/// stands for it, and a finished colony build ("Thirsk City") takes over the position of
/// its construction site, which is history once the FSS sees the finished station.
fn place_in_tree(
    tree: &[TreeNode],
    stations: &[StationData],
    signals: &[DiscoveredSignal],
) -> (Vec<StationData>, Vec<DiscoveredSignal>) {
    let mut bodies = Vec::new();
    orbitable_bodies(tree, &mut bodies);
    let mut placed = Vec::with_capacity(stations.len());
    let mut finished_sites: Vec<(&str, Option<i32>)> = Vec::new();
    for station in stations {
        let host = station_host(station, &bodies);
        if let Some(target) = construction_site_target(station) {
            let built = signals.iter().any(|s| s.display_name == target)
                || stations.iter().any(|s| s.name == target);
            if built {
                finished_sites.push((target, host));
                continue;
            }
        }
        let mut station = station.clone();
        station.host_body_id = host;
        placed.push(station);
    }
    let signals = signals
        .iter()
        .map(|sig| {
            let mut sig = sig.clone();
            if sig.body_id.is_none() {
                sig.body_id = finished_sites.iter().find(|(name, _)| *name == sig.display_name).and_then(|&(_, host)| host);
            }
            sig
        })
        .collect();
    (placed, signals)
}

/// Returns the enriched display name for a fleet carrier station by looking up
/// the matching FSS signal (which includes the full owner+callsign name).
fn carrier_display_name(station: &StationData, signals: &[DiscoveredSignal]) -> String {
    if station.station_type != "FleetCarrier" {
        return station.name.clone();
    }
    signals
        .iter()
        .filter(|s| s.signal_type.as_deref() == Some("FleetCarrier"))
        .find(|s| s.display_name == station.name || s.display_name.ends_with(&station.name))
        .map(|s| s.display_name.clone())
        .unwrap_or_else(|| station.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(body_id: i32, name: &str, dist: f32, star: bool, children: Vec<TreeNode>) -> TreeNode {
        let landable = name.ends_with(" a");
        TreeNode {
            name: name.into(),
            body_id,
            children,
            data: Some(BodyScan {
                body_id,
                body_name: name.into(),
                planet_class: if star { String::new() } else { "Icy body".into() },
                landable,
                scan_type: String::new(),
                distance_from_arrival_ls: dist,
                radius: 0.0,
                mass_em: 0.0,
                surface_temperature: 0.0,
                surface_gravity: 0.0,
                tidal_lock: false,
                volcanism: String::new(),
                atmosphere: String::new(),
                terraform_state: String::new(),
                star_type: if star { "M".into() } else { String::new() },
                parents: Vec::new(),
                rings: Vec::new(),
                materials: Vec::new(),
                value: None,
                mapped: None,
                footfall: false,
                composition: None,
                discovery: None,
                exo: None,
            }),
        }
    }

    fn station(market_id: i64, name: &str, station_type: &str, dist: f32) -> StationData {
        station_from_api(&StationSearchResult {
            market_id,
            system_address: 0,
            system_name: String::new(),
            name: name.into(),
            carrier_name: None,
            station_type: Some(station_type.into()),
            dist_from_star_ls: Some(dist),
            updated_at: Default::default(),
        })
    }

    fn signal(name: &str, signal_type: &str) -> DiscoveredSignal {
        DiscoveredSignal {
            display_name: name.into(),
            signal_type: Some(signal_type.into()),
            uss_type: None,
            spawning_state: None,
            spawning_faction: None,
            threat_level: None,
            time_remaining: None,
            is_station: false,
            body_id: None,
        }
    }

    fn scan_line(body_id: i32, name: &str, scan_type: &str, parent: Option<i32>) -> String {
        let parents = parent.map_or(String::new(), |p| format!(r#""Parents":[ {{"Star":{p}}} ], "#));
        let kind = if parent.is_some() { r#""PlanetClass":"Icy body", "MassEM":0.1"# } else { r#""StarType":"M", "StellarMass":0.4"# };
        format!(
            r#"{{ "timestamp":"2026-10-13T09:00:00Z", "event":"Scan", "ScanType":"{scan_type}", "BodyName":"{name}", "BodyID":{body_id}, {parents}"StarSystem":"Oochont", "SystemAddress":42, "DistanceFromArrivalLS":{body_id}.0, {kind}, "WasDiscovered":true, "WasMapped":false }}"#
        )
    }

    #[test]
    fn the_selection_follows_the_newest_fss_scan() {
        let mut journal = JournalData::new();
        journal.process_line(r#"{ "timestamp":"2026-10-13T09:00:00Z", "event":"FSDJump", "StarSystem":"Oochont", "SystemAddress":42, "StarPos":[0.0,0.0,0.0], "Population":0 }"#);
        journal.process_line(&scan_line(0, "Oochont A", "AutoScan", None));
        journal.process_line(&scan_line(3, "Oochont 1", "Detailed", Some(0)));
        journal.process_line(&scan_line(5, "Oochont 2", "Detailed", Some(0)));
        let mut view = ExplorerView::new();
        view.update(&journal);
        let selected = |v: &ExplorerView| v.flat_nodes[v.selected_idx].body_id;
        assert_eq!(selected(&view), 5);

        // The commander moves on; an AutoScan doesn't take the selection away…
        view.selected_idx = view.body_row(0).unwrap();
        journal.process_line(&scan_line(4, "Oochont 1 a", "AutoScan", Some(0)));
        view.update(&journal);
        assert_eq!(selected(&view), 0);
        // …the next FSS scan does, even into a folded subtree.
        view.collapsed.insert(0);
        journal.process_line(&scan_line(3, "Oochont 1", "Detailed", Some(0)));
        view.update(&journal);
        assert_eq!(selected(&view), 3);
    }

    #[test]
    fn the_selection_follows_the_body_targeted_or_approached_in_the_game() {
        let mut journal = JournalData::new();
        journal.process_line(r#"{ "timestamp":"2026-10-13T09:00:00Z", "event":"FSDJump", "StarSystem":"Oochont", "SystemAddress":42, "StarPos":[0.0,0.0,0.0], "Population":0 }"#);
        journal.process_line(&scan_line(0, "Oochont A", "AutoScan", None));
        journal.process_line(&scan_line(3, "Oochont 1", "AutoScan", Some(0)));
        journal.process_line(&scan_line(5, "Oochont 2", "AutoScan", Some(0)));
        let mut view = ExplorerView::new();
        view.update(&journal);
        let selected = |v: &ExplorerView| v.flat_nodes[v.selected_idx].body_id;
        let target = |system: i64, body: i32, name: &str| crate::journal_reader::StatusData {
            valid: true,
            destination_system: system,
            destination_body: body,
            destination: name.to_string(),
            ..Default::default()
        };

        // Targeting a body in the game selects it…
        journal.take_status(target(42, 5, "Oochont 2"));
        view.update(&journal);
        assert_eq!(selected(&view), 5);
        // …once: a status poll with the same target leaves the commander's own pick alone.
        view.selected_idx = view.body_row(0).unwrap();
        journal.take_status(target(42, 5, "Oochont 2"));
        view.update(&journal);
        assert_eq!(selected(&view), 0);
        // A station or another system's body is no body here.
        journal.take_status(target(42, 3, "Some Orbital"));
        journal.take_status(target(7, 3, "Oochont 1"));
        view.update(&journal);
        assert_eq!(selected(&view), 0);

        journal.process_line(r#"{ "timestamp":"2026-10-13T09:01:00Z", "event":"ApproachBody", "StarSystem":"Oochont", "SystemAddress":42, "Body":"Oochont 1", "BodyID":3 }"#);
        view.update(&journal);
        assert_eq!(selected(&view), 3);

        journal.process_line(r#"{ "timestamp":"2026-10-13T09:02:00Z", "event":"SupercruiseExit", "StarSystem":"Oochont", "SystemAddress":42, "Body":"Oochont 2", "BodyID":5, "BodyType":"Planet" }"#);
        view.update(&journal);
        assert_eq!(selected(&view), 5);
    }

    /// Antliae Sector KM-W c1-25 as the server knows it: A 5 and A 6 1.5 Ls apart, A 8 with a
    /// moon 0.4 Ls from it.
    fn system() -> Vec<TreeNode> {
        vec![node(1, "A", 0.0, true, vec![
            node(5, "A 3", 358.1, false, vec![]),
            node(7, "A 5", 617.0, false, vec![]),
            node(8, "A 6", 618.5, false, vec![]),
            node(10, "A 8", 1935.9, false, vec![node(11, "A 8 a", 1935.5, false, vec![])]),
        ])]
    }

    #[test]
    fn stations_go_to_the_nearest_body() {
        let stations = [
            station(1, "Selous Legacy", "Outpost", 357.8),
            station(2, "Planetary Construction Site: Stebbins Gateway", "PlanetaryConstructionDepot", 1935.6),
            station(3, "Far Away", "Coriolis", 5000.0),
        ];
        let (placed, _) = place_in_tree(&system(), &stations, &[]);
        let host = |id: i64| placed.iter().find(|s| s.market_id == id).unwrap().host_body_id;
        assert_eq!(host(1), Some(5));
        // Within tolerance of both A 8 and A 8 a: the closer one, not the first one walked.
        assert_eq!(host(2), Some(11));
        assert_eq!(host(3), None);
    }

    #[test]
    fn settlements_only_sit_on_landable_bodies() {
        // Nearer to A 8 (an icy giant here) than to its landable moon A 8 a.
        let stations = [station(1, "Gowon Drilling Base", "OnFootSettlement", 1935.8)];
        let (placed, _) = place_in_tree(&system(), &stations, &[]);
        assert_eq!(placed[0].host_body_id, Some(11));
    }

    #[test]
    fn finished_colony_signal_takes_its_construction_site_position() {
        let stations = [station(1, "Orbital Construction Site: Thirsk City", "SpaceConstructionDepot", 618.2)];
        let signals = [signal("Thirsk City", "Installation"), signal("Collins Terminal", "Installation")];
        let (placed, signals) = place_in_tree(&system(), &stations, &signals);
        assert!(placed.is_empty(), "the finished site is no longer listed");
        assert_eq!(signals[0].body_id, Some(8));
        assert_eq!(signals[1].body_id, None, "no station data, stays a system signal");
    }

    // Hegoea EN-F b39-0 B 6, 2026-10-14: 2 biological signals and 11 mining sites.
    #[test]
    fn mining_sites_stay_out_of_the_tree() {
        let mut journal = JournalData::new();
        journal.process_line(r#"{ "timestamp":"2026-10-13T09:00:00Z", "event":"FSDJump", "StarSystem":"Oochont", "SystemAddress":42, "StarPos":[0.0,0.0,0.0], "Population":0 }"#);
        journal.process_line(&scan_line(0, "Oochont A", "AutoScan", None));
        journal.process_line(&scan_line(3, "Oochont 1", "Detailed", Some(0)));
        journal.process_line(r#"{ "timestamp":"2026-10-13T09:01:00Z", "event":"SAASignalsFound", "BodyName":"Oochont 1", "SystemAddress":42, "BodyID":3, "Signals":[ { "Type":"$SAA_SignalType_Biological;", "Type_Localised":"Biologisch", "Count":2 }, { "Type":"$PlanetaryMiningLocation_Name;", "Type_Localised":"Planetarer Abbaustandort", "Count":11 } ], "Genuses":[] }"#);
        let mut view = ExplorerView::new();
        view.update(&journal);
        let signals: Vec<&str> = view
            .flat_nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::BodySignal(_)))
            .map(|n| n.short_name.as_str())
            .collect();
        assert_eq!(signals.len(), 1);
        assert!(!signals[0].contains("Abbaustandort"));
        // The body's details still have them.
        assert_eq!(journal.saa_data[&3].signals.len(), 2);
    }

    #[test]
    fn station_signals_are_covered_by_their_station_row() {
        let stations = [station(1, "Selous Legacy", "Outpost", 357.8), station(2, "Q2K-BHB", "FleetCarrier", 0.0)];
        assert!(is_known_station_signal(&signal("Selous Legacy", "Outpost"), &stations));
        assert!(is_known_station_signal(&signal("FUXBAU Q2K-BHB", "FleetCarrier"), &stations));
        assert!(!is_known_station_signal(&signal("Kushida Town", "Installation"), &stations));
    }
}
