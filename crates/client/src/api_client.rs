use edcas_common::api::{
    BodyResponse, CarrierQuery, CarrierResponse, ClientVersionResponse, FactionInfluencePoint, SystemTickPrediction, SystemTickResponse, CommodityPricePoint, ConstructionDepotResponse,
    ConstructionDepotSubmission, ConstructionQuery, FactionQuery, FactionResponse, GalaxyDensity,
    GalaxySample, GalaxySystem, InfluencePoint, MultiCommodityQuery, MultiCommodityResult, NearestCommodityQuery,
    NearestCommodityResult, StationQuery, StationResponse, StationSearchResult, SystemResponse,
    SystemSuggestion, TradeLoopResponse, TradeRouteResponse, FactionRank, FactionSearchResult,
    NearestModuleResult, OutfittingModuleInfo, PopularStation, PopularSystem, StationSuggestion,
    SystemSearchResult,
};

// ─── Native (async) implementation ────────────────────────────────────────────

use tracing::{debug, warn};

#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    client: reqwest::Client,
    rt: tokio::runtime::Handle,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>, rt: tokio::runtime::Handle) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build HTTP client"),
            rt,
        }
    }

    /// Spawn an async task on the background runtime. Results should be
    /// communicated back via channels or shared state — do not block the UI.
    pub fn spawn<F>(&self, f: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        // Whatever the task delivers, the UI needs a frame to pick it up.
        self.rt.spawn(async move {
            f.await;
            crate::wake::wake();
        });
    }

    /// Returns a clone of the underlying async HTTP client.
    pub fn http_client(&self) -> reqwest::Client {
        self.client.clone()
    }

    /// The latest available client build number, per the server. Used to flag a
    /// running client as outdated when it differs from `build_info::BUILD`.
    pub async fn get_latest_build(&self) -> anyhow::Result<String> {
        let url = format!("{}/api/v1/client-version", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if status.is_success() {
            let v: ClientVersionResponse = resp.json().await?;
            debug!(url, build = %v.build, "API response: get_latest_build");
            Ok(v.build)
        } else {
            warn!(url, %status, "API response: get_latest_build failed");
            anyhow::bail!("client-version request failed: {status}")
        }
    }

    pub async fn get_bodies(&self, system_address: i64) -> anyhow::Result<Vec<BodyResponse>> {
        let url = format!("{}/api/v1/systems/{}/bodies", self.base_url, system_address);
        debug!(url, system_address, "API call: get_bodies");
        let resp = self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() {
            let result: Vec<BodyResponse> = resp.json().await?;
            debug!(url, count = result.len(), "API response: get_bodies");
            Ok(result)
        } else {
            warn!(url, %status, "API response: get_bodies failed");
            Ok(vec![])
        }
    }

    pub async fn search_stations(&self, query: &StationQuery) -> anyhow::Result<Vec<StationResponse>> {
        let url = format!("{}/api/v1/stations", self.base_url);
        debug!(url, ?query, "API call: search_stations");
        let resp = self.client.get(&url).query(query).send().await?;
        let status = resp.status();
        if status.is_success() {
            let result: Vec<StationResponse> = resp.json().await?;
            debug!(url, count = result.len(), "API response: search_stations");
            Ok(result)
        } else {
            warn!(url, %status, "API response: search_stations failed");
            Ok(vec![])
        }
    }

    pub async fn search_factions(&self, query: &FactionQuery) -> anyhow::Result<Vec<FactionResponse>> {
        let url = format!("{}/api/v1/factions", self.base_url);
        debug!(url, ?query, "API call: search_factions");
        let resp = self.client.get(&url).query(query).send().await?;
        let status = resp.status();
        if status.is_success() {
            let result: Vec<FactionResponse> = resp.json().await?;
            debug!(url, count = result.len(), "API response: search_factions");
            Ok(result)
        } else {
            warn!(url, %status, "API response: search_factions failed");
            Ok(vec![])
        }
    }

    pub async fn search_construction_depots(
        &self,
        query: &ConstructionQuery,
    ) -> anyhow::Result<Vec<ConstructionDepotResponse>> {
        let url = format!("{}/api/v1/construction-depots", self.base_url);
        debug!(url, ?query, "API call: search_construction_depots");
        let resp = self.client.get(&url).query(query).send().await?;
        let status = resp.status();
        if status.is_success() {
            let result: Vec<ConstructionDepotResponse> = resp.json().await?;
            debug!(url, count = result.len(), "API response: search_construction_depots");
            Ok(result)
        } else {
            warn!(url, %status, "API response: search_construction_depots failed");
            Ok(vec![])
        }
    }

    pub async fn submit_construction_depot(
        &self,
        submission: &ConstructionDepotSubmission,
    ) -> anyhow::Result<()> {
        let url = format!("{}/api/v1/construction-depots", self.base_url);
        debug!(url, ?submission, "API call: submit_construction_depot");
        let resp = self.client.post(&url).json(submission).send().await?;
        let status = resp.status();
        if status.is_success() {
            debug!(url, "API response: submit_construction_depot ok");
        } else {
            warn!(url, %status, "API response: submit_construction_depot failed");
        }
        Ok(())
    }

    pub async fn fetch_trade_routes(&self) -> anyhow::Result<Vec<TradeRouteResponse>> {
        let url = format!("{}/api/v1/trade-routes", self.base_url);
        debug!(url, "API call: fetch_trade_routes");
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if status.is_success() {
            let result: Vec<TradeRouteResponse> = resp.json().await?;
            debug!(url, count = result.len(), "API response: fetch_trade_routes");
            Ok(result)
        } else {
            warn!(url, %status, "API response: fetch_trade_routes failed");
            Ok(vec![])
        }
    }

    pub async fn fetch_trade_loops(&self) -> anyhow::Result<Vec<TradeLoopResponse>> {
        let url = format!("{}/api/v1/trade-loops", self.base_url);
        debug!(url, "API call: fetch_trade_loops");
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if status.is_success() {
            let result: Vec<TradeLoopResponse> = resp.json().await?;
            debug!(url, count = result.len(), "API response: fetch_trade_loops");
            Ok(result)
        } else {
            warn!(url, %status, "API response: fetch_trade_loops failed");
            Ok(vec![])
        }
    }

    pub async fn search_nearest_commodity(
        &self,
        query: &NearestCommodityQuery,
    ) -> anyhow::Result<Vec<NearestCommodityResult>> {
        let url = format!("{}/api/v1/nearest-commodity", self.base_url);
        debug!(url, ?query, "API call: search_nearest_commodity");
        let resp = self.client.get(&url).query(query).send().await?;
        let status = resp.status();
        if status.is_success() {
            let result: Vec<NearestCommodityResult> = resp.json().await?;
            debug!(url, count = result.len(), "API response: search_nearest_commodity");
            Ok(result)
        } else {
            warn!(url, %status, "API response: search_nearest_commodity failed");
            Ok(vec![])
        }
    }

    pub async fn search_nearest_multi_commodity(
        &self,
        query: &MultiCommodityQuery,
    ) -> anyhow::Result<Vec<MultiCommodityResult>> {
        let url = format!("{}/api/v1/nearest-multi-commodity", self.base_url);
        debug!(url, "API call: search_nearest_multi_commodity ({} commodities)", query.commodities.len());
        let resp = self.client.post(&url).json(query).send().await?;
        let status = resp.status();
        if status.is_success() {
            let result: Vec<MultiCommodityResult> = resp.json().await?;
            debug!(url, count = result.len(), "API response: search_nearest_multi_commodity");
            Ok(result)
        } else {
            warn!(url, %status, "API response: search_nearest_multi_commodity failed");
            Ok(vec![])
        }
    }

    pub async fn fetch_faction_influence_history(
        &self,
        name: &str,
        system_address: i64,
        days: u32,
    ) -> anyhow::Result<Vec<InfluencePoint>> {
        let url = format!("{}/api/v1/faction-influence-history", self.base_url);
        debug!(url, name, system_address, days, "API call: fetch_faction_influence_history");
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("name", name.to_string()),
                ("system_address", system_address.to_string()),
                ("days", days.to_string()),
            ])
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() {
            let result: Vec<InfluencePoint> = resp.json().await?;
            debug!(url, count = result.len(), "API response: fetch_faction_influence_history");
            Ok(result)
        } else {
            warn!(url, %status, "API response: fetch_faction_influence_history failed");
            Ok(vec![])
        }
    }

    pub async fn fetch_commodity_price_history(
        &self,
        market_id: i64,
        commodity: &str,
        days: u32,
    ) -> anyhow::Result<Vec<CommodityPricePoint>> {
        let url = format!("{}/api/v1/commodity-price-history", self.base_url);
        debug!(url, market_id, commodity, days, "API call: fetch_commodity_price_history");
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("market_id", market_id.to_string()),
                ("commodity", commodity.to_string()),
                ("days", days.to_string()),
            ])
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() {
            let result: Vec<CommodityPricePoint> = resp.json().await?;
            debug!(url, count = result.len(), "API response: fetch_commodity_price_history");
            Ok(result)
        } else {
            warn!(url, %status, "API response: fetch_commodity_price_history failed");
            Ok(vec![])
        }
    }

    /// Every known system counted into coarse cells, for the galaxy map's overview.
    pub async fn fetch_galaxy_density(&self) -> anyhow::Result<GalaxyDensity> {
        let url = format!("{}/api/v1/galaxy/density", self.base_url);
        debug!(url, "API call: fetch_galaxy_density");
        let resp = self.client.get(&url).timeout(std::time::Duration::from_secs(60)).send().await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: fetch_galaxy_density failed");
            anyhow::bail!("galaxy density request failed: {status}");
        }
        let result: GalaxyDensity = resp.json().await?;
        debug!(url, cells = result.cells.len(), "API response: fetch_galaxy_density");
        Ok(result)
    }

    /// Known systems within `radius` ly of `pos`, nearest first.
    pub async fn fetch_galaxy_systems(&self, pos: [f32; 3], radius: f32, limit: u32) -> anyhow::Result<Vec<GalaxySystem>> {
        let url = format!("{}/api/v1/galaxy/systems", self.base_url);
        debug!(url, ?pos, radius, limit, "API call: fetch_galaxy_systems");
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("x", pos[0].to_string()),
                ("y", pos[1].to_string()),
                ("z", pos[2].to_string()),
                ("radius", radius.to_string()),
                ("limit", limit.to_string()),
            ])
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: fetch_galaxy_systems failed");
            anyhow::bail!("galaxy systems request failed: {status}");
        }
        let result: Vec<GalaxySystem> = resp.json().await?;
        debug!(url, count = result.len(), "API response: fetch_galaxy_systems");
        Ok(result)
    }

    /// An even sample of the known systems within `radius` ly of `pos`.
    pub async fn fetch_galaxy_sample(&self, pos: [f32; 3], radius: f32, limit: u32) -> anyhow::Result<GalaxySample> {
        let url = format!("{}/api/v1/galaxy/sample", self.base_url);
        debug!(url, ?pos, radius, limit, "API call: fetch_galaxy_sample");
        let resp = self
            .client
            .get(&url)
            .query(&[
                ("x", pos[0].to_string()),
                ("y", pos[1].to_string()),
                ("z", pos[2].to_string()),
                ("radius", radius.to_string()),
                ("limit", limit.to_string()),
            ])
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: fetch_galaxy_sample failed");
            anyhow::bail!("galaxy sample request failed: {status}");
        }
        let result: GalaxySample = resp.json().await?;
        debug!(url, points = result.points.len(), total = result.total, "API response: fetch_galaxy_sample");
        Ok(result)
    }

    /// Systems whose name starts with `q` (typeahead).
    pub async fn suggest_systems(&self, q: &str, limit: u32) -> anyhow::Result<Vec<SystemSuggestion>> {
        let url = format!("{}/api/v1/systems/suggest", self.base_url);
        debug!(url, q, "API call: suggest_systems");
        let resp = self.client.get(&url).query(&[("q", q.to_string()), ("limit", limit.to_string())]).send().await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: suggest_systems failed");
            anyhow::bail!("system suggestions failed: {status}");
        }
        Ok(resp.json().await?)
    }

    /// One system's coordinates, state and factions.
    pub async fn get_system(&self, system_address: i64) -> anyhow::Result<SystemResponse> {
        let url = format!("{}/api/v1/systems/{}", self.base_url, system_address);
        debug!(url, "API call: get_system");
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: get_system failed");
            anyhow::bail!("system request failed: {status}");
        }
        Ok(resp.json().await?)
    }

    /// Every faction's influence history in a system over the last `days`, ordered by faction
    /// then time.
    pub async fn get_system_influence_history(&self, system_address: i64, days: u32) -> anyhow::Result<Vec<FactionInfluencePoint>> {
        let url = format!("{}/api/v1/systems/{}/faction-influence-history", self.base_url, system_address);
        debug!(url, days, "API call: get_system_influence_history");
        let resp = self.client.get(&url).query(&[("days", days)]).send().await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: get_system_influence_history failed");
            anyhow::bail!("influence history request failed: {status}");
        }
        Ok(resp.json().await?)
    }

    /// A system's predicted next BGS tick; `None` while it has too little tick history.
    pub async fn get_tick_prediction(&self, system_address: i64) -> anyhow::Result<Option<SystemTickPrediction>> {
        let url = format!("{}/api/v1/systems/{}/tick-prediction", self.base_url, system_address);
        debug!(url, "API call: get_tick_prediction");
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: get_tick_prediction failed");
            anyhow::bail!("tick prediction request failed: {status}");
        }
        Ok(resp.json().await?)
    }

    /// A system's detected BGS ticks, newest first.
    pub async fn get_system_ticks(&self, system_address: i64) -> anyhow::Result<Vec<SystemTickResponse>> {
        let url = format!("{}/api/v1/systems/{}/ticks", self.base_url, system_address);
        debug!(url, "API call: get_system_ticks");
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: get_system_ticks failed");
            anyhow::bail!("system ticks request failed: {status}");
        }
        Ok(resp.json().await?)
    }

    /// The stations (and fleet carriers) last seen in a system.
    pub async fn get_system_stations(&self, system_address: i64) -> anyhow::Result<Vec<StationSearchResult>> {
        let url = format!("{}/api/v1/systems/{}/stations", self.base_url, system_address);
        debug!(url, "API call: get_system_stations");
        let resp = self.client.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: get_system_stations failed");
            anyhow::bail!("system stations request failed: {status}");
        }
        Ok(resp.json().await?)
    }

    /// Fleet carriers whose callsign or name contains `query.name` (or the one with
    /// `query.market_id`), with their market, outfitting and shipyard.
    pub async fn search_carriers(&self, query: &CarrierQuery) -> anyhow::Result<Vec<CarrierResponse>> {
        let url = format!("{}/api/v1/carriers", self.base_url);
        debug!(url, ?query, "API call: search_carriers");
        let resp = self.client.get(&url).query(query).send().await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: search_carriers failed");
            anyhow::bail!("carrier search failed: {status}");
        }
        Ok(resp.json().await?)
    }

    // ── Search tab: the light per-category lookups the website's search uses ──

    /// GETs `path` with `query` and reads the JSON reply; `what` names it in errors.
    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str, query: &[(&str, String)], what: &str) -> anyhow::Result<T> {
        let url = format!("{}{path}", self.base_url);
        debug!(url, ?query, "API call: {what}");
        let resp = self.client.get(&url).query(query).send().await?;
        let status = resp.status();
        if !status.is_success() {
            warn!(url, %status, "API response: {what} failed");
            anyhow::bail!("{what} failed: {status}");
        }
        Ok(resp.json().await?)
    }

    /// Systems whose name contains `q`.
    pub async fn search_systems(&self, q: &str, limit: u32) -> anyhow::Result<Vec<SystemSearchResult>> {
        self.get_json("/api/v1/systems", &[("name", q.to_string()), ("limit", limit.to_string())], "system search").await
    }

    /// Stations and carriers whose name starts with `q` (typeahead).
    pub async fn suggest_stations(&self, q: &str, limit: u32) -> anyhow::Result<Vec<StationSuggestion>> {
        self.get_json("/api/v1/stations/suggest", &[("q", q.to_string()), ("limit", limit.to_string())], "station suggestions").await
    }

    /// Factions whose name contains `q`, without their presences.
    pub async fn find_factions(&self, q: &str, limit: u32) -> anyhow::Result<Vec<FactionSearchResult>> {
        self.get_json("/api/v1/factions/search", &[("name", q.to_string()), ("limit", limit.to_string())], "faction search").await
    }

    /// The systems commanders visit most.
    pub async fn popular_systems(&self) -> anyhow::Result<Vec<PopularSystem>> {
        self.get_json("/api/v1/systems/popular", &[], "popular systems").await
    }

    /// The stations commanders dock at most.
    pub async fn popular_stations(&self) -> anyhow::Result<Vec<PopularStation>> {
        self.get_json("/api/v1/stations/popular", &[], "popular stations").await
    }

    /// The factions present in the most systems.
    pub async fn top_factions(&self) -> anyhow::Result<Vec<FactionRank>> {
        self.get_json("/api/v1/factions/top", &[], "top factions").await
    }

    /// Every module some station sells, with how many stations do.
    pub async fn outfitting_modules(&self) -> anyhow::Result<Vec<OutfittingModuleInfo>> {
        self.get_json("/api/v1/outfitting/modules", &[], "outfitting modules").await
    }

    /// The stations nearest `reference_system` that sell `module`.
    pub async fn nearest_outfitting(&self, module: &str, reference_system: &str, limit: u32) -> anyhow::Result<Vec<NearestModuleResult>> {
        let query = [("module", module.to_string()), ("reference_system", reference_system.to_string()), ("limit", limit.to_string())];
        self.get_json("/api/v1/nearest-outfitting", &query, "nearest outfitting").await
    }
}
