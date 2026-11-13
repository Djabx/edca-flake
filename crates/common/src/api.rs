use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// GET /api/v1/client-version — the latest available client build number, so a
/// running client can tell whether it's outdated. Matches the pipeline that built
/// the server (and the client artifacts published alongside it).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ClientVersionResponse {
    pub build: String,
}

/// GET /api/v1/systems?name= — slim row for name search / autocomplete.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SystemSearchResult {
    pub system_address: i64,
    pub name: String,
    pub allegiance: Option<String>,
    pub population: Option<i64>,
}

/// GET /api/v1/systems/:address
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SystemResponse {
    pub system_address: i64,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub allegiance: Option<String>,
    pub economy: Option<String>,
    pub second_economy: Option<String>,
    pub government: Option<String>,
    pub security: Option<String>,
    pub population: Option<i64>,
    pub controlling_power: Option<String>,
    /// When this system's core data was last updated (its newest journal event).
    pub updated_at: Option<DateTime<Utc>>,
    pub factions: Vec<SystemFactionInfo>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SystemFactionInfo {
    pub name: String,
    pub influence: f32,
    pub government: Option<String>,
    pub allegiance: Option<String>,
    pub happiness: Option<String>,
    pub active_states: Vec<String>,
    pub pending_states: Vec<String>,
    pub recovering_states: Vec<String>,
}

/// GET /api/v1/systems/:address/bodies
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BodyResponse {
    pub id: i32,
    pub system_address: i64,
    pub name: String,
    pub is_star: bool,
    /// star_type or planet_class
    pub body_class: Option<String>,
    pub distance_from_arrival_ls: Option<f32>,
    pub radius: Option<f32>,
    pub mass_em: Option<f32>,
    pub surface_temperature: Option<f32>,
    pub surface_gravity: Option<f32>,
    pub landable: bool,
    pub atmosphere: Option<String>,
    pub volcanism: Option<String>,
    pub terraform_state: Option<String>,
    pub tidal_lock: bool,
    /// `WasDiscovered` / `WasMapped` / `WasFootfalled` of the newest reported scan.
    pub was_discovered: bool,
    pub was_mapped: bool,
    #[serde(default)]
    pub was_footfalled: Option<bool>,
    pub estimated_value: Option<i64>,
    pub rings: Vec<RingResponse>,
    pub materials: Vec<MaterialResponse>,
    pub parents: Vec<ParentResponse>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RingResponse {
    pub name: String,
    pub ring_class: String,
    pub mass_mt: f64,
    pub inner_rad: f64,
    pub outer_rad: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MaterialResponse {
    pub name: String,
    pub percent: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ParentResponse {
    pub parent_type: String,
    pub parent_id: i32,
}

/// GET /api/v1/stations — query params: name, system_name, market_id
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StationResponse {
    pub market_id: i64,
    pub system_address: i64,
    pub system_name: String,
    pub name: String,
    pub station_type: Option<String>,
    pub faction_name: Option<String>,
    pub government: Option<String>,
    pub economy: Option<String>,
    pub economies: Vec<StationEconomyResponse>,
    pub services: Vec<String>,
    pub landing_pads: Option<LandingPadsResponse>,
    pub dist_from_star_ls: Option<f32>,
    pub carrier_name: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub market_updated_at: Option<DateTime<Utc>>,
    pub commodities: Vec<CommodityResponse>,
    pub modules: Vec<ModuleResponse>,
    pub ships: Vec<ShipResponse>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommodityResponse {
    pub name: String,
    pub mean_price: i32,
    pub buy_price: i32,
    pub stock: i32,
    pub sell_price: i32,
    pub demand: i32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ModuleResponse {
    pub id: String,
    pub name: Option<String>,
    pub category: Option<String>,
    pub cost: i64,
    pub ship: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ShipResponse {
    pub id: String,
    pub name: Option<String>,
    pub basevalue: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StationEconomyResponse {
    pub name: String,
    pub proportion: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LandingPadsResponse {
    pub small: i32,
    pub medium: i32,
    pub large: i32,
}

/// GET /api/v1/carriers — query params: name, callsign
/// Fleet carriers are stored in the stations table (type = "FleetCarrier").
pub type CarrierResponse = StationResponse;

/// Query parameters for station/carrier search
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct StationQuery {
    pub name: Option<String>,
    pub system_name: Option<String>,
    pub market_id: Option<i64>,
    pub limit: Option<i64>,
}

/// Query parameters for carrier search
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct CarrierQuery {
    pub name: Option<String>,
    pub callsign: Option<String>,
    pub system_name: Option<String>,
    pub market_id: Option<i64>,
    pub limit: Option<i64>,
}

/// One entry per unique faction; presences lists every system the faction inhabits.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FactionResponse {
    pub name: String,
    pub government: Option<String>,
    pub allegiance: Option<String>,
    pub presences: Vec<FactionPresence>,
}

/// The faction's presence in one specific system.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FactionPresence {
    pub system_address: i64,
    pub system_name: String,
    pub influence: f32,
    pub happiness: Option<String>,
    pub active_states: Vec<String>,
    pub pending_states: Vec<String>,
    pub recovering_states: Vec<String>,
    pub conflict: Option<ConflictInfo>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Conflict data for war-like states (War, CivilWar, Election).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ConflictInfo {
    pub war_type: String,
    pub status: String,
    pub opponent_name: String,
    pub our_won_days: i32,
    pub opponent_won_days: i32,
    pub our_stake: Option<String>,
    pub opponent_stake: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct FactionQuery {
    pub name: Option<String>,
    pub limit: Option<i64>,
}

/// GET /api/v1/construction-depots — query params: name, system_name, market_id, limit
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ConstructionDepotResponse {
    pub market_id: i64,
    pub system_address: i64,
    pub system_name: String,
    pub station_name: String,
    pub progress: f32,
    pub construction_complete: bool,
    pub construction_failed: bool,
    /// `None` for a depot whose timestamp is unknown (e.g. a locally-synthesised
    /// entry); otherwise the last update time, like the other API timestamps.
    pub last_updated: Option<DateTime<Utc>>,
    pub resources: Vec<ConstructionResourceResponse>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ConstructionResourceResponse {
    pub name: String,
    pub display_name: String,
    pub required_amount: i32,
    pub provided_amount: i32,
    pub payment: i64,
}

/// Query parameters for construction depot search
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ConstructionQuery {
    pub name: Option<String>,
    pub system_name: Option<String>,
    pub market_id: Option<i64>,
    pub limit: Option<i64>,
}

/// GET /api/v1/trade-routes
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TradeRouteResponse {
    pub from_market_id: i64,
    pub to_market_id: i64,
    pub commodity: String,
    pub buy_price: i32,
    pub sell_price: i32,
    pub profit: i32,
    pub supply: i32,
    pub demand: i32,
    pub distance_ly: f32,
    pub from_station_name: String,
    pub to_station_name: String,
    pub from_system_name: String,
    pub to_system_name: String,
    pub from_max_pad: Option<String>,
    pub to_max_pad: Option<String>,
    pub from_allegiance: Option<String>,
    pub to_allegiance: Option<String>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// GET /api/v1/trade-loops
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct TradeLoopResponse {
    pub market_id_a: i64,
    pub market_id_b: i64,
    pub commodity_out: String,
    pub buy_price_out: i32,
    pub sell_price_out: i32,
    pub profit_out: i32,
    pub commodity_back: String,
    pub buy_price_back: i32,
    pub sell_price_back: i32,
    pub profit_back: i32,
    pub total_profit: i32,
    pub distance_ly: f32,
    pub station_name_a: String,
    pub station_name_b: String,
    pub system_name_a: String,
    pub system_name_b: String,
    pub max_pad: Option<String>,
    pub allegiance_a: Option<String>,
    pub allegiance_b: Option<String>,
    pub supply_out: i32,
    pub supply_back: i32,
    pub demand_out: i32,
    pub demand_back: i32,
    pub cached_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct TradeRouteQuery {
    pub pad_size: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct TradeLoopQuery {
    pub pad_size: Option<String>,
}

/// POST /api/v1/construction-depots — client submits depot data from journal
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ConstructionDepotSubmission {
    pub market_id: i64,
    pub system_address: i64,
    pub station_name: String,
    pub progress: f32,
    pub construction_complete: bool,
    pub construction_failed: bool,
    pub resources: Vec<ConstructionResourceSubmission>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ConstructionResourceSubmission {
    pub name: String,
    pub display_name: String,
    pub required_amount: i32,
    pub provided_amount: i32,
    pub payment: i64,
}

impl ConstructionDepotSubmission {
    /// Upper bound on resources in one depot. A real construction site lists a
    /// fixed, small set of commodities (a few dozen at most); this cap only exists
    /// to stop a crafted submission from spawning an unbounded serial INSERT loop.
    pub const MAX_RESOURCES: usize = 256;
    /// Bound on free-text field lengths, matching the journal validator's `MAX_STR`.
    const MAX_STR: usize = 255;

    /// Rejects out-of-range or oversized submissions before they reach the depot
    /// upsert. Mirrors the journal ingest path's `JournalEvent::validate`: a public
    /// write endpoint must not trust client-supplied sizes or numeric ranges.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.market_id <= 0 {
            return Err("market_id must be positive");
        }
        if self.system_address <= 0 {
            return Err("system_address must be positive");
        }
        if !self.progress.is_finite() || !(0.0..=1.0).contains(&self.progress) {
            return Err("progress out of range");
        }
        if self.station_name.len() > Self::MAX_STR {
            return Err("station_name too long");
        }
        if self.resources.len() > Self::MAX_RESOURCES {
            return Err("too many resources");
        }
        for res in &self.resources {
            if res.name.len() > Self::MAX_STR || res.display_name.len() > Self::MAX_STR {
                return Err("resource name too long");
            }
            if res.required_amount < 0 || res.provided_amount < 0 || res.payment < 0 {
                return Err("resource amounts must be non-negative");
            }
        }
        Ok(())
    }
}

/// One time-series point for GET /api/v1/system-population-history
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PopulationPoint {
    pub population: i64,
    pub timestamp: DateTime<Utc>,
}

/// One time-series point for GET /api/v1/faction-influence-history
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InfluencePoint {
    pub influence: f32,
    pub timestamp: DateTime<Utc>,
}

/// One influence sample tagged with its faction, for a whole system's history
/// (GET /api/v1/systems/<addr>/faction-influence-history) — every faction's
/// series in one response so the web page can plot them on one chart.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FactionInfluencePoint {
    pub faction_name: String,
    pub influence: f32,
    pub timestamp: DateTime<Utc>,
}

/// One time-series point for GET /api/v1/commodity-price-history
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommodityPricePoint {
    pub buy_price: i32,
    pub sell_price: i32,
    pub stock: i32,
    pub demand: i32,
    pub timestamp: DateTime<Utc>,
}

/// One label/count pair in a distribution (e.g. star type → number of stars).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CountEntry {
    pub label: String,
    pub count: i64,
}

/// GET /api/v1/stats — EDDB-style aggregate statistics about the whole dataset.
/// Computed in the background and served from an in-memory cache, so the values
/// are a recent snapshot rather than live. Counts for the largest append-only
/// tables (journal_events, body, star) are fast row-estimate approximations.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct DatabaseStatsResponse {
    // ── Totals ──────────────────────────────────────────────
    pub systems: i64,
    pub populated_systems: i64,
    pub total_population: i64,
    pub bodies: i64,
    pub stars: i64,
    pub landable_bodies: i64,
    /// Stations excluding fleet carriers.
    pub stations: i64,
    pub fleet_carriers: i64,
    pub factions: i64,
    /// Stations with at least one market commodity listing.
    pub markets: i64,
    pub journal_events: i64,
    /// Timestamp of the most recently ingested journal event, if any.
    pub newest_event: Option<DateTime<Utc>>,
    // ── Distributions (each ordered by count desc) ──────────
    pub star_types: Vec<CountEntry>,
    pub planet_classes: Vec<CountEntry>,
    pub allegiances: Vec<CountEntry>,
    pub economies: Vec<CountEntry>,
    pub governments: Vec<CountEntry>,
    pub security_levels: Vec<CountEntry>,
    pub station_types: Vec<CountEntry>,
}

/// One detected tick for a system. GET /api/v1/systems/:address/ticks returns these
/// newest-first — the most recent is this system's "last BGS update", and the full list
/// is the system's tick history (for charting its rhythm).
///
/// The galaxy tick is a single time; these are *this system's* updates, which the
/// staggered BGS rollout places at consistent but system-specific times. An empty list
/// means the system has too little recent traffic to bracket any tick (the UI shows
/// "Insufficient traffic data"). A wider `bracket_seconds` is lower confidence.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SystemTickResponse {
    pub system_address: i64,
    /// Last reading carrying the pre-tick influence values.
    pub tick_lower: DateTime<Utc>,
    /// First reading carrying the post-tick influence values. The tick fell in
    /// `(tick_lower, tick_upper]`.
    pub tick_upper: DateTime<Utc>,
    /// Bracket width in seconds — detection precision (smaller is tighter/better).
    pub bracket_seconds: i32,
    /// When the detector last computed this.
    pub detected_at: DateTime<Utc>,
}

/// GET /api/v1/systems/:address/tick-prediction — this system's predicted next tick.
///
/// Each system ticks at its own consistent time(s) of day (the staggered BGS rollout is
/// per-system, not one galaxy-wide instant). From the system's detected tick history we
/// derive a recurring time-of-day window and project the next occurrence: `next_tick` is
/// the middle of that window and `window_minutes` is the ± half-width. For systems that
/// tick twice a day this is the *next upcoming* window. `null` when there's too little
/// history to predict.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SystemTickPrediction {
    pub system_address: i64,
    /// Most recent actually-detected tick (first post-tick reading), if any.
    pub last_tick: Option<DateTime<Utc>>,
    /// Predicted centre of the next tick window (UTC).
    pub next_tick: DateTime<Utc>,
    /// ± half-width of the window, in minutes (the tick is expected in
    /// `[next_tick - window_minutes, next_tick + window_minutes]`).
    pub window_minutes: i32,
    /// Detected ticks supporting the predicted window (higher = more confident).
    pub samples: i32,
    /// Distinct daily tick slots detected for this system (1, or 2 for double-tickers).
    pub slots_per_day: i32,
}

/// POST /api/v1/nearest-multi-commodity
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct MultiCommodityQuery {
    pub commodities: Vec<String>,
    pub reference_system: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MultiCommodityResult {
    pub market_id: i64,
    pub system_name: String,
    pub station_name: String,
    pub station_type: Option<String>,
    pub distance_ly: f32,
    pub has_large_pad: bool,
    pub has_medium_pad: bool,
    pub matched_commodities: Vec<String>,
    pub matched_count: i64,
}

/// GET /api/v1/nearest-commodity?commodity=X&reference_system=Y&limit=N
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct NearestCommodityQuery {
    pub commodity: String,
    pub reference_system: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct NearestCommodityResult {
    pub market_id: i64,
    pub system_address: i64,
    pub system_name: String,
    pub station_name: String,
    pub station_type: Option<String>,
    pub distance_ly: f32,
    pub commodity_name: String,
    pub buy_price: i32,
    pub stock: i32,
    pub sell_price: i32,
    pub has_large_pad: bool,
    pub has_medium_pad: bool,
}

// ── Lightweight per-category search results (web tabs) ──────────────────────
// Single-representative-table rows, deliberately without the heavy per-result
// enrichment of the full station/faction endpoints (markets, presences, …).

/// GET /api/v1/stations/search?name= — slim station row (includes fleet carriers).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StationSearchResult {
    pub market_id: i64,
    pub system_address: i64,
    pub system_name: String,
    pub name: String,
    pub carrier_name: Option<String>,
    pub station_type: Option<String>,
    /// Light seconds from the arrival star, as last reported on docking.
    #[serde(default)]
    pub dist_from_star_ls: Option<f32>,
    pub updated_at: DateTime<Utc>,
}

/// GET /api/v1/factions/search?name= — slim faction row with its system count.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FactionSearchResult {
    pub name: String,
    pub allegiance: Option<String>,
    pub government: Option<String>,
    pub system_count: i64,
}

/// GET /api/v1/systems/suggest?q= — typeahead prefix match (minimal payload).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SystemSuggestion {
    pub system_address: i64,
    pub name: String,
}

/// GET /api/v1/stations/suggest?q= — typeahead prefix match (minimal payload).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StationSuggestion {
    pub market_id: i64,
    pub name: String,
    pub system_name: String,
}

// ── Cached "popular" / "top" lists (per-category default views) ─────────────

/// GET /api/v1/systems/popular — most-visited systems in the last 24h.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PopularSystem {
    pub system_address: i64,
    pub name: String,
    pub hits: i64,
}

/// GET /api/v1/stations/popular — most-updated stations in the last 24h.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PopularStation {
    pub market_id: i64,
    pub system_address: i64,
    pub system_name: String,
    pub name: String,
    pub station_type: Option<String>,
    pub hits: i64,
}

/// GET /api/v1/factions/top — factions present in the most systems.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct FactionRank {
    pub name: String,
    pub allegiance: Option<String>,
    pub system_count: i64,
}

/// GET /api/v1/outfitting/modules — distinct outfitting modules available across
/// markets (the picker list for the Outfitting tab). `name` is the game symbol.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OutfittingModuleInfo {
    pub name: String,
    pub category: Option<String>,
    pub market_count: i64,
}

/// GET /api/v1/nearest-outfitting — a station selling a given module, with its
/// distance from the reference system. Ordered nearest-first by the endpoint.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct NearestModuleResult {
    pub market_id: i64,
    pub system_address: i64,
    pub system_name: String,
    pub station_name: String,
    pub station_type: Option<String>,
    pub distance_ly: f32,
    pub module_name: String,
    pub category: Option<String>,
    pub cost: i64,
    /// Set when the module is ship-specific (otherwise applies to any ship).
    pub ship: Option<String>,
    pub has_large_pad: bool,
    pub has_medium_pad: bool,
}

