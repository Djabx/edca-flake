//! Credit-value estimates for exploration and exobiology data.
//!
//! Elite's `Scan` journal events don't carry a value, so body values come from the
//! community's formula (Frontier forums, "Exploration value formulae", as EDDiscovery's
//! `EstimatedValues.cs` implements it). Replayed against a commander's own sales it pays to
//! the credit for plain scans and within about 1 % on a long expedition with mapping.
//! Exobiology payouts are not in the journal until the sale, so they come from a
//! per-species table. Both are *estimates* and are labelled as such in the UI.

use crate::journal::Scan;

/// Frontier's mass exponent constant for the body-value formula.
const Q: f64 = 0.56591828;
/// First discovery multiplies everything the body is worth.
const FIRST_DISCOVERY: f64 = 2.6;
/// Mapping multipliers: the commander was first to discover and map it, only first to
/// map it, or neither.
const MAP_FIRST_BOTH: f64 = 3.699622554;
const MAP_FIRST_ONLY: f64 = 8.0956;
const MAP_KNOWN: f64 = 3.3333333;
/// Using no more probes than the body's efficiency target.
const MAP_EFFICIENT: f64 = 1.25;

/// Base `k` for a star, by its `StarType`.
fn star_k(star_type: &str) -> f64 {
    match star_type {
        "SupermassiveBlackHole" => 33.5678,
        t if t.starts_with('D') => 14_057.0, // white dwarfs (D, DA, DAB, …)
        "N" | "H" => 22_628.0,               // neutron stars, black holes
        _ => 1_200.0,
    }
}

/// Base `k` for a planet, by `PlanetClass`, with the terraforming bonus folded in.
fn planet_k(class: &str, terraformable: bool) -> f64 {
    let tf = |bonus: f64| if terraformable { bonus } else { 0.0 };
    match class.to_ascii_lowercase().as_str() {
        "metal rich body" => 21_790.0 + tf(65_631.0),
        "ammonia world" => 96_932.0,
        "sudarsky class i gas giant" => 1_656.0,
        "sudarsky class ii gas giant" | "high metal content body" => 9_654.0 + tf(100_677.0),
        "water world" => 64_831.0 + tf(116_295.0),
        // Always terraformable as far as the payout goes.
        "earthlike body" => 64_831.0 + 116_295.0,
        _ => 300.0 + tf(93_328.0),
    }
}

/// What a scanned body's data sells for at Universal Cartographics, given the scan and
/// whether the commander maps it ([`ScanValue::credits`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScanValue {
    /// The detailed-scan value without first-discovery or mapping bonus.
    base: f64,
    /// A planet or moon; stars can't be mapped, and belt clusters are worth nothing.
    mappable: bool,
    was_discovered: Option<bool>,
    was_mapped: Option<bool>,
    /// Scanned in an Odyssey client, where mapping pays 30 % more (at least 555 Cr).
    odyssey: bool,
}

impl ScanValue {
    pub fn of(e: &Scan, odyssey: bool) -> Self {
        let (base, mappable) = if let Some(star) = e.star_type.as_deref().filter(|s| !s.is_empty()) {
            let k = star_k(star);
            (k + e.stellar_mass.unwrap_or(1.0) as f64 * k / 66.25, false)
        } else if let Some(class) = e.planet_class.as_deref().filter(|c| !c.is_empty()) {
            let terraformable = e
                .terraform_state
                .as_deref()
                .is_some_and(|s| !s.is_empty() && !s.eq_ignore_ascii_case("Not Terraformable"));
            let k = planet_k(class, terraformable);
            let mass = e.mass_em.unwrap_or(1.0).max(0.0) as f64;
            ((k + k * Q * mass.powf(0.2)).max(500.0), true)
        } else {
            (0.0, false)
        };
        Self {
            base,
            mappable,
            was_discovered: e.was_discovered,
            // Both flags came with 3.3: a scan without WasDiscovered has no WasMapped either.
            was_mapped: e.was_discovered.map(|_| e.was_mapped),
            odyssey,
        }
    }

    /// Whether mapping the body pays (planets and moons).
    pub fn mappable(&self) -> bool {
        self.mappable
    }

