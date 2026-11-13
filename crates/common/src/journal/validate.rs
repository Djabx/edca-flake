//! Semantic validation/sanitation of parsed journal events, applied at the
//! server's single ingest choke point before events reach the derived tables.
//!
//! Parsing stays deliberately lenient (no `deny_unknown_fields`, lots of
//! `#[serde(default)]`) so new game/EDDN fields never break ingest. This module
//! is the semantic layer on top:
//! - [`JournalEvent::sanitize`] makes *soft* fixes in place — trims/truncates
//!   strings, lowercases commodity names, clamps influence/population into range,
//!   and drops clearly-bad individual commodity rows (keeping the rest).
//! - [`JournalEvent::validate`] *hard-rejects* an event (don't dispatch) when it
//!   would corrupt the data: non-positive identifiers, impossible coordinates,
//!   empty key names, or a market left with no valid commodities.

use super::types::Faction;
use super::JournalEvent;

/// Generous galaxy coordinate ceiling (Ly) — rejects clearly-corrupt positions
/// while staying well outside the real galaxy's extent.
const COORD_BOUND: f32 = 100_000.0;
/// Sanity ceiling for commodity prices (CR/unit); rejects garbage without
/// clipping any real commodity.
const PRICE_CAP: i32 = 10_000_000;
/// Longest string kept — matches the `VARCHAR(255)` columns so an over-long
/// value is clipped here rather than erroring the insert.
const MAX_STR: usize = 255;

impl JournalEvent {
    /// Soft, in-place fixes applied before [`validate`](Self::validate)/dispatch.
    pub fn sanitize(&mut self) {
        match self {
            Self::Commodities(c) => {
                clean_str(&mut c.station_name);
                c.commodities.retain(commodity_ok);
                for x in &mut c.commodities {
                    x.name = x.name.trim().to_lowercase();
                    truncate_chars(&mut x.name);
                }
            }
            Self::Docked(d) => clean_str(&mut d.station_name),
            Self::Outfitting(o) => clean_str(&mut o.station_name),
            Self::Shipyard(s) => clean_str(&mut s.station_name),
            Self::FsdJump(e) => {
                clean_str(&mut e.star_system);
                e.population = e.population.max(0);
                clamp_influence(&mut e.factions);
            }
            Self::Location(e) => {
                clean_str(&mut e.star_system);
                e.population = e.population.max(0);
                clamp_influence(&mut e.factions);
            }
            Self::CarrierJump(e) => {
                clean_str(&mut e.star_system);
                e.population = e.population.max(0);
                clamp_influence(&mut e.factions);
            }
            _ => {}
        }
    }

    /// Hard rejection check. `Err(reason)` means the event must NOT be dispatched
    /// to the derived tables (the raw event is still stored for provenance).
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::FsdJump(e) => {
                check_id(e.system_address)?;
                check_coords(&e.star_pos)?;
            }
            Self::Location(e) => {
                check_id(e.system_address)?;
                check_coords(&e.star_pos)?;
            }
            Self::CarrierJump(e) => {
                check_id(e.system_address)?;
                check_coords(&e.star_pos)?;
            }
            Self::Docked(e) => {
                check_id(e.market_id)?;
                if e.station_name.trim().is_empty() {
                    return Err("empty station_name");
                }
            }
            Self::Commodities(e) => {
                check_id(e.market_id)?;
                if e.commodities.is_empty() {
                    return Err("no valid commodities");
                }
            }
            Self::Outfitting(e) => check_id(e.market_id)?,
            Self::Shipyard(e) => check_id(e.market_id)?,
            Self::Scan(e) => check_id(e.system_address)?,
            Self::ColonisationConstructionDepot(e) => check_id(e.market_id)?,
            _ => {}
        }
        Ok(())
    }
}

fn check_id(id: i64) -> Result<(), &'static str> {
    if id > 0 {
        Ok(())
    } else {
        Err("non-positive identifier")
    }
}

fn check_coords(pos: &[f32]) -> Result<(), &'static str> {
    if pos.len() != 3 {
        return Err("star_pos must have 3 coordinates");
    }
    if pos.iter().any(|c| !c.is_finite() || c.abs() > COORD_BOUND) {
        return Err("coordinates out of range");
    }
    Ok(())
}

fn commodity_ok(x: &super::station::Commodity) -> bool {
    !x.name.trim().is_empty()
        && (0..=PRICE_CAP).contains(&x.buy_price)
        && (0..=PRICE_CAP).contains(&x.sell_price)
        && (0..=PRICE_CAP).contains(&x.mean_price)
        && x.stock >= 0
        && x.demand >= 0
        && (0..=3).contains(&x.stock_bracket)
        && (0..=3).contains(&x.demand_bracket)
}

fn clamp_influence(factions: &mut Option<Vec<Faction>>) {
    if let Some(fs) = factions {
        for f in fs {
            f.influence = if f.influence.is_finite() {
                f.influence.clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
    }
}

fn clean_str(s: &mut String) {
    let trimmed = s.trim();
    if trimmed.len() != s.len() {
        *s = trimmed.to_string();
    }
    truncate_chars(s);
}

fn truncate_chars(s: &mut String) {
    if s.chars().count() > MAX_STR {
        *s = s.chars().take(MAX_STR).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::super::JournalEvent;

    fn ev(json: &str) -> JournalEvent {
        JournalEvent::from_eddn_message(serde_json::from_str(json).unwrap()).expect("parses")
    }

    const TS: &str = r#""timestamp":"2024-01-01T00:00:00Z""#;

    #[test]
    fn rejects_bad_identifiers_and_coords() {
        let mut e = ev(&format!(r#"{{{TS},"event":"FSDJump","StarSystem":"X","SystemAddress":0,"StarPos":[1.0,2.0,3.0]}}"#));
        e.sanitize();
        assert!(e.validate().is_err(), "system_address 0 must reject");

        let mut e = ev(&format!(r#"{{{TS},"event":"FSDJump","StarSystem":"X","SystemAddress":1,"StarPos":[1e9,0.0,0.0]}}"#));
        e.sanitize();
        assert!(e.validate().is_err(), "absurd coordinate must reject");

        let mut e = ev(&format!(r#"{{{TS},"event":"FSDJump","StarSystem":"X","SystemAddress":1,"StarPos":[1.0,2.0,3.0]}}"#));
        e.sanitize();
        assert!(e.validate().is_ok(), "normal jump must pass");
    }

    #[test]
    fn sanitizes_commodities_and_drops_bad_rows() {
        let mut e = ev(
            r#"{"timestamp":"2024-01-01T00:00:00Z","marketId":42,"stationName":" Foo ","systemName":"Sys","commodities":[
                {"name":"Steel","meanPrice":100,"buyPrice":50,"stock":10,"stockBracket":2,"sellPrice":60,"demand":5,"demandBracket":2,"statusFlags":[]},
                {"name":"junk","meanPrice":-1,"buyPrice":-5,"stock":-9,"stockBracket":9,"sellPrice":0,"demand":0,"demandBracket":0,"statusFlags":[]}
            ]}"#,
        );
        e.sanitize();
        assert!(e.validate().is_ok());
        if let JournalEvent::Commodities(c) = &e {
            assert_eq!(c.station_name, "Foo", "station name trimmed");
            assert_eq!(c.commodities.len(), 1, "bad row dropped");
            assert_eq!(c.commodities[0].name, "steel", "name lowercased");
        } else {
            panic!("expected Commodities");
        }
    }
}
