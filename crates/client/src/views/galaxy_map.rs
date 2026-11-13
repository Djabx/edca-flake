//! Galaxy Map tab: a 3D view of every system the EDCAS server knows, with the commander's
//! position marked, a system search, and the picked system's bodies in the Explorer's tree.
//!
//! The server holds millions of systems, so the map works at three levels of detail: a
//! coarse density grid of the whole charted galaxy (fetched once), an even sample of the
//! systems within a few thousand ly of the camera's focus, and every system within a few
//! hundred ly of it (both fetched again as the focus moves).
//! Drawing lives in `gui.rs` (with the OpenGL point renderer in `stars.rs`).

mod camera;
mod gui;
mod stars;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use edcas_common::api::{GalaxyDensity, GalaxySample, GalaxySystem, StationSearchResult, SystemResponse, SystemSuggestion};
use tracing::{info, warn};

use camera::{Camera, V3};
use stars::PointCloud;

use super::explorer::station_from_api;
use super::{ExplorerView, ViewEvent};
use crate::api_client::ApiClient;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::JournalData;

/// A background request's result, picked up by `poll`.
type Slot<T> = Arc<Mutex<Option<T>>>;
/// A region fetch's answer with what was asked for: centre, radius and limit.
type Fetched<T> = (V3, f32, u32, Result<T, String>);

/// Individual systems are fetched only once the camera is this close to its focus (the
/// largest sphere, 250 ly, stops showing at a fifth of the camera distance); farther out
/// the sample and the density grid show the shape.
const DETAIL_RANGE: f32 = 1_250.0;
/// The sampled middle layer is fetched once the camera is this close to its focus (its
/// 3 000 ly sphere likewise stops showing beyond).
const SAMPLE_RANGE: f32 = 15_000.0;
/// Sample radii come in steps of this factor, so small zooms reuse the current sample.
const SAMPLE_STEP: f32 = 1.5;
/// The camera must rest this long before the neighbourhood is fetched again.
const DETAIL_SETTLE: Duration = Duration::from_millis(300);
/// Typing pause before a search suggestion request.
const SUGGEST_DELAY: Duration = Duration::from_millis(200);
/// Distance the camera settles at when it flies to a system.
const SYSTEM_VIEW_DISTANCE: f32 = 60.0;

/// The whole charted galaxy, seen from above the bubble's side of it.
fn galaxy_view() -> Camera {
    Camera { target: [0.0, 0.0, 25_000.0], distance: 95_000.0, yaw: 0.0, pitch: 70f32.to_radians() }
}

#[derive(Default)]
enum Load {
    #[default]
    Idle,
    Loading,
    Failed(String),
    Ready,
}

/// The systems drawn individually around the focus.
#[derive(Default)]
struct Nearby {
    systems: Vec<GalaxySystem>,
    cloud: PointCloud,
    center: V3,
    /// Radius around `center` the list is complete for.
    radius: f32,
    /// The server's limit cut the list short, so a larger radius would add nothing.
    capped: bool,
    /// The most systems asked for (the "Stars Near Focus" setting at the time).
    limit: u32,
}

/// An even sample of the systems around the focus: what the middle zoom range shows.
#[derive(Default)]
struct Sampled {
    cloud: PointCloud,
    center: V3,
    radius: f32,
    /// Points asked for (the "Stars Sampled Around" setting at the time).
    limit: u32,
}

/// Radius of the sampled layer for a camera at `distance` from its focus: about the view's
/// size, in steps of [`SAMPLE_STEP`] from 300 ly up to the server's 3 000 ly.
fn sample_radius(distance: f32) -> f32 {
    let raw = (distance * 1.5).clamp(300.0, 3_000.0);
    let k = ((raw / 300.0).ln() / SAMPLE_STEP.ln()).round();
    (300.0 * SAMPLE_STEP.powf(k)).min(3_000.0)
}

/// The system picked by search or click, and what the server knows about it.
struct Selected {
    address: i64,
    name: String,
    pos: Option<V3>,
    info: Option<SystemResponse>,
    loading: bool,
    error: Option<String>,
    /// Fly there once the coordinates arrive (a search pick).
    fly_on_load: bool,
}

struct SystemDetails {
    info: SystemResponse,
    bodies: Vec<edcas_common::api::BodyResponse>,
    stations: Vec<StationSearchResult>,
}

/// A camera move animated over `duration`.
struct Flight {
    from: Camera,
    to: Camera,
    start: Instant,
    duration: Duration,
}