    /// Credits the data sells for; `mapped` is the commander's own surface map, with
    /// whether it met the probe efficiency target.
    pub fn credits(&self, mapped: Option<bool>) -> i64 {
        let first_discovery = self.was_discovered == Some(false);
        let first_map = self.was_mapped == Some(false);
        let map = |mult: f64, efficient: bool| {
            let v = self.base * mult;
            let v = if self.odyssey { v + (v * 0.3).max(555.0) } else { v };
            if efficient { v * MAP_EFFICIENT } else { v }
        };
        let value = match mapped.filter(|_| self.mappable) {
            // Pre-charted bodies (undiscovered, yet mapped) pay like known ones.
            None if first_discovery && self.was_mapped == Some(true) => self.base,
            Some(eff) if first_discovery && self.was_mapped == Some(true) => map(MAP_KNOWN, eff),
            Some(eff) if first_discovery && first_map => map(MAP_FIRST_BOTH, eff) * FIRST_DISCOVERY,
            Some(eff) if first_map => map(MAP_FIRST_ONLY, eff),
            Some(eff) => map(MAP_KNOWN, eff),
            None if first_discovery => self.base * FIRST_DISCOVERY,
            None => self.base,
        };
        // Sales pay the rounded figure (a 1,203.9 Cr star sold for 1,204).
        value.round() as i64
    }
}

/// Estimates what a scanned body's data is worth unmapped (see [`ScanValue`]), assuming an
/// Odyssey-era scan.
pub fn exploration_value(e: &Scan) -> i64 {
    ScanValue::of(e, true).credits(None)
}

/// The first commander to set foot on a body earns four times a sample's value on top.
pub const FIRST_FOOTFALL: i64 = 5;

