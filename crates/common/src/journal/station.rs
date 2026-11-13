use serde::{Deserialize, Serialize};

use crate::journal::types::{LandingPads, StationEconomy, StationFaction};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Docked {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "StationName")]
    pub station_name: String,
    #[serde(rename = "StationType")]
    pub station_type: String,
    #[serde(rename = "MarketID")]
    pub market_id: i64,
    #[serde(rename = "SystemAddress")]
    pub system_address: i64,
    #[serde(rename = "StarSystem")]
    pub star_system: String,
    #[serde(rename = "StationFaction")]
    pub station_faction: Option<StationFaction>,
    #[serde(rename = "StationGovernment", default)]
    pub station_government: String,
    #[serde(rename = "StationGovernment_Localised", default)]
    pub station_government_localised: String,
    #[serde(rename = "StationAllegiance", default)]
    pub station_allegiance: String,
    #[serde(rename = "StationServices")]
    pub station_services: Option<Vec<String>>,
    #[serde(rename = "StationEconomy", default)]
    pub station_economy: String,
    #[serde(rename = "StationEconomy_Localised", default)]
    pub station_economy_localised: String,
    #[serde(rename = "StationEconomies")]
    pub station_economies: Option<Vec<StationEconomy>>,
    #[serde(rename = "LandingPads")]
    pub landing_pads: Option<LandingPads>,
    #[serde(rename = "DistFromStarLS")]
    pub dist_from_star_ls: Option<f32>,
    #[serde(rename = "Wanted", default)]
    pub wanted: bool,
    #[serde(rename = "ActiveFine", default)]
    pub active_fine: bool,
    #[serde(rename = "Taxi", default)]
    pub taxi: bool,
    #[serde(rename = "horizons", default)]
    pub horizons: bool,
    #[serde(rename = "odyssey", default)]
    pub odyssey: bool,
}

/// Emitted when the player views their fleet carrier stats panel.
/// Contains the custom owner-given name separate from the callsign.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CarrierStats {
    #[serde(rename = "CarrierID")]
    pub carrier_id: i64,
    #[serde(rename = "Callsign")]
    pub callsign: String,
    #[serde(rename = "Name")]
    pub name: String,
}

/// EDDN commodities schema (not a journal event; sent separately by the game)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Commodities {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "marketId")]
    pub market_id: i64,
    #[serde(rename = "stationName")]
    pub station_name: String,
    #[serde(rename = "systemName")]
    pub system_name: String,
    #[serde(rename = "horizons", default)]
    pub horizons: bool,
    #[serde(rename = "odyssey", default)]
    pub odyssey: bool,
    #[serde(rename = "commodities")]
    pub commodities: Vec<Commodity>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Commodity {
    pub name: String,
    #[serde(rename = "meanPrice")]
    pub mean_price: i32,
    #[serde(rename = "buyPrice")]
    pub buy_price: i32,
    #[serde(rename = "stock")]
    pub stock: i32,
    #[serde(rename = "stockBracket")]
    pub stock_bracket: i32,
    #[serde(rename = "sellPrice")]
    pub sell_price: i32,
    #[serde(rename = "demand")]
    pub demand: i32,
    #[serde(rename = "demandBracket")]
    pub demand_bracket: i32,
    #[serde(rename = "statusFlags")]
    pub status_flags: Option<Vec<String>>,
}

/// EDDN outfitting / modules schema
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Outfitting {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "marketId")]
    pub market_id: i64,
    #[serde(rename = "stationName")]
    pub station_name: String,
    #[serde(rename = "systemName")]
    pub system_name: String,
    #[serde(rename = "horizons", default)]
    pub horizons: bool,
    #[serde(rename = "odyssey", default)]
    pub odyssey: bool,
    #[serde(deserialize_with = "modules_any")]
    pub modules: Vec<OutfittingModule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutfittingModule {
    pub id: i64,
    pub category: String,
    pub name: String,
    pub cost: Option<i64>,
    pub ship: Option<String>,
}

