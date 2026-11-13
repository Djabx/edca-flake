mod gui;

use crate::api_client::ApiClient;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::views::ViewEvent;
use tracing::warn;
use edcas_common::api::{MultiCommodityQuery, MultiCommodityResult, NearestCommodityQuery, NearestCommodityResult};

use std::sync::{Arc, Mutex};

const MAX_SUGGESTIONS: usize = 8;

/// Every commodity's name as the game shows it, for picking one by name.
pub fn commodity_names() -> &'static [&'static str] {
    COMMODITY_NAMES
}

static COMMODITY_NAMES: &[&str] = &[
    "Advanced Catalysers",
    "Advanced Medicines",
    "Agri-Medicines",
    "Ai Relics",
    "Alexandrite",
    "Algae",
    "Aluminium",
    "Animal Meat",
    "Animal Monitors",
    "Anomalous Bulk Scan Data",
    "Antique Jewellery",
    "Antiquities",
    "Aquaponic Systems",
    "Assault Plans",
    "Atmospheric Processors",
    "Basic Medicines",
    "Battle Weapons",
    "Bauxite",
    "Beer",
    "Benitoite",
    "Beryllium",
    "Bertrandite",
    "Bioreducing Lichen",
    "Biowaste",
    "Bismuth",
    "Black Box",
    "Bootleg Liquor",
    "Bromellite",
    "Building Fabricators",
    "Ceramic Composites",
    "Chemical Waste",
    "Classified Experimental Equipment",
    "Clothing",
    "CMM Composite",
    "Cobalt",
    "Coffee",
    "Coltan",
    "Combat Stabilisers",
    "Commercial Samples",
    "Computer Components",
    "Conductive Fabrics",
    "Consumer Technology",
    "Copper",
    "Crop Harvesters",
    "Cryolite",
    "Damaged Escape Pod",
    "Data Core",
    "Diplomatic Bag",
    "Domestic Appliances",
    "Emergency Power Cells",
    "Encrypted Correspondence",
    "Encrypted Data Storage",
    "Energy Grid Assembly",
    "Engineering Technical Drawings",
    "Evacuation Shelter",
    "Exhaust Manifold",
    "Experimental Chemicals",
    "Explosives",
    "Fish",
    "Food Cartridges",
    "Fossil Remnants",
    "Fruit And Vegetables",
    "Gallite",
    "Gallium",
    "Geological Equipment",
    "Gold",
    "Goslarite",
    "Grain",
    "Grand Prismatic Sapphire",
    "Hafnium 178",
    "Hazardous Environment Suits",
    "Heatsink Interlink",
    "HN Shock Mount",
    "Hydrogen Fuels",
    "Hydrogen Peroxide",
    "Imperial Slaves",
    "Improvised Components",
    "Indite",
    "Indium",
    "Industrial Firmware",
    "Insulating Membrane",
    "Intact Technology Package",
    "Ion Distributor",
    "Jadeite",
    "Kernite",
    "Land Enrichment Systems",
    "Landmines",
    "Large Survey Data Cache",
    "Lanthanum",
    "Leaked Installation Data",
    "Leather",
    "Lepidolite",
    "Liquid Oxygen",
    "Liquor",
    "Lithium",
    "Low Temperature Diamonds",
    "Magnetic Emitter Coil",
    "Marine Equipment",
    "Medical Diagnostic Equipment",
    "Methane Clathrate",
    "Methanol Monohydrate Crystals",
    "Microbial Furnaces",
    "Military Grade Fabrics",
    "Military Intelligence",
    "Military Plans",
    "Mineral Extractors",
    "Mineral Oil",
    "Modular Terminals",
    "Moissanite",
    "Monazite",
    "Muon Imager",
    "Musgravite",
    "Mysterious Idol",
    "Nanobreakers",
    "Narcotics",
    "Natural Fabrics",
    "Nerve Agents",
    "Non-Lethal Weapons",
    "Occupied Cryopod",
    "Osmium",
    "Painite",
    "Palladium",
    "Performance Enhancers",
    "Personal Effects",
    "Personal Weapons",
    "Pesticides",
    "Platinum",
    "Pod Core Tissue",
    "Pod Dead Tissue",
    "Pod Mesoglea",
    "Pod Outer Tissue",
    "Pod Shell Tissue",
    "Pod Surface Tissue",
    "Pod Tissue Sample",
    "Polymers",
    "Power Converter",
    "Power Generators",
    "Power Transfer Bus",
    "Praseodymium",
    "Precious Gems",
    "Progenitor Cells",
    "Prohibited Research Materials",
    "Prototype Tech",
    "Pyrophyllite",
    "Radiation Baffle",
    "Rare Artwork",
    "Reactive Armour",
    "Rebel Transmissions",
    "Reinforced Mounting Plate",
    "Resonating Separators",
    "Rhodplumsite",
    "Robotics",
    "Rutile",
    "Salvageable Wreckage",
    "Samarium",
    "Sap 8 Core Container",
    "Scientific Research",
    "Scientific Samples",
    "Scrap",
    "Semiconductors",
    "Serendibite",
    "Silver",
    "Skimmer Components",
    "Slaves",
    "Space Pioneer Relics",
    "Stimulants",
    "Structural Regulators",
    "Superconductors",
    "Surface Stabilisers",
    "Survival Equipment",
    "Synthetic Fabrics",
    "Synthetic Meat",
    "Synthetic Reagents",
    "Taaffeite",
    "Tactical Data",
    "Tantalum",
    "Tea",
    "Technical Blueprints",
    "Telemetry Suite",
    "Thallium",
    "Thermal Cooling Units",
    "Thorium",
    "Titanium",
    "Tobacco",
    "Toxic Waste",
    "Trade Data",
    "Tritium",
    "Unknown Artefact",
    "Unstable Data Core",
    "Uraninite",
    "Uranium",
    "Void Opals",
    "Water Purifiers",
    "Wine",
    "Wreckage Components",
];

