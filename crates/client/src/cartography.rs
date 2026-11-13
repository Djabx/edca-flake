//! Exploration data the commander carries and hasn't sold: every body scanned, and surface
//! mapped, since its system was last sold at Universal Cartographics, valued with
//! [`ScanValue`], and every organic species analysed since it was last sold at Vista
//! Genomics, valued with [`exobio_value`]. Rebuilt from the whole journal folder
//! ([`crate::journal_follow`]), since an expedition spans many sessions. Dying loses it all.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use edcas_common::exploration::{exobio_value, ScanValue, FIRST_FOOTFALL};
use edcas_common::journal::Scan;
use serde_json::Value;

use crate::journal_follow::{JournalFollower, JournalLog};

#[derive(Clone, Default)]
pub struct UnsoldData {
    /// By system address.
    systems: HashMap<i64, UnsoldSystem>,
    /// Analysed organic species, oldest first.
    bio: Vec<UnsoldBio>,
    /// The running client is Odyssey (from `LoadGame`): its mapping pays more.
    odyssey: bool,
    pub loaded: bool,
}

/// One analysed organic species.
#[derive(Clone)]
pub struct UnsoldBio {
    pub address: i64,
    pub body_id: Option<i32>,
    /// Codex symbol, what `SellOrganicData` names it by.
    species: String,
    /// Vista Genomics payout, first-footfall bonus included; `None` for a species
    /// [`exobio_value`] doesn't know.
    pub value: Option<i64>,
    /// The body was untrodden when scanned, so it pays the first-footfall bonus.
    pub first_footfall: bool,
}

#[derive(Clone, Default)]
pub struct UnsoldSystem {
    pub name: String,
    /// By body id.
    bodies: HashMap<i32, UnsoldBody>,
}

#[derive(Clone, Copy)]
struct UnsoldBody {
    value: ScanValue,
    /// The commander's own surface map: whether it met the probe efficiency target.
    mapped: Option<bool>,
    /// Someone had set foot on it (`WasFootfalled`), as of its first scan.
    footfalled: Option<bool>,
}

impl UnsoldSystem {
    pub fn credits(&self) -> i64 {
        self.bodies.values().map(|b| b.value.credits(b.mapped)).sum()
    }
}

impl UnsoldData {
    pub fn total(&self) -> i64 {
        self.systems.values().map(UnsoldSystem::credits).sum()
    }

    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    /// Whether the commander still carries unsold data of this body.
    pub fn holds(&self, address: i64, body_id: i32) -> bool {
        self.systems.get(&address).is_some_and(|s| s.bodies.contains_key(&body_id))
    }

    /// Estimated value of the unsold organic data.
    pub fn bio_total(&self) -> i64 {
        self.bio.iter().filter_map(|b| b.value).sum()
    }

    /// Analysed species not sold yet.
    pub fn bio_count(&self) -> usize {
        self.bio.len()
    }

    /// Whether the commander still carries analysed data of this species from this body.
    pub fn holds_bio(&self, address: i64, body_id: i32, species: &str) -> bool {
        self.bio_on(address, body_id).any(|b| b.species == species)
    }

    /// The unsold organic data of one body.
    pub fn bio_on(&self, address: i64, body_id: i32) -> impl Iterator<Item = &UnsoldBio> {
        self.bio.iter().filter(move |b| b.address == address && b.body_id == Some(body_id))
    }

    fn sold(&mut self, name: &str) {
        self.systems.retain(|_, s| s.name != name);
    }
}