/// EDDN shipyard schema
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Shipyard {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "marketId")]
    pub market_id: i64,
    #[serde(rename = "stationName")]
    pub station_name: String,
    #[serde(rename = "systemName")]
    pub system_name: String,
    #[serde(rename = "horizons", default)]
    pub horizons: bool,
    #[serde(rename = "odyssey", default)]
    pub odyssey: bool,
    #[serde(deserialize_with = "ships_any")]
    pub ships: Vec<ShipyardShip>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShipyardShip {
    pub id: i64,
    pub name: String,
    #[serde(rename = "basevalue")]
    pub base_value: Option<i64>,
}

/// A shipyard entry as EDDN's `shipyard/2` sends it (the ship's symbol alone) or
/// as the edcas client converts `Shipyard.json` (with an id and price).
#[derive(Deserialize)]
#[serde(untagged)]
enum ShipEntry {
    Symbol(String),
    Full(ShipyardShip),
}

/// The ships of a shipyard message, whichever form they come in. A bare symbol
/// gets its place in the list as id: the id only has to be unique per station.
fn ships_any<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<ShipyardShip>, D::Error> {
    let entries = Vec::<ShipEntry>::deserialize(d)?;
    Ok(entries
        .into_iter()
        .enumerate()
        .map(|(i, e)| match e {
            ShipEntry::Symbol(name) => ShipyardShip { id: i as i64, name, base_value: None },
            ShipEntry::Full(ship) => ship,
        })
        .collect())
}

/// An outfitting entry as EDDN's `outfitting/2` sends it (the module's symbol
/// alone), as some uploaders send it (`Outfitting.json`'s id, Name and BuyPrice),
/// or as the edcas client converts it.
#[derive(Deserialize)]
#[serde(untagged)]
enum ModuleEntry {
    Symbol(String),
    Full(OutfittingModule),
    Priced {
        id: i64,
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "BuyPrice")]
        buy_price: Option<i64>,
    },
}

fn modules_any<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<OutfittingModule>, D::Error> {
    let entries = Vec::<ModuleEntry>::deserialize(d)?;
    Ok(entries
        .into_iter()
        .enumerate()
        .map(|(i, e)| match e {
            ModuleEntry::Symbol(name) => OutfittingModule {
                id: i as i64,
                category: module_category(&name).to_owned(),
                name,
                cost: None,
                ship: None,
            },
            ModuleEntry::Full(module) => module,
            ModuleEntry::Priced { id, name, buy_price } => OutfittingModule {
                id,
                category: module_category(&name).to_owned(),
                name,
                cost: buy_price,
                ship: None,
            },
        })
        .collect())
}

/// A module's category as far as its symbol tells it.
fn module_category(name: &str) -> &'static str {
    let name = name.to_ascii_lowercase();
    if name.starts_with("hpt_") {
        "hardpoint"
    } else if name.starts_with("int_") {
        "internal"
    } else {
        ""
    }
}

// ── Journal companion-file format (PascalCase / game-native) ─────────────────

/// Market.json companion-file format (written by the game when opening the market UI).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketJournal {
    pub timestamp: String,
    #[serde(rename = "MarketID")]
    pub market_id: i64,
    #[serde(rename = "StationName")]
    pub station_name: String,
    #[serde(rename = "StarSystem")]
    pub star_system: String,
    #[serde(rename = "Items", default)]
    pub items: Vec<MarketJournalItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketJournalItem {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "MeanPrice")]
    pub mean_price: i32,
    #[serde(rename = "BuyPrice")]
    pub buy_price: i32,
    #[serde(rename = "Stock")]
    pub stock: i32,
    #[serde(rename = "StockBracket")]
    pub stock_bracket: i32,
    #[serde(rename = "SellPrice")]
    pub sell_price: i32,
    #[serde(rename = "Demand")]
    pub demand: i32,
    #[serde(rename = "DemandBracket")]
    pub demand_bracket: i32,
}

impl From<MarketJournal> for Commodities {
    fn from(m: MarketJournal) -> Self {
        let commodities = m.items.into_iter().map(|item| {
            // "$platinum_name;" → "platinum". Trim the `;` and `_name` suffixes
            // separately so a name without the `_name` part (e.g. "$Drones;")
            // doesn't keep a trailing `;`.
            let name = item.name
                .trim_start_matches('$')
                .trim_end_matches(';')
                .trim_end_matches("_name")
                .to_owned();
            Commodity {
                name,
                mean_price: item.mean_price,
                buy_price: item.buy_price,
                stock: item.stock,
                stock_bracket: item.stock_bracket,
                sell_price: item.sell_price,
                demand: item.demand,
                demand_bracket: item.demand_bracket,
                status_flags: None,
            }
        }).collect();
        Commodities {
            timestamp: m.timestamp,
            market_id: m.market_id,
            station_name: m.station_name,
            system_name: m.star_system,
            horizons: false,
            odyssey: false,
            commodities,
        }
    }
}