pub struct GalaxyMapView {
    camera: Camera,
    flight: Option<Flight>,
    /// The camera has been put somewhere deliberately (on the commander, or by the user);
    /// until then it shows the whole galaxy and moves to the commander once known.
    placed: bool,
    camera_moved: Instant,

    density: PointCloud,
    cell_ly: f32,
    max_log: f32,
    total_systems: i64,
    density_load: Load,
    density_rx: Slot<Result<GalaxyDensity, String>>,

    nearby: Nearby,
    nearby_load: Load,
    nearby_rx: Slot<Fetched<Vec<GalaxySystem>>>,

    sampled: Sampled,
    sampled_load: Load,
    sampled_rx: Slot<Fetched<GalaxySample>>,
    /// Stars per layer, from Settings → Galaxy Map.
    nearby_limit: u32,
    sample_limit: u32,

    search: String,
    suggestions: Vec<SystemSuggestion>,
    suggest_due: Option<Instant>,
    suggest_rx: Slot<(String, Result<Vec<SystemSuggestion>, String>)>,
    /// Enter was pressed before suggestions arrived: take the first one when they do.
    pick_first: bool,
    search_status: String,
    /// `/` asks the window to put the cursor in the search field.
    focus_search: bool,

    selected: Option<Selected>,
    selected_rx: Slot<(i64, Result<SystemDetails, String>)>,
    /// The picked system's bodies and stations, drawn with the Explorer's tree and details.
    system_map: ExplorerView,

    generation: u64,
    gui: gui::GuiState,
}

impl GalaxyMapView {
    pub fn new() -> Self {
        Self {
            camera: galaxy_view(),
            flight: None,
            placed: false,
            camera_moved: Instant::now(),
            density: PointCloud::default(),
            cell_ly: 500.0,
            max_log: 1.0,
            total_systems: 0,
            density_load: Load::Idle,
            density_rx: Arc::default(),
            nearby: Nearby::default(),
            nearby_load: Load::Idle,
            nearby_rx: Arc::default(),
            sampled: Sampled::default(),
            sampled_load: Load::Idle,
            sampled_rx: Arc::default(),
            nearby_limit: crate::settings::galaxy_map::GalaxyMapSettings::default().nearby_stars,
            sample_limit: crate::settings::galaxy_map::GalaxyMapSettings::default().sampled_stars,
            search: String::new(),
            suggestions: Vec::new(),
            suggest_due: None,
            suggest_rx: Arc::default(),
            pick_first: false,
            search_status: String::new(),
            focus_search: false,
            selected: None,
            selected_rx: Arc::default(),
            system_map: ExplorerView::new(),
            generation: 0,
            gui: gui::GuiState::default(),
        }
    }

    fn next_generation(&mut self) -> u64 {
        self.generation += 1;
        self.generation
    }

    /// Starts loading the galaxy overview the first time the tab is shown (and after a
    /// failure, so revisiting the tab retries).
    pub fn on_enter(&mut self, api: &ApiClient) {
        if matches!(self.density_load, Load::Idle | Load::Failed(_)) {
            self.density_load = Load::Loading;
            let slot = Arc::clone(&self.density_rx);
            let api_owned = api.clone();
            api.spawn(async move {
                let result = api_owned.fetch_galaxy_density().await.map_err(|e| format!("{e:#}"));
                *slot.lock().unwrap() = Some(result);
            });
        }
    }