/// GET /api/v1/ingest-stats — cumulative ingestion counters since server start,
/// for quickly checking feed health (received vs dispatched/skipped/rejected/errors).
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct IngestStats {
    pub eddn_received: u64,
    pub eddn_dispatched: u64,
    /// EDDN messages skipped before processing (schema not handled / unhandled event type).
    pub eddn_skipped: u64,
    /// Handled EDDN events dropped by validation.
    pub eddn_rejected: u64,
    pub eddn_errors: u64,
    pub client_received: u64,
    pub client_dispatched: u64,
    pub client_skipped: u64,
    pub client_rejected: u64,
    /// Client uploads refused by the per-IP rate limiter.
    pub client_rate_limited: u64,
    pub client_errors: u64,
}

/// GET /api/v1/request-stats — one aggregate row per (method, route, hashed IP).
/// `ip_hash` is a salted HMAC pseudonym (same as journal_events.uploader_ip_hash),
/// never a raw IP. Operational telemetry; keep internal.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ApiRequestStat {
    pub method: String,
    pub endpoint: String,
    pub ip_hash: String,
    pub hits: i64,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
}

/// GET /api/v1/commodities — one row per commodity, aggregated galaxy-wide from
/// `commodity_listening` (refreshed in the 15-min category-stats cache). The set
/// is small (~350 commodities), so it's returned whole and filtered client-side.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommodityStat {
    pub name: String,
    /// Number of markets currently listing the commodity (the "most traded" rank).
    pub market_count: i64,
    pub total_stock: i64,
    pub total_demand: i64,
    /// Galactic reference (mean) price — representative; averaged across markets.
    pub mean_price: i32,
    /// Cheapest buy price across markets that have it in stock (0 if none).
    /// Fleet carriers and zero-bracket listings are excluded — their prices are
    /// player-set or not really tradeable, and would otherwise dominate the
    /// galaxy-wide extreme. Comes from the same row as `best_buy_*`.
    pub min_buy_price: i32,
    /// Highest sell price across markets that demand it (0 if none), filtered the
    /// same way as `min_buy_price` and taken from the same row as `best_sell_*`.
    pub max_sell_price: i32,
    /// The market offering `min_buy_price`: its station/system, and the supply there.
    pub best_buy_market_id: Option<i64>,
    pub best_buy_station: Option<String>,
    pub best_buy_system: Option<String>,
    pub best_buy_stock: i32,
    /// The market offering `max_sell_price`: its station/system, and the demand there.
    pub best_sell_market_id: Option<i64>,
    pub best_sell_station: Option<String>,
    pub best_sell_system: Option<String>,
    pub best_sell_demand: i32,
}