impl JournalLog for UnsoldData {
    const NAME: &'static str = "unsold-data";
    const EVENTS: &'static [&'static [u8]] = &[
        b"\"event\":\"Scan\"",
        b"\"event\":\"SAAScanComplete\"",
        b"\"event\":\"SellExplorationData\"",
        b"\"event\":\"MultiSellExplorationData\"",
        b"\"event\":\"ScanOrganic\"",
        b"\"event\":\"SellOrganicData\"",
        b"\"event\":\"Died\"",
        b"\"event\":\"LoadGame\"",
    ];

    fn apply(&mut self, v: &Value) {
        match v["event"].as_str() {
            Some("LoadGame") => self.odyssey = v["Odyssey"].as_bool().unwrap_or(false),
            // Nav beacon data comes free and sells for nothing.
            Some("Scan") if v["ScanType"].as_str() != Some("NavBeaconDetail") => {
                let Ok(scan) = serde_json::from_value::<Scan>(v.clone()) else { return };
                let system = self.systems.entry(scan.system_address).or_default();
                if let Some(name) = v["StarSystem"].as_str() {
                    system.name = name.to_string();
                }
                // A rescan (FSS, then again with the surface map) keeps the map, and the
                // footfall as it was before the commander landed.
                let old = system.bodies.get(&scan.body_id);
                let mapped = old.and_then(|b| b.mapped);
                let footfalled = old.and_then(|b| b.footfalled).or(scan.was_footfalled);
                system.bodies.insert(scan.body_id, UnsoldBody { value: ScanValue::of(&scan, self.odyssey), mapped, footfalled });
            }
            Some("SAAScanComplete") => {
                let (Some(address), Some(id)) = (v["SystemAddress"].as_i64(), v["BodyID"].as_i64()) else { return };
                let efficient = v["ProbesUsed"].as_i64().unwrap_or(i64::MAX) <= v["EfficiencyTarget"].as_i64().unwrap_or(0);
                if let Some(body) = self.systems.get_mut(&address).and_then(|s| s.bodies.get_mut(&(id as i32))) {
                    body.mapped = Some(efficient);
                }
            }
            Some("SellExplorationData") => {
                for name in v["Systems"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                    self.sold(name);
                }
            }
            Some("MultiSellExplorationData") => {
                for name in v["Discovered"].as_array().into_iter().flatten().filter_map(|d| d["SystemName"].as_str()) {
                    self.sold(name);
                }
            }
            // The third sample completes a species.
            Some("ScanOrganic") if v["ScanType"].as_str() == Some("Analyse") => {
                let (Some(address), Some(species)) = (v["SystemAddress"].as_i64(), v["Species"].as_str()) else { return };
                let body_id = v["Body"].as_i64().and_then(|id| i32::try_from(id).ok());
                let footfalled = body_id.and_then(|id| self.systems.get(&address)?.bodies.get(&id)?.footfalled);
                let first_footfall = footfalled == Some(false);
                let value = exobio_value(species, v["Genus"].as_str().unwrap_or_default())
                    .map(|base| if first_footfall { base * FIRST_FOOTFALL } else { base });
                self.bio.push(UnsoldBio { address, body_id, species: species.to_string(), value, first_footfall });
            }
            // A sale clears every analysis of the species it lists (Radicoida's come merged).
            Some("SellOrganicData") => {
                let sold: Vec<&str> = v["BioData"].as_array().into_iter().flatten().filter_map(|d| d["Species"].as_str()).collect();
                self.bio.retain(|b| !sold.contains(&b.species.as_str()));
            }
            Some("Died") => {
                self.systems.clear();
                self.bio.clear();
            }
            _ => {}
        }
    }

    fn set_loaded(&mut self) {
        self.loaded = true;
    }

    fn summary(&self) -> String {
        format!(
            "{} Cr unsold in {} systems, {} Cr in {} organic species",
            self.total(),
            self.systems.len(),
            self.bio_total(),
            self.bio.len()
        )
    }
}

/// The unsold-data scan of the configured journal folder.
#[derive(Default)]
pub struct Unsold {
    scanner: Option<JournalFollower<UnsoldData>>,
}

impl Unsold {
    /// Starts the scan, or restarts it after the journal folder changed; cheap otherwise.
    pub fn follow(&mut self, dir: &str) {
        let dir = dir.trim();
        if dir.is_empty() {
            self.scanner = None;
            return;
        }
        let dir = PathBuf::from(dir);
        if self.scanner.as_ref().is_none_or(|s| s.dir != dir) {
            self.scanner = Some(JournalFollower::start(dir));
        }
    }