/// Resolve a raw journal commodity key (e.g. `"$buildingfabricators_name;"`) to the
/// proper canonical name used by the API (e.g. `"Building Fabricators"`).
/// Strips `$`, `;`, `_Name`, `_name`, then matches against the known commodity list
/// by normalizing both sides to lowercase alphanumeric.
pub(super) fn resolve_commodity_canonical(raw: &str) -> String {
    let base = raw
        .trim_start_matches('$')
        .trim_end_matches(';')
        .trim_end_matches("_Name")
        .trim_end_matches("_name");
    let normalized = base.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    COMMODITY_NAMES
        .iter()
        .find(|&&name| {
            let n = name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
            n == normalized
        })
        .map(|&s| s.to_string())
        .unwrap_or_else(|| base.to_string())
}

/// Split `(display_name, raw journal key)` pairs into the localised names shown to
/// the user and the canonical names the multi-commodity search matches on. The
/// server compares names case-, space- and hyphen-insensitively against the stored
/// market symbol, so a localised name (e.g. German `"Computerteile"`) never matches.
pub(super) fn multi_search_names<'a>(
    resources: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> (Vec<String>, Vec<String>) {
    resources
        .into_iter()
        .map(|(display, raw)| (display.to_string(), resolve_commodity_canonical(raw)))
        .unzip()
}

/// Map a commodity name returned by the server (a lowercase market symbol such as
/// `"ceramiccomposites"`) back to the display name it was searched under.
fn display_for_matched(matched: &str, canonical: &[String], display: &[String]) -> String {
    let key = SearchNearestView::normalize_for_match(matched);
    canonical
        .iter()
        .position(|c| SearchNearestView::normalize_for_match(c) == key)
        .and_then(|i| display.get(i))
        .cloned()
        .unwrap_or_else(|| matched.to_string())
}

#[derive(Clone, Copy, PartialEq)]
enum ActiveField {
    Commodity,
    System,
}