/// Outfitting.json companion-file format.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutfittingJournal {
    pub timestamp: String,
    #[serde(rename = "MarketID")]
    pub market_id: i64,
    #[serde(rename = "StationName")]
    pub station_name: String,
    #[serde(rename = "StarSystem")]
    pub star_system: String,
    #[serde(rename = "Items", default)]
    pub items: Vec<OutfittingJournalItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutfittingJournalItem {
    pub id: i64,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "BuyPrice")]
    pub buy_price: i32,
}

impl From<OutfittingJournal> for Outfitting {
    fn from(o: OutfittingJournal) -> Self {
        let modules = o.items.into_iter().map(|item| {
            OutfittingModule {
                id: item.id,
                category: module_category(&item.name).to_owned(),
                name: item.name,
                cost: Some(item.buy_price as i64),
                ship: None,
            }
        }).collect();
        Outfitting {
            timestamp: o.timestamp,
            market_id: o.market_id,
            station_name: o.station_name,
            system_name: o.star_system,
            horizons: false,
            odyssey: false,
            modules,
        }
    }
}

/// Shipyard.json companion-file format.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShipyardJournal {
    pub timestamp: String,
    #[serde(rename = "MarketID")]
    pub market_id: i64,
    #[serde(rename = "StationName")]
    pub station_name: String,
    #[serde(rename = "StarSystem")]
    pub star_system: String,
    #[serde(rename = "PriceList", default)]
    pub price_list: Vec<ShipyardJournalItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShipyardJournalItem {
    #[serde(rename = "ShipType")]
    pub ship_type: String,
    #[serde(rename = "ShipPrice")]
    pub ship_price: i64,
}

impl From<ShipyardJournal> for Shipyard {
    fn from(s: ShipyardJournal) -> Self {
        // The companion file always has id=0 for all ships, so we use a sequential
        // index as the unique key within this market to avoid DB conflicts.
        let ships = s.price_list.into_iter().enumerate().map(|(i, item)| {
            ShipyardShip {
                id: i as i64,
                name: item.ship_type,
                base_value: Some(item.ship_price),
            }
        }).collect();
        Shipyard {
            timestamp: s.timestamp,
            market_id: s.market_id,
            station_name: s.station_name,
            system_name: s.star_system,
            horizons: false,
            odyssey: false,
            ships,
        }
    }
}

/// Whether a `Shipyard.json` lists the commander's own ships at the station rather than
/// what the station sells. The game writes that file when the shipyard opens, and it
/// can hold the ship flown plus those stored there, each priced at its own value — at
/// Fuxport it named the Type-9 alone while the shipyard sold 36 types. Such a list is
/// recognised by the `StoredShips` event written with it: every ship stored here is in
/// the list at exactly its value, and the one entry left over is the ship flown
/// (`current_ship`, the `Loadout` event's `Ship`).
pub fn shipyard_is_own_fleet(
    shipyard: &serde_json::Value,
    stored_ships: &serde_json::Value,
    current_ship: Option<&str>,
) -> bool {
    let Some(list) = shipyard["PriceList"].as_array() else { return false };
    let here = stored_ships["ShipsHere"].as_array().map(Vec::as_slice).unwrap_or_default();
    if list.len() != here.len() + 1 {
        return false;
    }
    let ty = |v: &serde_json::Value, key: &str| v[key].as_str().map(str::to_lowercase);
    let mut left: Vec<&serde_json::Value> = list.iter().collect();
    for ship in here {
        let Some(at) = left
            .iter()
            .position(|e| ty(e, "ShipType") == ty(ship, "ShipType") && e["ShipPrice"].as_i64() == ship["Value"].as_i64())
        else {
            return false;
        };
        left.remove(at);
    }
    current_ship.is_some_and(|c| ty(left[0], "ShipType").as_deref() == Some(&c.to_lowercase()))
}