    /// Picks up finished background requests.
    pub fn poll(&mut self, api: &ApiClient) {
        let density = self.density_rx.lock().unwrap().take();
        if let Some(result) = density {
            match result {
                Ok(d) => {
                    info!("Galaxy map: {} density cells, {} systems", d.cells.len(), d.total_systems);
                    let cell = d.cell_ly;
                    let max = d.cells.iter().map(|c| c[3]).max().unwrap_or(1);
                    let data = d
                        .cells
                        .iter()
                        .map(|c| [(c[0] as f32 + 0.5) * cell, (c[1] as f32 + 0.5) * cell, (c[2] as f32 + 0.5) * cell, c[3] as f32])
                        .collect();
                    self.cell_ly = cell;
                    self.max_log = (1.0 + max as f32).ln();
                    self.total_systems = d.total_systems;
                    self.density = PointCloud { data: Arc::new(data), generation: self.next_generation() };
                    self.density_load = Load::Ready;
                }
                Err(e) => {
                    warn!("Galaxy map: density fetch failed: {e}");
                    self.density_load = Load::Failed(e);
                }
            }
        }

        let nearby = self.nearby_rx.lock().unwrap().take();
        if let Some((center, radius, limit, result)) = nearby {
            match result {
                Ok(systems) => {
                    let capped = systems.len() as u32 >= limit;
                    // A capped list is complete only out to its farthest system.
                    let radius = if capped {
                        systems.last().map(|s| camera::dist([s.x, s.y, s.z], center)).unwrap_or(radius)
                    } else {
                        radius
                    };
                    let data = systems.iter().map(|s| [s.x, s.y, s.z, if s.population > 0 { 1.0 } else { 0.0 }]).collect();
                    let generation = self.next_generation();
                    self.nearby = Nearby { systems, cloud: PointCloud { data: Arc::new(data), generation }, center, radius, capped, limit };
                    self.nearby_load = Load::Ready;
                    self.gui.hovered = None;
                }
                Err(e) => {
                    warn!("Galaxy map: nearby systems fetch failed: {e}");
                    self.nearby_load = Load::Failed(e);
                }
            }
        }

        let sampled = self.sampled_rx.lock().unwrap().take();
        if let Some((center, radius, limit, result)) = sampled {
            match result {
                Ok(sample) => {
                    let generation = self.next_generation();
                    self.sampled = Sampled { cloud: PointCloud { data: Arc::new(sample.points), generation }, center, radius, limit };
                    self.sampled_load = Load::Ready;
                }
                Err(e) => {
                    warn!("Galaxy map: sample fetch failed: {e}");
                    self.sampled_load = Load::Failed(e);
                }
            }
        }

        let suggest = self.suggest_rx.lock().unwrap().take();
        if let Some((query, result)) = suggest {
            // Answers to an older query than what's in the field now are stale.
            if query == self.search.trim() {
                match result {
                    Ok(list) => {
                        self.search_status = if list.is_empty() { format!("No system starts with \"{query}\"") } else { String::new() };
                        self.suggestions = list;
                        if std::mem::take(&mut self.pick_first) {
                            self.pick_suggestion(api, 0);
                        }
                    }
                    Err(e) => {
                        self.search_status = format!("Search failed: {e}");
                        self.pick_first = false;
                    }
                }
            }
        }

        let selected = self.selected_rx.lock().unwrap().take();
        if let Some((address, result)) = selected {
            let Some(sel) = self.selected.as_mut().filter(|s| s.address == address) else { return };
            sel.loading = false;
            match result {
                Ok(details) => {
                    let pos = [details.info.x, details.info.y, details.info.z];
                    sel.pos = Some(pos);
                    sel.name = details.info.name.clone();
                    let bodies: Vec<_> = details.bodies.iter().map(crate::app::body_from_api).collect();
                    let stations = details.stations.iter().filter(|s| !is_carrier(s)).map(station_from_api).collect();
                    self.system_map.show_remote_system(&details.info.name, &bodies, stations);
                    sel.info = Some(details.info);
                    if std::mem::take(&mut sel.fly_on_load) {
                        self.fly_to(pos, SYSTEM_VIEW_DISTANCE);
                    }
                }
                Err(e) => {
                    warn!("Galaxy map: system {address} fetch failed: {e}");
                    sel.error = Some(e);
                }
            }
        }
    }

    /// Advances a camera flight. Returns whether one is still under way (keep repainting).
    fn step_flight(&mut self, now: Instant) -> bool {
        let Some(f) = &self.flight else { return false };
        let t = (now.duration_since(f.start).as_secs_f32() / f.duration.as_secs_f32()).min(1.0);
        let s = t * t * (3.0 - 2.0 * t);
        // The distance follows a curve in log space that rises over long hops, so the view
        // pulls back to show where it is going instead of skimming through the stars.
        let travel = camera::dist(f.from.target, f.to.target);
        let (d0, d1) = (f.from.distance.ln(), f.to.distance.ln());
        let mid = f.from.distance.max(f.to.distance).max(travel * 0.6).ln();
        let ln_d = (1.0 - s) * (1.0 - s) * d0 + 2.0 * s * (1.0 - s) * mid + s * s * d1;
        self.camera = Camera {
            target: camera::lerp(f.from.target, f.to.target, s),
            distance: ln_d.exp(),
            yaw: f.from.yaw + (f.to.yaw - f.from.yaw) * s,
            pitch: f.from.pitch + (f.to.pitch - f.from.pitch) * s,
        };
        self.camera_moved = now;
        if t >= 1.0 {
            self.flight = None;
        }
        true
    }

    /// Animates the camera to look at `target` from `distance`.
    fn fly_to(&mut self, target: V3, distance: f32) {
        let to = Camera { target, distance, ..self.camera };
        self.fly(to);
    }