    pub fn data(&self) -> Arc<UnsoldData> {
        self.scanner.as_ref().map(JournalFollower::snapshot).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal_follow::feed;

    fn replay(lines: &[&str]) -> UnsoldData {
        let mut d = UnsoldData::default();
        feed(&mut d, (lines.join("\n") + "\n").as_bytes());
        d
    }

    const LOAD: &str = r#"{ "timestamp":"2026-10-12T22:00:00Z", "event":"LoadGame", "Odyssey":true }"#;
    const STAR: &str = r#"{ "timestamp":"2026-10-12T22:01:00Z", "event":"Scan", "ScanType":"AutoScan", "BodyName":"Oochont LI-N c9-0 A", "BodyID":1, "StarSystem":"Oochont LI-N c9-0", "SystemAddress":76738793810, "StarType":"K", "StellarMass":0.832, "WasDiscovered":true, "WasMapped":false }"#;
    const PLANET: &str = r#"{ "timestamp":"2026-10-12T22:02:00Z", "event":"Scan", "ScanType":"Detailed", "BodyName":"Oochont LI-N c9-0 A 1", "BodyID":5, "StarSystem":"Oochont LI-N c9-0", "SystemAddress":76738793810, "PlanetClass":"High metal content body", "MassEM":1.0, "WasDiscovered":true, "WasMapped":true }"#;
    const MAP: &str = r#"{ "timestamp":"2026-10-12T22:05:00Z", "event":"SAAScanComplete", "BodyName":"Oochont LI-N c9-0 A 1", "SystemAddress":76738793810, "BodyID":5, "ProbesUsed":3, "EfficiencyTarget":4 }"#;

    #[test]
    fn mapping_adds_to_the_system_and_a_rescan_keeps_it() {
        let scanned = replay(&[LOAD, STAR, PLANET]);
        let mapped = replay(&[LOAD, STAR, PLANET, MAP, PLANET]);
        assert_eq!(scanned.system_count(), 1);
        assert_eq!(scanned.total(), 1215 + 15_117);
        // HMC base 15,117 × 3⅓, +30 % (Odyssey), ×1.25 (efficient), plus the star.
        assert_eq!(mapped.total(), 1215 + 81_886);
    }

    #[test]
    fn selling_or_dying_clears_it() {
        let sell = r#"{ "timestamp":"2026-10-13T08:00:00Z", "event":"MultiSellExplorationData", "Discovered":[ { "SystemName":"Oochont LI-N c9-0", "NumBodies":2 } ], "BaseValue":1, "Bonus":0, "TotalEarnings":1 }"#;
        assert_eq!(replay(&[LOAD, STAR, PLANET, sell]).total(), 0);
        let died = r#"{ "timestamp":"2026-10-13T08:00:00Z", "event":"Died" }"#;
        assert_eq!(replay(&[LOAD, STAR, PLANET, died]).system_count(), 0);
        // Data scanned after a sale counts again.
        assert_eq!(replay(&[LOAD, STAR, sell, PLANET]).total(), 15_117);
    }

    const BIO: &str = r#"{ "timestamp":"2026-10-13T07:11:39Z", "event":"ScanOrganic", "ScanType":"Analyse", "Genus":"$Codex_Ent_Stratum_Genus_Name;", "Species":"$Codex_Ent_Stratum_07_Name;", "SystemAddress":76738793810, "Body":5 }"#;

    #[test]
    fn organic_data_counts_until_sold_with_the_footfall_bonus() {
        // The planet's scan says nobody has landed: the first-footfall bonus applies.
        let untrodden = PLANET.replace(r#""WasMapped":true"#, r#""WasMapped":true, "WasFootfalled":false"#);
        let d = replay(&[LOAD, &untrodden, BIO]);
        assert_eq!(d.bio_total(), 19_010_800 * 5);
        assert_eq!(d.bio_on(76738793810, 5).count(), 1);
        // Without a scan saying so, no bonus.
        assert_eq!(replay(&[LOAD, PLANET, BIO]).bio_total(), 19_010_800);
        // Selling the exploration data keeps the organic data, and the other way round.
        let sell_uc = r#"{ "timestamp":"2026-10-13T08:00:00Z", "event":"SellExplorationData", "Systems":[ "Oochont LI-N c9-0" ], "BaseValue":1, "Bonus":0, "TotalEarnings":1 }"#;
        assert_eq!(replay(&[LOAD, PLANET, BIO, sell_uc]).bio_count(), 1);
        let sell_bio = r#"{ "timestamp":"2026-10-13T08:00:00Z", "event":"SellOrganicData", "MarketID":1, "BioData":[ { "Genus":"$Codex_Ent_Stratum_Genus_Name;", "Species":"$Codex_Ent_Stratum_07_Name;", "Value":19010800, "Bonus":0 } ] }"#;
        let sold = replay(&[LOAD, PLANET, BIO, BIO, sell_bio]);
        assert_eq!((sold.bio_count(), sold.total()), (0, 15_117));
        let died = r#"{ "timestamp":"2026-10-13T08:00:00Z", "event":"Died" }"#;
        assert_eq!(replay(&[LOAD, PLANET, BIO, died]).bio_count(), 0);
    }

    #[test]
    fn nav_beacon_data_is_not_counted() {
        let beacon = STAR.replace("AutoScan", "NavBeaconDetail");
        assert_eq!(replay(&[LOAD, &beacon]).system_count(), 0);
    }
}