/// One market's listing for a single commodity (GET /api/v1/commodities/listings).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommodityMarketListing {
    pub market_id: i64,
    pub system_address: i64,
    pub system_name: String,
    pub station_name: String,
    pub station_type: Option<String>,
    pub buy_price: i32,
    pub sell_price: i32,
    pub stock: i32,
    pub demand: i32,
    pub has_large_pad: bool,
    pub has_medium_pad: bool,
    pub updated_at: Option<DateTime<Utc>>,
}

/// GET /api/v1/commodities/listings?name= — the best places to buy (cheapest) and
/// sell (dearest) one commodity, galaxy-wide.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CommodityListingsResponse {
    pub buys: Vec<CommodityMarketListing>,
    pub sells: Vec<CommodityMarketListing>,
}

// ── Galaxy map ───────────────────────────────────────────────────────────────

/// GET /api/v1/galaxy/density — every known system counted into cubic cells of
/// `cell_ly` light years, for drawing the shape of the charted galaxy without
/// shipping millions of points. Only non-empty cells are listed.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct GalaxyDensity {
    /// Edge length of one cell in light years.
    pub cell_ly: f32,
    /// `[ix, iy, iz, systems]`: the cell spans `ix * cell_ly .. (ix + 1) * cell_ly` on x
    /// (likewise y and z) and holds `systems` known systems. Arrays rather than objects
    /// keep the ~80k cells to about a megabyte of JSON.
    pub cells: Vec<[i32; 4]>,
    /// Number of known systems over all cells.
    pub total_systems: i64,
}