    fn fly(&mut self, to: Camera) {
        self.placed = true;
        let hop = camera::dist(self.camera.target, to.target) / self.camera.distance.max(to.distance);
        let secs = (0.6 + 0.25 * (1.0 + hop).ln()).min(2.2);
        self.flight = Some(Flight { from: self.camera, to, start: Instant::now(), duration: Duration::from_secs_f32(secs) });
    }

    /// The user moved the camera by hand: stop any flight.
    fn touched(&mut self) {
        self.placed = true;
        self.flight = None;
        self.camera_moved = Instant::now();
    }

    pub(crate) fn commander_pos(journal: &JournalData) -> Option<V3> {
        journal.current_system.as_ref().and_then(|s| s.star_pos)
    }

    fn center_on_commander(&mut self, journal: &JournalData) {
        if let Some(pos) = Self::commander_pos(journal) {
            self.fly_to(pos, SYSTEM_VIEW_DISTANCE.max(self.camera.distance.min(400.0)));
        }
    }

    /// Fetches the sampled and the single systems around the focus when the camera has come
    /// to rest somewhere the current sets don't cover. Returns how long to wait before
    /// asking again, if a fetch is still due.
    fn maybe_fetch_around(&mut self, api: &ApiClient) -> Option<Duration> {
        [self.maybe_fetch_sample(api), self.maybe_fetch_nearby(api)].into_iter().flatten().min()
    }

    fn maybe_fetch_sample(&mut self, api: &ApiClient) -> Option<Duration> {
        if matches!(self.sampled_load, Load::Loading) || self.camera.distance > SAMPLE_RANGE {
            return None;
        }
        let want = sample_radius(self.camera.distance);
        let target = self.camera.target;
        let s = &self.sampled;
        let covered = matches!(self.sampled_load, Load::Ready | Load::Failed(_));
        let moved = camera::dist(target, s.center) > 0.3 * s.radius.max(1.0);
        if covered && !moved && want == s.radius && s.limit == self.sample_limit {
            return None;
        }
        let rest = self.camera_moved.elapsed();
        if self.flight.is_some() || rest < DETAIL_SETTLE {
            return Some(DETAIL_SETTLE.saturating_sub(rest).max(Duration::from_millis(50)));
        }
        self.sampled_load = Load::Loading;
        let slot = Arc::clone(&self.sampled_rx);
        let api_owned = api.clone();
        let limit = self.sample_limit;
        api.spawn(async move {
            let result = api_owned.fetch_galaxy_sample(target, want, limit).await.map_err(|e| format!("{e:#}"));
            *slot.lock().unwrap() = Some((target, want, limit, result));
        });
        None
    }

    fn maybe_fetch_nearby(&mut self, api: &ApiClient) -> Option<Duration> {
        if matches!(self.nearby_load, Load::Loading) || self.camera.distance > DETAIL_RANGE {
            return None;
        }
        // A little less than the camera's distance, so the camera looks at the neighbourhood
        // from outside instead of sitting in it.
        let want = (self.camera.distance * 0.8).clamp(30.0, 250.0);
        let target = self.camera.target;
        let n = &self.nearby;
        let covered = !n.systems.is_empty() || matches!(self.nearby_load, Load::Ready | Load::Failed(_));
        let moved = camera::dist(target, n.center) > 0.35 * n.radius.max(1.0);
        let too_small = !n.capped && want > n.radius * 1.6;
        if covered && !moved && !too_small && n.limit == self.nearby_limit {
            return None;
        }
        let rest = self.camera_moved.elapsed();
        if self.flight.is_some() || rest < DETAIL_SETTLE {
            return Some(DETAIL_SETTLE.saturating_sub(rest).max(Duration::from_millis(50)));
        }
        self.nearby_load = Load::Loading;
        let slot = Arc::clone(&self.nearby_rx);
        let api_owned = api.clone();
        let limit = self.nearby_limit;
        api.spawn(async move {
            let result = api_owned.fetch_galaxy_systems(target, want, limit).await.map_err(|e| format!("{e:#}"));
            *slot.lock().unwrap() = Some((target, want, limit, result));
        });
        None
    }

    /// Asks for name suggestions once typing has paused. Returns how long until then.
    fn maybe_suggest(&mut self, api: &ApiClient) -> Option<Duration> {
        let due = self.suggest_due?;
        let now = Instant::now();
        if now < due {
            return Some(due - now);
        }
        self.suggest_due = None;
        self.request_suggestions(api);
        None
    }

