use std::cmp::Ordering;
use std::collections::HashMap;

use edcas_common::api::CommodityResponse;
use ratatui::style::{Color, Modifier, Style};

use crate::journal_reader::JournalData;
use crate::todo::ModKind;

/// Style for a view's free-text status/message line, inferred from its content so callers
/// don't need to thread a separate "is this an error?" flag through every call site.
/// Mirrors the colors `news.rs` already uses for its explicit loading/error states.
pub(super) fn status_style(msg: &str) -> Style {
    let lower = msg.to_lowercase();
    if lower.starts_with("loading") || lower.starts_with("searching") {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else if lower.contains("error") || lower.contains("unavailable") || lower.contains("no results")
        || lower.contains("not found") || lower.contains("no ") && lower.contains("data yet")
    {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

pub(super) fn normalize_commodity_name(name: &str) -> String {
    name.trim_start_matches('$')
        .trim_end_matches(';')
        .trim_end_matches("_name")
        .to_lowercase()
}

/// Sortable market columns, used by both station and carrier views.
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum MarketSortCol {
    #[default]
    Name,
    Buy,
    BuyDiff,
    Sell,
    SellDiff,
    Mean,
    Stock,
    Demand,
}

impl MarketSortCol {
    pub(super) fn from_digit(c: char) -> Option<Self> {
        match c {
            '1' => Some(Self::Name),
            '2' => Some(Self::Buy),
            '3' => Some(Self::BuyDiff),
            '4' => Some(Self::Sell),
            '5' => Some(Self::SellDiff),
            '6' => Some(Self::Mean),
            '7' => Some(Self::Stock),
            '8' => Some(Self::Demand),
            _ => None,
        }
    }
}

/// Absolute credit deviation of `price` from `mean` for sorting (returns i32::MIN when N/A
/// so unavailable entries sort last in ascending order).
pub(super) fn raw_diff(price: i32, mean: i32) -> i32 {
    if price <= 0 || mean <= 0 { i32::MIN } else { price - mean }
}

pub(super) fn fmt_ts(ts: Option<&chrono::DateTime<chrono::Utc>>) -> String {
    ts.map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "\u{2014}".to_string())
}

pub(super) fn fmt_ts_short(ts: Option<&chrono::DateTime<chrono::Utc>>) -> String {
    ts.map(|t| t.format("%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "\u{2014}".to_string())
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum SearchState {
    Idle,
    Typing,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum FocusArea {
    List,
    Detail,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum StationDetailTab {
    Overview,
    Market,
    Outfitting,
    Shipyard,
    PriceHistory,
}

impl StationDetailTab {
    pub(super) fn next(self) -> Option<Self> {
        match self {
            Self::Overview      => Some(Self::Market),
            Self::Market        => Some(Self::Outfitting),
            Self::Outfitting    => Some(Self::Shipyard),
            Self::Shipyard      => Some(Self::PriceHistory),
            Self::PriceHistory  => None,
        }
    }
    pub(super) fn prev(self) -> Option<Self> {
        match self {
            Self::Overview      => None,
            Self::Market        => Some(Self::Overview),
            Self::Outfitting    => Some(Self::Market),
            Self::Shipyard      => Some(Self::Outfitting),
            Self::PriceHistory  => Some(Self::Shipyard),
        }
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Overview      => "Overview",
            Self::Market        => "Market",
            Self::Outfitting    => "Outfitting",
            Self::Shipyard      => "Shipyard",
            Self::PriceHistory  => "Price History",
        }
    }
}

/// Compute remaining construction needs, subtracting ship cargo.
pub(super) fn compute_todo_needed(
    construction_items: &[crate::todo::ConstructionTodoItem],
    cargo: &[crate::journal_reader::CargoItem],
) -> HashMap<String, i32> {
    let mut needed: HashMap<String, i32> = HashMap::new();
    for item in construction_items {
        for res in &item.resources {
            let remaining = (res.required_amount - res.provided_amount).max(0);
            if remaining > 0 {
                *needed.entry(normalize_commodity_name(&res.commodity_name)).or_insert(0) += remaining;
            }
        }
    }
    for c in cargo {
        let norm = normalize_commodity_name(&c.name);
        if let Some(n) = needed.get_mut(&norm) {
            *n = (*n - c.count).max(0);
        }
    }
    needed.retain(|_, v| *v > 0);
    needed
}

/// Filter `todo_needed` to only commodities a market actually has for sale.
pub(super) fn effective_todo_for_market(
    commodities: &[CommodityResponse],
    todo_needed: &HashMap<String, i32>,
) -> HashMap<String, i32> {
    commodities.iter()
        .filter(|c| c.buy_price > 0 && c.stock > 0)
        .filter_map(|c| {
            let norm = normalize_commodity_name(&c.name);
            todo_needed.get(&norm).map(|&n| (norm, n))
        })
        .collect()
}

/// Sort commodities: todo items first, then by the chosen column.
pub(super) fn sorted_commodities<'a>(
    commodities: &'a [CommodityResponse],
    effective_todo: &HashMap<String, i32>,
    sort_col: MarketSortCol,
    sort_asc: bool,
) -> Vec<&'a CommodityResponse> {
    let mut sorted: Vec<&CommodityResponse> = commodities.iter().collect();
    sorted.sort_by(|a, b| {
        let a_norm = normalize_commodity_name(&a.name);
        let b_norm = normalize_commodity_name(&b.name);
        let group = effective_todo.contains_key(&b_norm).cmp(&effective_todo.contains_key(&a_norm));
        if group != Ordering::Equal { return group; }
        let col_ord = match sort_col {
            MarketSortCol::Name     => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            MarketSortCol::Buy      => a.buy_price.cmp(&b.buy_price),
            MarketSortCol::BuyDiff  => raw_diff(a.buy_price, a.mean_price).cmp(&raw_diff(b.buy_price, b.mean_price)),
            MarketSortCol::Sell     => a.sell_price.cmp(&b.sell_price),
            MarketSortCol::SellDiff => raw_diff(a.sell_price, a.mean_price).cmp(&raw_diff(b.sell_price, b.mean_price)),
            MarketSortCol::Mean     => a.mean_price.cmp(&b.mean_price),
            MarketSortCol::Stock    => a.stock.cmp(&b.stock),
            MarketSortCol::Demand   => a.demand.cmp(&b.demand),
        };
        if sort_asc { col_ord } else { col_ord.reverse() }
    });
    sorted
}

/// Squashes a material name down to lowercase alphanumerics so the journal's
/// internal `Name` (e.g. "carbonfibreplating") can be compared against the
/// bundled modifications.json's spaced display name (e.g. "Carbon Fibre
/// Plating") regardless of casing/spacing differences.
fn normalize_material_key(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// Count of a material/item the commander currently holds, keyed by `kind`
/// (ship engineering materials vs on-foot inventory). Shared by the Todo view
/// and the Engineers view so both show the same "have" counts.
///
/// Matches only against `InventoryItem.name` — the journal's internal
/// identifier, which is always English and locale-independent — never
/// `.localised`, which is whatever language the game client was running in
/// when that item was picked up (so it can be German in one session's
/// journal and English in another's, and never matches modifications.json's
/// English display names).
pub(super) fn have_count(mat_name: &str, kind: &ModKind, journal: &JournalData) -> i32 {
    let key = normalize_material_key(mat_name);
    match kind {
        ModKind::Ship => {
            journal
                .materials_raw
                .iter()
                .chain(journal.materials_manufactured.iter())
                .chain(journal.materials_encoded.iter())
                .filter(|i| normalize_material_key(&i.name) == key)
                .map(|i| i.count)
                .sum()
        }
        ModKind::OnFoot => {
            journal
                .shiplocker
                .items
                .iter()
                .chain(journal.shiplocker.components.iter())
                .chain(journal.shiplocker.consumables.iter())
                .chain(journal.shiplocker.data.iter())
                .chain(journal.backpack.items.iter())
                .chain(journal.backpack.components.iter())
                .chain(journal.backpack.consumables.iter())
                .chain(journal.backpack.data.iter())
                // Sum rather than find-first: the same material can be split between the
                // ship locker and the backpack (carried on foot vs. stored), and a single
                // container's own list is already merged per-material by parse_inventory_array.
                .filter(|i| normalize_material_key(&i.name) == key)
                .map(|i| i.count)
                .sum()
        }
    }
}

#[cfg(test)]
mod have_count_tests {
    use super::*;
    use crate::journal_reader::InventoryItem;

    fn item(name: &str, localised: &str, count: i32) -> InventoryItem {
        InventoryItem { name: name.to_string(), localised: localised.to_string(), count }
    }

    // A material can be split between the ship locker and the backpack (some carried on
    // foot, some stored) — have_count must sum both, not just report the first it finds.
    #[test]
    fn onfoot_have_count_sums_shiplocker_and_backpack() {
        let mut journal = JournalData::new();
        journal.shiplocker.components.push(item("carbonfibreplating", "Carbon Fibre Plating", 4));
        journal.backpack.components.push(item("carbonfibreplating", "Carbon Fibre Plating", 3));

        assert_eq!(have_count("Carbon Fibre Plating", &ModKind::OnFoot, &journal), 7);
    }

    #[test]
    fn onfoot_have_count_matches_internal_name_regardless_of_case_or_spacing() {
        let mut journal = JournalData::new();
        journal.shiplocker.items.push(item("geneticrepairmeds", "Genetic Repair Meds", 2));

        assert_eq!(have_count("geneticrepairmeds", &ModKind::OnFoot, &journal), 2);
        assert_eq!(have_count("Genetic Repair Meds", &ModKind::OnFoot, &journal), 2);
    }

    // Regression test: modifications.json's material names are always English
    // ("Carbon Fibre Plating"), but `.localised` reflects whatever language the
    // game client was in when an item was picked up — German in this case. The
    // match must go through `.name` (always "carbonfibreplating", locale-independent),
    // never `.localised`, or every non-English player sees 0 have-counts on foot.
    #[test]
    fn onfoot_have_count_ignores_non_english_localised_name() {
        let mut journal = JournalData::new();
        journal.shiplocker.components.push(item("carbonfibreplating", "Carbonfaserplatten", 6));

        assert_eq!(have_count("Carbon Fibre Plating", &ModKind::OnFoot, &journal), 6);
    }

    #[test]
    fn ship_have_count_matches_internal_name_only() {
        let mut journal = JournalData::new();
        journal.materials_raw.push(item("carbon", "Kohlenstoff", 42));

        assert_eq!(have_count("carbon", &ModKind::Ship, &journal), 42);
    }
}