/// GET /api/v1/galaxy/systems?x=&y=&z=&radius=&limit= — known systems within
/// `radius` ly of a point, nearest first.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GalaxySystem {
    pub system_address: i64,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// 0 when unpopulated or unknown.
    pub population: i64,
}

/// GET /api/v1/galaxy/sample?x=&y=&z=&radius=&limit= — an even sample of the known
/// systems within `radius` ly of a point, for the middle zoom range where there are too
/// many systems to send them all. The same systems are picked on every call (by a hash of
/// the address), so the sample doesn't flicker as the region moves.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct GalaxySample {
    /// Known systems in the sphere; each point stands for `total / points.len()` of them.
    pub total: i64,
    /// `[x, y, z, populated]`, populated being 1 or 0.
    pub points: Vec<[f32; 4]>,
}

/// GET /api/v1/codex — every codex entry commanders logged, with how often and where.
/// Served from a cache refreshed every few hours.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct CodexResponse {
    pub entries: Vec<CodexEntryStat>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// GET /api/v1/codex/entry/<key> — every system where commanders logged one codex
/// entry. Served from the codex cache.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct CodexEntryResponse {
    /// The entry's statistics, `None` when no one logged it.
    pub entry: Option<CodexEntryStat>,
    pub systems: Vec<CodexEntrySystem>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// A system where a codex entry was logged.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CodexEntrySystem {
    pub system_address: i64,
    pub name: String,
    /// Galactic coordinates (ly), when a find carried them.
    pub coords: Option<[f32; 3]>,
    /// The bodies it was logged on or near (none for finds away from any body).
    pub bodies: Vec<CodexEntryBody>,
    pub finds: i64,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
}

