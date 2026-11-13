//! What the commander has found on each body, over every journal: organic species (codex
//! entries and samples taken) and geological sites (codex entries), so a body's details can
//! show what was already logged there on an earlier visit. Rebuilt from the whole journal
//! folder ([`crate::journal_follow`]).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use edcas_common::exobiology::species_of_codex;
use edcas_common::exploration::exobio_value;
use serde_json::Value;

use crate::journal_follow::{JournalFollower, JournalLog};

#[derive(Clone, Default)]
pub struct BodyFinds {
    /// By system address, then body id; in the order first found.
    systems: HashMap<i64, HashMap<i32, Vec<Find>>>,
    pub loaded: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FindKind {
    Biological,
    Geological,
}

#[derive(Clone, Debug)]
pub struct Find {
    pub kind: FindKind,
    /// The codex variant, e.g. "Stratum Tectonicas - Green", else the species.
    pub name: String,
    /// Codex variant symbol (`$Codex_Ent_Stratum_07_M_Name;`), if known.
    variant: Option<String>,
    /// Species symbol of an organic, once sampled.
    pub species: Option<String>,
    /// Localised species name of an organic, once sampled.
    species_name: Option<String>,
    /// Samples taken: 1 logged, 2 sampled, 3 analysed.
    pub samples: u8,
    /// Vista Genomics payout without the first-footfall bonus: the sampled species', else
    /// the one a codex entry's variant belongs to (a ship or SRV scan names the species).
    pub value: Option<i64>,
    /// The codex entry was new to the commander's codex (in that region).
    pub new_entry: bool,
    /// When it was first found (journal timestamp).
    pub first_seen: String,
}

impl Find {
    /// Codex symbol of the genus, when the species is known.
    pub fn genus(&self) -> Option<&'static str> {
        let symbol = self.species.as_deref().or(self.variant.as_deref())?;
        edcas_common::exobiology::species_of_codex(symbol).map(|s| s.genus.as_str())
    }

    /// Whether a codex entry or sample named `variant` / `name` is this find.
    fn is(&self, variant: Option<&str>, name: &str) -> bool {
        if let (Some(a), Some(b)) = (self.variant.as_deref(), variant) {
            return a == b;
        }
        // Older samples carry no variant: "Stratum Tectonicas - Green" is "Stratum Tectonicas".
        let species = self.species_name.as_deref().unwrap_or(&self.name);
        !name.is_empty() && (self.name.starts_with(name) || name.starts_with(species))
    }
}

impl BodyFinds {
    /// What was found on one body, in the order found.
    pub fn on(&self, address: i64, body_id: i32) -> &[Find] {
        self.systems.get(&address).and_then(|s| s.get(&body_id)).map_or(&[], Vec::as_slice)
    }

    /// Every body of a system with finds.
    pub fn in_system(&self, address: i64) -> impl Iterator<Item = (i32, &[Find])> {
        self.systems.get(&address).into_iter().flatten().map(|(id, f)| (*id, f.as_slice()))
    }

    fn body(&mut self, v: &Value, id_field: &str) -> Option<&mut Vec<Find>> {
        let address = v["SystemAddress"].as_i64()?;
        let body_id = i32::try_from(v[id_field].as_i64()?).ok()?;
        Some(self.systems.entry(address).or_default().entry(body_id).or_default())
    }
}

impl JournalLog for BodyFinds {
    const NAME: &'static str = "body-finds";
    const EVENTS: &'static [&'static [u8]] = &[b"\"event\":\"CodexEntry\"", b"\"event\":\"ScanOrganic\""];

    fn apply(&mut self, v: &Value) {
        let str_of = |k: &str| v[k].as_str().filter(|s| !s.is_empty());
        let ts = str_of("timestamp").unwrap_or_default().to_string();
        match v["event"].as_str() {
            Some("CodexEntry") => {
                let kind = match str_of("SubCategory") {
                    Some("$Codex_SubCategory_Organic_Structures;") => FindKind::Biological,
                    Some("$Codex_SubCategory_Geology_and_Anomalies;") => FindKind::Geological,
                    _ => return,
                };
                let variant = str_of("Name");
                let name = str_of("Name_Localised").or(variant).unwrap_or_default().to_string();
                let new_entry = v["IsNewEntry"].as_bool().unwrap_or(false);
                let value = variant.and_then(species_of_codex).and_then(|s| exobio_value(&s.symbol, &s.genus));
                // Space anomalies carry no body.
                let Some(finds) = self.body(v, "BodyID") else { return };
                if let Some(f) = finds.iter_mut().find(|f| f.kind == kind && f.is(variant, &name)) {
                    f.new_entry |= new_entry;
                    f.variant = f.variant.take().or(variant.map(str::to_string));
                    f.value = f.value.or(value);
                    f.name = name;
                } else {
                    finds.push(Find {
                        kind,
                        name,
                        variant: variant.map(str::to_string),
                        species: None,
                        species_name: None,
                        samples: 0,
                        value,
                        new_entry,
                        first_seen: ts,
                    });
                }
            }
            Some("ScanOrganic") => {
                let samples = match str_of("ScanType") {
                    Some("Log") => 1,
                    Some("Sample") => 2,
                    Some("Analyse") => 3,
                    _ => return,
                };
                let Some(species) = str_of("Species") else { return };
                let species_name = str_of("Species_Localised").unwrap_or(species).to_string();
                let variant = str_of("Variant");
                let value = exobio_value(species, str_of("Genus").unwrap_or_default());
                let Some(finds) = self.body(v, "Body") else { return };
                let i = match finds.iter().position(|f| f.kind == FindKind::Biological && f.is(variant, &species_name)) {
                    Some(i) => i,
                    None => {
                        let name = str_of("Variant_Localised").unwrap_or(&species_name).to_string();
                        finds.push(Find {
                            kind: FindKind::Biological,
                            name,
                            variant: variant.map(str::to_string),
                            species: None,
                            species_name: None,
                            samples: 0,
                            value: None,
                            new_entry: false,
                            first_seen: ts,
                        });
                        finds.len() - 1
                    }
                };
                let f = &mut finds[i];
                // Once analysed it stays analysed: a later round of samples doesn't undo it.
                f.samples = f.samples.max(samples);
                f.species = Some(species.to_string());
                f.species_name = Some(species_name);
                f.value = value;
                if f.variant.is_none() {
                    f.variant = variant.map(str::to_string);
                }
            }
            _ => {}
        }
    }

    fn set_loaded(&mut self) {
        self.loaded = true;
    }

    fn summary(&self) -> String {
        let bodies: usize = self.systems.values().map(HashMap::len).sum();
        format!("finds on {bodies} bodies in {} systems", self.systems.len())
    }
}