#[cfg(test)]
mod own_fleet_tests {
    use super::shipyard_is_own_fleet;
    use serde_json::json;

    #[test]
    fn own_fleet_lists_are_recognised() {
        // Legions Point, 2026-10-14: the Type-9 flown and three ships stored there.
        let shipyard = json!({ "PriceList": [
            { "ShipType": "type9", "ShipPrice": 169483352 },
            { "ShipType": "sidewinder", "ShipPrice": 0 },
            { "ShipType": "viper", "ShipPrice": 1654791 },
            { "ShipType": "type9_military", "ShipPrice": 321840307 } ] });
        let stored = json!({ "ShipsHere": [
            { "ShipType": "sidewinder", "Value": 0 },
            { "ShipType": "viper", "Value": 1654791 },
            { "ShipType": "type9_military", "Value": 321840307 } ] });
        assert!(shipyard_is_own_fleet(&shipyard, &stored, Some("Type9")));
        // Fuxport: nothing stored, the ship flown alone.
        let fuxport = json!({ "PriceList": [ { "ShipType": "type9", "ShipPrice": 169483352 } ] });
        assert!(shipyard_is_own_fleet(&fuxport, &json!({ "ShipsHere": [] }), Some("type9")));
    }

    #[test]
    fn stock_lists_are_not() {
        let stock = json!({ "PriceList": [
            { "ShipType": "sidewinder", "ShipPrice": 31200 },
            { "ShipType": "eagle", "ShipPrice": 43680 } ] });
        let stored = json!({ "ShipsHere": [ { "ShipType": "sidewinder", "Value": 0 } ] });
        assert!(!shipyard_is_own_fleet(&stock, &stored, Some("eagle")));
        // One ship for sale, the same type as the one flown but nothing stored: the
        // flown ship decides.
        let one = json!({ "PriceList": [ { "ShipType": "adder", "ShipPrice": 85613 } ] });
        assert!(!shipyard_is_own_fleet(&one, &json!({ "ShipsHere": [] }), Some("type9")));
        assert!(!shipyard_is_own_fleet(&one, &json!({ "ShipsHere": [] }), None));
    }
}

#[cfg(test)]
mod eddn_shape_tests {
    use crate::journal::JournalEvent;
    use serde_json::json;

    #[test]
    fn eddn_shipyard_lists_of_symbols_are_read() {
        let msg = json!({ "ships": ["panthermkii", "smallcombat01_nx", "vulture"], "odyssey": true, "horizons": true,
            "marketId": 128666762, "timestamp": "2026-10-14T01:43:03Z", "systemName": "Shinrarta Dezhra",
            "stationName": "Jameson Memorial", "allowCobraMkIV": true });
        let Some(JournalEvent::Shipyard(s)) = JournalEvent::from_eddn_message(msg) else { panic!("not read") };
        let names: Vec<&str> = s.ships.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["panthermkii", "smallcombat01_nx", "vulture"]);
        assert_eq!(s.ships.iter().map(|s| s.id).collect::<Vec<_>>(), [0, 1, 2]);
    }

    #[test]
    fn eddn_outfitting_symbols_and_priced_entries_are_read() {
        let symbols = json!({ "modules": ["krait_light_armour_mirrored", "hpt_railgun_fixed_small"], "marketId": 1,
            "timestamp": "2026-10-14T01:43:03Z", "systemName": "Sol", "stationName": "Abraham Lincoln" });
        let Some(JournalEvent::Outfitting(o)) = JournalEvent::from_eddn_message(symbols) else { panic!("not read") };
        assert_eq!(o.modules.len(), 2);
        assert_eq!(o.modules[1].category, "hardpoint");
        let priced = json!({ "modules": [{ "id": 128049489, "Name": "hpt_railgun_fixed_medium", "BuyPrice": 342108, "BuyMercCoinsPrice": 0 }],
            "marketId": 1, "timestamp": "2026-10-14T01:43:03Z", "systemName": "Sol", "stationName": "Abraham Lincoln" });
        let Some(JournalEvent::Outfitting(o)) = JournalEvent::from_eddn_message(priced) else { panic!("not read") };
        assert_eq!((o.modules[0].id, o.modules[0].cost), (128049489, Some(342108)));
    }
}