pub struct SearchNearestView {
    commodity_input: String,
    /// Canonical English name used for the API query (may differ from localized commodity_input)
    commodity_canonical: String,
    system_input: String,
    active_field: ActiveField,
    editing: bool,
    results: Vec<NearestCommodityResult>,
    selected_idx: usize,
    scroll: usize,
    status_msg: String,
    is_loading: bool,
    suggestions: Vec<&'static str>,
    suggestion_idx: Option<usize>,
    /// Minimum pad size filter: 'S' = no filter, 'M' = medium or large, 'L' = large only.
    pad_filter: char,
    pending: Arc<Mutex<Option<Result<Vec<NearestCommodityResult>, String>>>>,
    // ── Multi-commodity mode ──────────────────────────────────────
    multi_mode: bool,
    multi_results: Vec<MultiCommodityResult>,
    /// Total number of commodities in the current multi search.
    multi_total: usize,
    /// Display names of the searched commodities (for the summary panel).
    multi_commodities: Vec<String>,
    /// Canonical names sent to the API, index-aligned with `multi_commodities`.
    multi_canonical: Vec<String>,
    pending_multi: Arc<Mutex<Option<Result<Vec<MultiCommodityResult>, String>>>>,
    clipboard: Option<arboard::Clipboard>,
}

impl SearchNearestView {
    pub fn new() -> Self {
        Self {
            commodity_input: String::new(),
            commodity_canonical: String::new(),
            system_input: String::new(),
            active_field: ActiveField::Commodity,
            editing: false,
            results: Vec::new(),
            selected_idx: 0,
            scroll: 0,
            status_msg: "Press Enter to start editing".into(),
            is_loading: false,
            suggestions: Vec::new(),
            suggestion_idx: None,
            pad_filter: 'S',
            pending: Arc::new(Mutex::new(None)),
            multi_mode: false,
            multi_results: Vec::new(),
            multi_total: 0,
            multi_commodities: Vec::new(),
            multi_canonical: Vec::new(),
            pending_multi: Arc::new(Mutex::new(None)),
            clipboard: arboard::Clipboard::new().ok(),
        }
    }

    pub fn prefill_and_search(&mut self, commodity: &str, canonical_name: &str, system: &str, ship_pad_size: char, api: &ApiClient) {
        self.multi_mode = false;
        self.commodity_input = commodity.to_string();
        self.commodity_canonical = canonical_name.to_string();
        self.system_input = system.to_string();
        self.pad_filter = ship_pad_size;
        self.active_field = ActiveField::Commodity;
        self.editing = false;
        self.do_search(api);
    }

    /// `commodities` are the display names shown to the user; `canonical_names`
    /// (index-aligned) are what the API matches on.
    pub fn start_multi_search(&mut self, commodities: Vec<String>, canonical_names: Vec<String>, system: String, ship_pad_size: char, api: &ApiClient) {
        self.multi_mode = true;
        self.multi_total = commodities.len();
        self.multi_commodities = commodities.clone();
        self.multi_canonical = canonical_names.clone();
        self.system_input = system.clone();
        self.pad_filter = ship_pad_size;
        self.selected_idx = 0;
        self.scroll = 0;
        self.multi_results.clear();
        self.is_loading = true;
        self.status_msg = format!("Searching {} commodities near '{}'…", commodities.len(), system);
        self.do_multi_search(api, canonical_names, system);
    }

    fn do_multi_search(&mut self, api: &ApiClient, commodities: Vec<String>, system: String) {
        let pending = Arc::clone(&self.pending_multi);
        let api_owned = api.clone();
        let query = MultiCommodityQuery { commodities, reference_system: system, limit: Some(15) };
        api.spawn(async move {
            let result = api_owned.search_nearest_multi_commodity(&query).await.map_err(|e| e.to_string());
            *pending.lock().unwrap() = Some(result);
        });
    }

    fn pad_matches(&self, r: &NearestCommodityResult) -> bool {
        match self.pad_filter {
            'L' => r.has_large_pad,
            'M' => r.has_large_pad || r.has_medium_pad,
            _ => true,
        }
    }

    fn normalize_for_match(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    }

    fn update_suggestions(&mut self) {
        if self.commodity_input.is_empty() {
            self.suggestions.clear();
            self.suggestion_idx = None;
            return;
        }
        let needle = Self::normalize_for_match(&self.commodity_input);
        self.suggestions = COMMODITY_NAMES
            .iter()
            .copied()
            .filter(|name| Self::normalize_for_match(name).contains(&needle))
            .take(MAX_SUGGESTIONS)
            .collect();
        if let Some(idx) = self.suggestion_idx {
            if idx >= self.suggestions.len() {
                self.suggestion_idx = None;
            }
        }
    }