/// A body a codex entry was logged on or near.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CodexEntryBody {
    pub body_id: Option<i32>,
    /// Its name, or "Body 7" when no scan of it named it.
    pub name: String,
    /// Logged on its surface; otherwise in space near it.
    pub surface: bool,
    /// Where on the surface the first find was logged.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// GET /api/v1/codex/guardian — every system with a Guardian site, beacon or surface
/// signal commanders reported. Served from a cache refreshed every few hours.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct GuardianResponse {
    pub systems: Vec<GuardianSystem>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// A system with something of the Guardians in it.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct GuardianSystem {
    pub system_address: i64,
    pub name: String,
    /// Galactic coordinates (ly), when the system's position is known.
    pub coords: Option<[f32; 3]>,
    pub bodies: Vec<SiteBody>,
    /// A Guardian Beacon was found in the system.
    pub beacon: bool,
    /// Guardian codex entries logged here (`$Codex_Ent_Guardian_Terminal_Name;` …), with finds.
    pub codex: Vec<CountEntry>,
    /// The newest report of any of it.
    pub last_seen: Option<DateTime<Utc>>,
}

/// A body with Guardian or Thargoid signals or sites.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SiteBody {
    pub body_id: Option<i32>,
    pub name: String,
    /// Signals of the kind the list is about that the detailed surface scan counted on it.
    pub signals: Option<i32>,
    /// The sites commanders flew to on it.
    pub sites: Vec<SurfaceSite>,
}