/// The body-finds scan of the configured journal folder.
#[derive(Default)]
pub struct Finds {
    scanner: Option<JournalFollower<BodyFinds>>,
}

impl Finds {
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

    pub fn data(&self) -> Arc<BodyFinds> {
        self.scanner.as_ref().map(JournalFollower::snapshot).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal_follow::feed;

    fn replay(lines: &[&str]) -> BodyFinds {
        let mut d = BodyFinds::default();
        feed(&mut d, (lines.join("\n") + "\n").as_bytes());
        d
    }

    const CODEX: &str = r#"{ "timestamp":"2026-10-13T07:05:51Z", "event":"CodexEntry", "EntryID":2420703, "Name":"$Codex_Ent_Stratum_07_M_Name;", "Name_Localised":"Stratum Tectonicas - Green", "SubCategory":"$Codex_SubCategory_Organic_Structures;", "Category":"$Codex_Category_Biology;", "System":"Oochont JC-G b16-1", "SystemAddress":2806089327241, "BodyID":41, "IsNewEntry":true }"#;
    const LOG: &str = r#"{ "timestamp":"2026-10-13T07:05:52Z", "event":"ScanOrganic", "ScanType":"Log", "Genus":"$Codex_Ent_Stratum_Genus_Name;", "Genus_Localised":"Stratum", "Species":"$Codex_Ent_Stratum_07_Name;", "Species_Localised":"Stratum Tectonicas", "Variant":"$Codex_Ent_Stratum_07_M_Name;", "Variant_Localised":"Stratum Tectonicas - Green", "SystemAddress":2806089327241, "Body":41 }"#;
    const GEO: &str = r#"{ "timestamp":"2026-10-13T07:30:00Z", "event":"CodexEntry", "EntryID":1400114, "Name":"$Codex_Ent_IceGeysers_WaterMagma_Name;", "Name_Localised":"Water Magma Ice Geyser", "SubCategory":"$Codex_SubCategory_Geology_and_Anomalies;", "Category":"$Codex_Category_Biology;", "System":"Oochont JC-G b16-1", "SystemAddress":2806089327241, "BodyID":41 }"#;

    #[test]
    fn codex_entry_and_samples_are_one_find() {
        let analyse = LOG.replace(r#""Log""#, r#""Analyse""#);
        let d = replay(&[CODEX, LOG, &analyse, CODEX, GEO]);
        let finds = d.on(2806089327241, 41);
        assert_eq!(finds.len(), 2);
        assert_eq!(finds[0].name, "Stratum Tectonicas - Green");
        assert_eq!((finds[0].samples, finds[0].value, finds[0].new_entry), (3, Some(19_010_800), true));
        assert_eq!((finds[1].kind, finds[1].samples), (FindKind::Geological, 0));
    }

    #[test]
    fn a_ship_or_srv_scan_values_the_variants_species() {
        let finds = replay(&[CODEX]);
        assert_eq!(finds.on(2806089327241, 41)[0].value, Some(19_010_800));
    }

    #[test]
    fn samples_without_a_variant_match_the_codex_entry() {
        let old = LOG.replace(r#""Variant":"$Codex_Ent_Stratum_07_M_Name;", "Variant_Localised":"Stratum Tectonicas - Green", "#, "");
        assert_eq!(replay(&[CODEX, &old]).on(2806089327241, 41).len(), 1);
        // Sampled first, the codex entry following.
        assert_eq!(replay(&[&old, CODEX]).on(2806089327241, 41).len(), 1);
    }

    #[test]
    fn space_anomalies_have_no_body() {
        let cloud = GEO.replace(r#", "BodyID":41"#, "");
        assert_eq!(replay(&[&cloud]).in_system(2806089327241).count(), 0);
    }
}