    fn request_suggestions(&mut self, api: &ApiClient) {
        let query = self.search.trim().to_string();
        if query.is_empty() {
            self.suggestions.clear();
            self.search_status.clear();
            return;
        }
        self.search_status = "Searching…".into();
        let slot = Arc::clone(&self.suggest_rx);
        let api_owned = api.clone();
        api.spawn(async move {
            let result = api_owned.suggest_systems(&query, 10).await.map_err(|e| format!("{e:#}"));
            *slot.lock().unwrap() = Some((query, result));
        });
    }

    /// The search text changed: suggestions follow after a short pause.
    fn search_edited(&mut self) {
        self.pick_first = false;
        if self.search.trim().is_empty() {
            self.suggestions.clear();
            self.search_status.clear();
            self.suggest_due = None;
        } else {
            self.suggest_due = Some(Instant::now() + SUGGEST_DELAY);
        }
    }

    /// Enter in the search field: take the first suggestion, now or when they arrive.
    fn search_submitted(&mut self, api: &ApiClient) {
        if self.suggest_due.is_none() && !self.suggestions.is_empty() {
            self.pick_suggestion(api, 0);
        } else if !self.search.trim().is_empty() {
            self.suggest_due = None;
            self.pick_first = true;
            self.request_suggestions(api);
        }
    }

    fn pick_suggestion(&mut self, api: &ApiClient, i: usize) {
        let Some(s) = self.suggestions.get(i).cloned() else { return };
        self.search = s.name.clone();
        self.suggestions.clear();
        self.search_status.clear();
        self.select(api, s.system_address, s.name, None, true);
    }

    /// Shows a system picked elsewhere (the Search tab's Systems) and flies to it.
    pub fn show_system(&mut self, api: &ApiClient, address: i64, name: String) {
        self.search = name.clone();
        self.suggestions.clear();
        self.search_status.clear();
        self.select(api, address, name, None, true);
    }

    /// Picks a system: shows what is known right away and fetches the rest. `fly` moves
    /// the camera there (at once if the position is known, else when it arrives).
    fn select(&mut self, api: &ApiClient, address: i64, name: String, pos: Option<V3>, fly: bool) {
        let pos = pos.or_else(|| self.selected.as_ref().filter(|s| s.address == address).and_then(|s| s.pos));
        if let (true, Some(p)) = (fly, pos) {
            self.fly_to(p, SYSTEM_VIEW_DISTANCE.min(self.camera.distance.max(SYSTEM_VIEW_DISTANCE)));
        }
        if self.selected.as_ref().is_some_and(|s| s.address == address && (s.info.is_some() || s.loading)) {
            return;
        }
        info!("Galaxy map: selected {name} ({address})");
        self.system_map.show_remote_system(&name, &[], Vec::new());
        self.selected = Some(Selected { address, name, pos, info: None, loading: true, error: None, fly_on_load: fly && pos.is_none() });
        let slot = Arc::clone(&self.selected_rx);
        let api_owned = api.clone();
        api.spawn(async move {
            let (info, bodies, stations) = tokio::join!(
                api_owned.get_system(address),
                api_owned.get_bodies(address),
                api_owned.get_system_stations(address),
            );
            let result = match info {
                Ok(info) => Ok(SystemDetails { info, bodies: bodies.unwrap_or_default(), stations: stations.unwrap_or_default() }),
                Err(e) => Err(format!("{e:#}")),
            };
            *slot.lock().unwrap() = Some((address, result));
        });
    }

    pub fn handle_key(&mut self, key: &KeyEvent, journal: &JournalData) -> ViewEvent {
        match key.code {
            KeyCode::Char('c') => {
                self.center_on_commander(journal);
                ViewEvent::Consumed
            }
            KeyCode::Char('/') => {
                self.focus_search = true;
                ViewEvent::Consumed
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.touched();
                self.camera.zoom(0.7);
                ViewEvent::Consumed
            }
            KeyCode::Char('-') => {
                self.touched();
                self.camera.zoom(1.0 / 0.7);
                ViewEvent::Consumed
            }
            KeyCode::Char('g') => {
                self.fly(galaxy_view());
                ViewEvent::Consumed
            }
            // Held WASD moves the camera (see the desktop window's map input); a selected
            // system's bodies step with the arrow keys instead.
            KeyCode::Char('w' | 'a' | 's' | 'd' | 'W' | 'A' | 'S' | 'D') => ViewEvent::Consumed,
            _ if self.selected.is_some() => match self.system_map.handle_key(key) {
                ViewEvent::None if key.code == KeyCode::Esc => {
                    self.selected = None;
                    ViewEvent::Consumed
                }
                event => event,
            },
            _ => ViewEvent::None,
        }
    }
}

fn is_carrier(s: &StationSearchResult) -> bool {
    s.station_type.as_deref() == Some("FleetCarrier")
}