/// One site on a body.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SurfaceSite {
    /// The settlement's symbol without its index: `$Ancient` (Guardian ruins),
    /// `$Ancient_Small_005` (a Guardian structure — see [`crate::codex::guardian`]) or
    /// `$Settlement_Unflattened_TGMegaBarnacle` (see [`crate::codex::thargoid`]).
    pub kind: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// GET /api/v1/codex/thargoid — every system with Thargoids in it: a war state, surface
/// signals, barnacle or crash sites, Thargoid conflict zones, non-human signal sources
/// or Thargoid codex finds. Served from a cache refreshed every few hours.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ThargoidResponse {
    pub systems: Vec<ThargoidSystem>,
    pub cached_at: Option<DateTime<Utc>>,
}

/// A system with something of the Thargoids in it.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ThargoidSystem {
    pub system_address: i64,
    pub name: String,
    pub coords: Option<[f32; 3]>,
    /// The system's war, as the newest jump that reported one saw it (`None` when none did).
    pub war: Option<ThargoidWar>,
    pub bodies: Vec<SiteBody>,
    /// Thargoid conflict zones found, by intensity (`Low`, `Med`, `High`, `VeryHigh`).
    pub conflict_zones: Vec<CountEntry>,
    /// Non-human signal sources reported.
    pub nonhuman_signals: i64,
    /// Thargoid codex entries logged here, with finds.
    pub codex: Vec<CountEntry>,
    pub last_seen: Option<DateTime<Utc>>,
}