    fn do_search(&mut self, api: &ApiClient) {
        if self.commodity_input.is_empty() {
            self.status_msg = "Enter a commodity name first".into();
            return;
        }
        self.is_loading = true;
        self.status_msg = format!(
            "Searching for '{}' near '{}'…",
            self.commodity_input, self.system_input
        );
        let pending = Arc::clone(&self.pending);
        let api_owned = api.clone();
        let commodity_query = if self.commodity_canonical.is_empty() {
            self.commodity_input.clone()
        } else {
            self.commodity_canonical.clone()
        };
        let query = NearestCommodityQuery {
            commodity: commodity_query,
            reference_system: self.system_input.clone(),
            limit: Some(10),
        };
        api.spawn(async move {
            let result = api_owned
                .search_nearest_commodity(&query)
                .await
                .map_err(|e| e.to_string());
            *pending.lock().unwrap() = Some(result);
        });
    }

    pub fn poll_search(&mut self) {
        if let Some(result) = self.pending.lock().unwrap().take() {
            self.is_loading = false;
            match result {
                Ok(results) => {
                    self.results = results.into_iter().filter(|r| self.pad_matches(r)).collect();
                    let count = self.results.len();
                    self.selected_idx = 0;
                    self.scroll = 0;
                    let pad_note = match self.pad_filter {
                        'L' => " (large pad only)",
                        'M' => " (medium+ pad)",
                        _ => "",
                    };
                    self.status_msg = if count == 0 {
                        if self.system_input.is_empty() {
                            format!("No '{}' in stock anywhere{pad_note}", self.commodity_input)
                        } else {
                            format!(
                                "No '{}' found near '{}'{pad_note} — system may not be in DB",
                                self.commodity_input, self.system_input
                            )
                        }
                    } else {
                        format!("{count} station(s) found{pad_note}")
                    };
                }
                Err(e) => {
                    warn!(error = %e, "search_nearest_commodity error");
                    self.status_msg = format!("API error: {e}");
                }
            }
        }
        if let Some(result) = self.pending_multi.lock().unwrap().take() {
            self.is_loading = false;
            match result {
                Ok(results) => {
                    self.multi_results = results.into_iter()
                        .filter(|r| match self.pad_filter {
                            'L' => r.has_large_pad,
                            'M' => r.has_large_pad || r.has_medium_pad,
                            _ => true,
                        })
                        .map(|mut r| {
                            r.matched_commodities = r.matched_commodities.iter()
                                .map(|m| display_for_matched(m, &self.multi_canonical, &self.multi_commodities))
                                .collect();
                            r
                        })
                        .collect();
                    let count = self.multi_results.len();
                    let pad_note = match self.pad_filter {
                        'L' => " (large pad only)",
                        'M' => " (medium+ pad)",
                        _ => "",
                    };
                    self.status_msg = if count == 0 {
                        format!("No stations found near '{}'{pad_note}", self.system_input)
                    } else {
                        format!("{count} station(s) found{pad_note}")
                    };
                }
                Err(e) => {
                    warn!(error = %e, "search_nearest_multi_commodity error");
                    self.status_msg = format!("API error: {e}");
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent, api: &ApiClient) -> ViewEvent {
        if self.editing {
            match key.code {
                KeyCode::Enter => {
                    if let Some(idx) = self.suggestion_idx {
                        if let Some(&name) = self.suggestions.get(idx) {
                            self.commodity_input = name.to_string();
                        }
                    }
                    self.editing = false;
                    self.suggestions.clear();
                    self.suggestion_idx = None;
                    self.do_search(api);
                }
                KeyCode::Esc => {
                    self.editing = false;
                    self.suggestions.clear();
                    self.suggestion_idx = None;
                }
                KeyCode::Tab => {
                    if self.active_field == ActiveField::Commodity && !self.suggestions.is_empty() {
                        let idx = self.suggestion_idx.unwrap_or(0);
                        if let Some(&name) = self.suggestions.get(idx) {
                            self.commodity_input = name.to_string();
                        }
                        self.suggestions.clear();
                        self.suggestion_idx = None;
                        self.active_field = ActiveField::System;
                    } else {
                        self.suggestions.clear();
                        self.suggestion_idx = None;
                        self.active_field = match self.active_field {
                            ActiveField::Commodity => ActiveField::System,
                            ActiveField::System => ActiveField::Commodity,
                        };
                    }
                }
                KeyCode::Down => {
                    if self.active_field == ActiveField::Commodity && !self.suggestions.is_empty() {
                        let next = self.suggestion_idx.map_or(0, |i| (i + 1).min(self.suggestions.len() - 1));
                        self.suggestion_idx = Some(next);
                    }
                }
                KeyCode::Up => {
                    if self.active_field == ActiveField::Commodity && !self.suggestions.is_empty() {
                        self.suggestion_idx = match self.suggestion_idx {
                            Some(0) | None => None,
                            Some(i) => Some(i - 1),
                        };
                    }
                }
                KeyCode::Backspace => match self.active_field {
                    ActiveField::Commodity => {
                        self.commodity_input.pop();
                        self.commodity_canonical.clear();
                        self.update_suggestions();
                    }
                    ActiveField::System => { self.system_input.pop(); }
                },
                KeyCode::Char(c) => match self.active_field {
                    ActiveField::Commodity => {
                        self.commodity_input.push(c);
                        self.commodity_canonical.clear();
                        self.suggestion_idx = None;
                        self.update_suggestions();
                    }
                    ActiveField::System => self.system_input.push(c),
                },
                _ => {}
            }
            return ViewEvent::Consumed;
        }

        match key.code {
            KeyCode::Enter => {
                if !self.multi_mode {
                    self.editing = true;
                }
                return ViewEvent::Consumed;
            }
            KeyCode::Char('w') | KeyCode::Up => {
                self.selected_idx = self.selected_idx.saturating_sub(1);
                if self.selected_idx < self.scroll {
                    self.scroll = self.selected_idx;
                }
            }
            KeyCode::Char('s') | KeyCode::Down => {
                let max = if self.multi_mode { self.multi_results.len() } else { self.results.len() };
                if self.selected_idx + 1 < max {
                    self.selected_idx += 1;
                }
            }
            KeyCode::Char('c') => {
                let name = if self.multi_mode {
                    self.multi_results.get(self.selected_idx).map(|r| r.system_name.trim().to_string())
                } else {
                    self.results.get(self.selected_idx).map(|r| r.system_name.trim().to_string())
                };
                if let Some(name) = name {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_search_names_sends_canonical_keeps_localised_display() {
        let (display, canonical) = multi_search_names([
            ("Keramik-Verbundwerkstoffe", "$ceramiccomposites_name;"),
            ("Computerteile", "$ComputerComponents_Name;"),
            ("Gebäudefabrikatoren", "$buildingfabricators_name;"),
        ]);
        assert_eq!(display, ["Keramik-Verbundwerkstoffe", "Computerteile", "Gebäudefabrikatoren"]);
        assert_eq!(canonical, ["Ceramic Composites", "Computer Components", "Building Fabricators"]);
    }

    #[test]
    fn multi_search_names_falls_back_to_symbol_for_unknown_commodity() {
        let (_, canonical) = multi_search_names([("Mikrobielle Öfen", "$heliostaticfurnaces_name;")]);
        assert_eq!(canonical, ["heliostaticfurnaces"]);
    }

    #[test]
    fn matched_server_names_map_back_to_display_names() {
        let canonical = vec!["Ceramic Composites".to_string(), "heliostaticfurnaces".to_string()];
        let display = vec!["Keramik-Verbundwerkstoffe".to_string(), "Mikrobielle Öfen".to_string()];
        assert_eq!(display_for_matched("ceramiccomposites", &canonical, &display), "Keramik-Verbundwerkstoffe");
        assert_eq!(display_for_matched("heliostaticfurnaces", &canonical, &display), "Mikrobielle Öfen");
        assert_eq!(display_for_matched("steel", &canonical, &display), "steel");
    }
}