/// Vista Genomics payout (in credits) for one analysed organic species, without the
/// first-footfall bonus, by its codex symbols (`Species` / `Genus` of `ScanOrganic`, e.g.
/// `$Codex_Ent_Stratum_07_Name;`). The symbols are the same in every game language; the
/// figures are the current ones, checked against `SellOrganicData` payouts. `None` for a
/// species this table doesn't know.
pub fn exobio_value(species: &str, genus: &str) -> Option<i64> {
    let key = |s: &str| s.trim_start_matches("$Codex_Ent_").trim_end_matches(';').trim_end_matches("_Name").to_string();
    let value = match key(species).as_str() {
        "Aleoids_01" => 7_252_500,  // Arcus
        "Aleoids_02" => 6_284_600,  // Coronamus
        "Aleoids_03" => 3_385_200,  // Spica
        "Aleoids_04" => 3_385_200,  // Laminiae
        "Aleoids_05" => 12_934_900, // Gravis
        "Bacterial_01" => 1_000_000, // Aurasus
        "Bacterial_02" => 5_289_900, // Nebulus
        "Bacterial_03" => 4_934_500, // Scopulum
        "Bacterial_04" => 1_000_000, // Acies
        "Bacterial_05" => 1_000_000, // Vesicula
        "Bacterial_06" => 1_658_500, // Alcyoneum
        "Bacterial_07" => 1_949_000, // Tela
        "Bacterial_08" => 8_418_000, // Informem
        "Bacterial_09" => 7_774_700, // Volu
        "Bacterial_10" => 1_152_500, // Bullaris
        "Bacterial_11" => 4_638_900, // Omentum
        "Bacterial_12" => 1_689_800, // Cerbrus
        "Bacterial_13" => 3_897_000, // Verrata
        "Cactoid_01" => 3_667_600,  // Cortexum
        "Cactoid_02" => 2_483_600,  // Lapis
        "Cactoid_03" => 16_202_800, // Vermis
        "Cactoid_04" => 3_667_600,  // Pullulanta
        "Cactoid_05" => 2_483_600,  // Peperatis
        "Clypeus_01" => 8_418_000,  // Lacrimam
        "Clypeus_02" => 11_873_200, // Margaritus
        "Clypeus_03" => 16_202_800, // Speculumi
        "Conchas_01" => 4_572_400,  // Renibus
        "Conchas_02" => 7_774_700,  // Aureolas
        "Conchas_03" => 2_352_400,  // Labiata
        "Conchas_04" => 16_777_600, // Biconcavis
        "Electricae_01" => 6_284_600, // Pluma
        "Electricae_02" => 6_284_600, // Radialem
        "Fonticulus_01" => 19_010_800, // Segmentatus
        "Fonticulus_02" => 1_000_000,  // Campestris
        "Fonticulus_03" => 9_701_800,  // Upupam
        "Fonticulus_04" => 6_017_400,  // Lapida
        "Fonticulus_05" => 20_000_000, // Fluctus
        "Fonticulus_06" => 3_928_300,  // Digitos
        "Shrubs_01" => 1_808_900,  // Frutexa Flabellum
        "Shrubs_02" => 7_774_700,  // Acus
        "Shrubs_03" => 1_632_500,  // Metallicum
        "Shrubs_04" => 10_326_000, // Flammasis
        "Shrubs_05" => 1_632_500,  // Fera
        "Shrubs_06" => 5_988_000,  // Sponsae
        "Shrubs_07" => 1_639_800,  // Collum
        "Fumerolas_01" => 6_284_600,  // Carbosis
        "Fumerolas_02" => 16_202_800, // Extremus
        "Fumerolas_03" => 7_500_900,  // Nitris
        "Fumerolas_04" => 6_284_600,  // Aquatis
        "Fungoids_01" => 1_670_100, // Setisis
        "Fungoids_02" => 2_680_300, // Stabitis
        "Fungoids_03" => 3_703_200, // Bullarum
        "Fungoids_04" => 3_330_300, // Gelata
        "Osseus_01" => 4_027_800,  // Fractus
        "Osseus_02" => 12_934_900, // Discus
        "Osseus_03" => 2_404_700,  // Spiralis
        "Osseus_04" => 3_156_300,  // Pumice
        "Osseus_05" => 1_483_000,  // Cornibus
        "Osseus_06" => 9_739_000,  // Pellebantus
        "Recepta_01" => 12_934_900, // Umbrux
        "Recepta_02" => 16_202_800, // Deltahedronix
        "Recepta_03" => 14_313_700, // Conditivus
        "Stratum_01" => 2_448_900,  // Excutitus
        "Stratum_02" => 1_362_000,  // Paleas
        "Stratum_03" => 2_788_300,  // Laminamus
        "Stratum_04" => 2_448_900,  // Araneamus
        "Stratum_05" => 1_362_000,  // Limaxus
        "Stratum_06" => 16_202_800, // Cucumisis
        "Stratum_07" => 19_010_800, // Tectonicas
        "Stratum_08" => 2_637_500,  // Frigus
        "Tubus_01" => 2_415_500,  // Conifer
        "Tubus_02" => 5_727_600,  // Sororibus
        "Tubus_03" => 11_873_200, // Cavas
        "Tubus_04" => 2_637_500,  // Rosarium
        "Tubus_05" => 7_774_700,  // Compagibus
        "Tussocks_01" => 5_853_800,  // Pennata
        "Tussocks_02" => 3_277_700,  // Ventusa
        "Tussocks_03" => 1_849_000,  // Ignis
        "Tussocks_04" => 1_766_600,  // Cultro
        "Tussocks_05" => 1_766_600,  // Catena
        "Tussocks_06" => 1_000_000,  // Pennatis
        "Tussocks_07" => 4_447_100,  // Serrati
        "Tussocks_08" => 3_252_500,  // Albata
        "Tussocks_09" => 1_000_000,  // Propagito
        "Tussocks_10" => 1_766_600,  // Divisa
        "Tussocks_11" => 3_472_400,  // Caputus
        "Tussocks_12" => 7_774_700,  // Triticum
        "Tussocks_13" => 19_010_800, // Stigmasis
        "Tussocks_14" => 14_313_700, // Virgam
        "Tussocks_15" => 7_025_800,  // Capillum
        // Genera whose species (colour variants) all pay the same.
        _ => match key(genus).as_str() {
            "Brancae" => 1_593_700,           // brain trees
            "Sphere" => 1_499_900,            // anemones
            "Tube" => 1_514_500,              // sinuous tubers
            "Cone" => 1_471_900,              // bark mounds
            "Ground_Struct_Ice" => 1_628_800, // crystalline shards
            "Vents" => 1_628_800,             // amphora plant
            // Radicoida Unica pays per analysis, a fraction of the others.
            "Ingensradices_Genus" => 119_037,
            _ => return None,
        },
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(json: &str) -> Scan {
        serde_json::from_str(json).expect("valid Scan json")
    }

    #[test]
    fn first_discovered_elw_is_worth_more_than_undiscovered() {
        let undiscovered = scan(
            r#"{"timestamp":"t","event":"Scan","BodyName":"b","BodyID":1,"SystemAddress":1,
                "PlanetClass":"Earthlike body","MassEM":1.0,"WasDiscovered":true,"WasMapped":false}"#,
        );
        let first = scan(
            r#"{"timestamp":"t","event":"Scan","BodyName":"b","BodyID":1,"SystemAddress":1,
                "PlanetClass":"Earthlike body","MassEM":1.0,"WasDiscovered":false,"WasMapped":false}"#,
        );
        let v_undisc = exploration_value(&undiscovered);
        let v_first = exploration_value(&first);
        // An ELW FSS scan is worth roughly 100k; first discovery applies a ~2.6× bonus.
        assert!(v_undisc > 50_000, "ELW value too low: {v_undisc}");
        assert!(v_first > v_undisc, "first discovery should pay more");
    }

    #[test]
    fn belt_cluster_and_barycentre_have_no_value() {
        let belt = scan(
            r#"{"timestamp":"t","event":"Scan","BodyName":"b","BodyID":1,"SystemAddress":1}"#,
        );
        assert_eq!(exploration_value(&belt), 0);
    }

    /// Prices of single stars the author's commander sold, from `MultiSellExplorationData`.
    #[test]
    fn stars_pay_what_universal_cartographics_paid() {
        for (star_type, mass, paid) in [("M", 0.215, 1204), ("K", 0.832, 1215), ("K", 0.438, 1208), ("M", 0.258, 1205)] {
            let s = scan(&format!(
                r#"{{"timestamp":"t","event":"Scan","BodyName":"b","BodyID":1,"SystemAddress":1,
                    "StarType":"{star_type}","StellarMass":{mass},"WasDiscovered":true,"WasMapped":false}}"#
            ));
            assert_eq!(ScanValue::of(&s, true).credits(None), paid, "{star_type} {mass}");
        }
    }

    #[test]
    fn earthlike_worlds_always_get_the_terraforming_bonus() {
        let elw = scan(
            r#"{"timestamp":"t","event":"Scan","BodyName":"b","BodyID":1,"SystemAddress":1,
                "PlanetClass":"Earthlike body","MassEM":1.0,"TerraformState":"","WasDiscovered":true,"WasMapped":false}"#,
        );
        // (64,831 + 116,295) × (1 + 0.56591828)
        assert_eq!(ScanValue::of(&elw, false).credits(None), 283_629);
    }

    #[test]
    fn mapping_and_first_discovery_multiply_the_base() {
        let hmc = |discovered: bool, mapped: bool| {
            scan(&format!(
                r#"{{"timestamp":"t","event":"Scan","BodyName":"b","BodyID":1,"SystemAddress":1,
                    "PlanetClass":"High metal content body","MassEM":1.0,"WasDiscovered":{discovered},"WasMapped":{mapped}}}"#
            ))
        };
        let base = 9_654.0 * (1.0 + Q);
        let v = |s: &Scan, odyssey, mapped| ScanValue::of(s, odyssey).credits(mapped);
        let known = hmc(true, true);
        assert_eq!(v(&known, false, None), base.round() as i64);
        assert_eq!(v(&known, false, Some(false)), (base * MAP_KNOWN).round() as i64);
        assert_eq!(v(&known, false, Some(true)), (base * MAP_KNOWN * MAP_EFFICIENT).round() as i64);
        // Odyssey adds 30 % to the mapped value.
        assert_eq!(v(&known, true, Some(false)), (base * MAP_KNOWN * 1.3).round() as i64);
        assert_eq!(v(&hmc(true, false), false, Some(false)), (base * MAP_FIRST_ONLY).round() as i64);
        let fresh = hmc(false, false);
        assert_eq!(v(&fresh, false, None), (base * FIRST_DISCOVERY).round() as i64);
        assert_eq!(v(&fresh, false, Some(false)), (base * MAP_FIRST_BOTH * FIRST_DISCOVERY).round() as i64);
        // Undiscovered but mapped: charted before discovery tags, pays like a known body.
        assert_eq!(v(&hmc(false, true), false, None), base.round() as i64);
    }

    #[test]
    fn stars_cannot_be_mapped() {
        let star = scan(
            r#"{"timestamp":"t","event":"Scan","BodyName":"b","BodyID":1,"SystemAddress":1,
                "StarType":"G","StellarMass":1.0,"WasDiscovered":true,"WasMapped":false}"#,
        );
        let v = ScanValue::of(&star, true);
        assert!(!v.mappable());
        assert_eq!(v.credits(Some(true)), v.credits(None));
    }

    #[test]
    fn exobio_by_codex_symbol() {
        assert_eq!(exobio_value("$Codex_Ent_Stratum_07_Name;", "$Codex_Ent_Stratum_Genus_Name;"), Some(19_010_800));
        assert_eq!(exobio_value("$Codex_Ent_Tubus_02_Name;", "$Codex_Ent_Tubus_Genus_Name;"), Some(5_727_600));
        // Brain trees pay the same in every colour.
        assert_eq!(exobio_value("$Codex_Ent_SeedEFGH_01_Name;", "$Codex_Ent_Brancae_Name;"), Some(1_593_700));
        assert_eq!(exobio_value("$Codex_Ent_Foo_01_Name;", "$Codex_Ent_Foo_Genus_Name;"), None);
    }
}