/// The Thargoid war in a system, from the journal's `ThargoidWar` block.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ThargoidWar {
    /// `Thargoid_Controlled`, `Thargoid_Stronghold`, `Thargoid_Harvest`, `Thargoid_Probing`
    /// or `Thargoid_Recovery`.
    pub state: String,
    /// Share of the way to the next state (0..1).
    pub progress: f64,
    pub next_success: Option<String>,
    pub next_failure: Option<String>,
    pub remaining_ports: Option<i64>,
    pub success_reached: bool,
    pub reported_at: Option<DateTime<Utc>>,
    /// Newer reports of the system carry no war: this one is history.
    #[serde(default)]
    pub over: bool,
}

/// One codex entry (a species' colour variant, a geological feature, a Thargoid or
/// Guardian site…), as the journal names it.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CodexEntryStat {
    /// Codex symbol, e.g. `$Codex_Ent_Stratum_07_K_Name;`.
    pub name: String,
    /// e.g. `$Codex_Category_Biology;`.
    pub category: String,
    /// e.g. `$Codex_SubCategory_Organic_Structures;`.
    pub subcategory: String,
    /// Codex entries logged (each commander logs a find once per region).
    pub finds: i64,
    pub systems: i64,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
    /// Finds per galactic region (`$Codex_RegionName_18;`), most first.
    pub regions: Vec<CountEntry>,
}
