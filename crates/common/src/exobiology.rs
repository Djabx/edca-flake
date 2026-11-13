//! Which exobiology a body can carry, from its scan and its system's stars.
//!
//! The habitats per species come from `data/exobiology.json`, which
//! `scripts/exobiology/build.py` derives from the biological codex finds commanders shared,
//! joined to their bodies' scans: per atmosphere and body class the gravity, temperature and
//! pressure a species was found at, whether it needs volcanism or a sulphur dioxide trace,
//! and the stars it grows under. A species can grow on a body that fits any of its habitats.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

const DATA: &str = include_str!("../data/exobiology.json");

#[derive(Deserialize)]
struct Data {
    genera: BTreeMap<String, String>,
    species: Vec<Species>,
    /// The colour of each codex variant, as commanders' games named it.
    #[serde(default)]
    variants: BTreeMap<String, String>,
}

#[derive(Deserialize)]
pub struct Species {
    /// Codex symbol, e.g. `$Codex_Ent_Stratum_07_Name;`.
    pub symbol: String,
    /// Genus codex symbol, e.g. `$Codex_Ent_Stratum_Genus_Name;`.
    pub genus: String,
    pub name: String,
    /// How many finds the habitats were derived from.
    pub finds: u32,
    habitats: Vec<Habitat>,
    /// Classes of the star the body orbits it was never found under, though expected often.
    #[serde(default)]
    not_under: Vec<String>,
    /// The only classes it grows under, of the star the body orbits or the main star.
    only_under: Option<OnlyUnder>,
    /// Found only within this distance of systems it was found in before.
    near: Option<Near>,
    /// Found only in some places, which the rules can't tell apart.
    #[serde(default)]
    location_bound: bool,
}

/// Where a species was found: one atmosphere on one body class, with the ranges of its finds.
#[derive(Deserialize)]
pub struct Habitat {
    atmosphere: String,
    body: String,
    /// How many finds it was derived from.
    finds: u32,
    /// m/s².
    gravity: Option<[f32; 2]>,
    /// K.
    temperature: Option<[f32; 2]>,
    /// Pa.
    pressure: Option<[f32; 2]>,
    volcanism: Option<Volcanism>,
    /// Least sulphur dioxide share of the atmosphere, in percent.
    so2_min: Option<f32>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Volcanism {
    /// "none": only without volcanism.
    None(NoVolcanism),
    /// Only with one of these kinds ("water geysers", "rocky magma"…).
    OneOf(Vec<String>),
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum NoVolcanism {
    None,
}

#[derive(Deserialize)]
struct OnlyUnder {
    /// "parent" (the star the body orbits) or "main".
    star: String,
    classes: Vec<String>,
}

#[derive(Deserialize)]
struct Near {
    radius_ly: f32,
    systems: Vec<[f32; 3]>,
}

fn data() -> &'static Data {
    static PARSED: OnceLock<Data> = OnceLock::new();
    PARSED.get_or_init(|| serde_json::from_str(DATA).expect("data/exobiology.json is valid"))
}

/// The name of a genus by its codex symbol, if any species of it is known.
pub fn genus_name(genus: &str) -> Option<&'static str> {
    data().genera.get(genus).map(String::as_str)
}

/// Every species with known habitats.
pub fn all_species() -> &'static [Species] {
    &data().species
}

/// The genera with their names, by codex symbol.
pub fn all_genera() -> impl Iterator<Item = (&'static str, &'static str)> {
    data().genera.iter().map(|(k, v)| (k.as_str(), v.as_str()))
}

/// A colour a species grows in, and what decides it.
pub struct Variant {
    /// Codex symbol, e.g. `$Codex_Ent_Stratum_07_M_Name;`.
    pub symbol: &'static str,
    pub colour: &'static str,
    /// The star class (`M`, `TTS`…) or crust material (`Polonium`) that gives this colour.
    pub code: &'static str,
}

impl Variant {
    /// Whether the colour comes from the star (a class code) rather than the crust (an element).
    pub fn from_star(&self) -> bool {
        self.code.len() <= 3 && self.code.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && self.code.chars().skip(1).all(|c| c.is_ascii_uppercase() || c == 'e')
    }
}

/// The species a codex entry is a variant of: `$Codex_Ent_Stratum_07_K_Name;` → Stratum
/// Tectonicas. Older species are their own variants.
pub fn species_of_codex(entry: &str) -> Option<&'static Species> {
    let species = data().species.iter().find(|s| s.symbol == entry);
    species.or_else(|| {
        // Odyssey variants add a colour code before "_Name;".
        let base = entry.strip_suffix("_Name;")?;
        let (head, _colour) = base.rsplit_once('_')?;
        let symbol = format!("{head}_Name;");
        data().species.iter().find(|s| s.symbol == symbol)
    })
}

/// What the prediction needs to know of a body and its system, in journal units.
#[derive(Clone, Debug, Default)]
pub struct BodyFacts {
    pub planet_class: String,
    /// `AtmosphereType`; empty or "None" for an airless body.
    pub atmosphere_type: String,
    /// Sulphur dioxide share of the atmosphere, in percent.
    pub so2_percent: f32,
    /// m/s².
    pub surface_gravity: f32,
    /// K.
    pub surface_temperature: f32,
    /// Pa; 0 when unknown.
    pub surface_pressure: f32,
    /// The scan's `Volcanism` text; empty for none.
    pub volcanism: String,
    /// `StarType` of the star the body orbits; empty when not scanned.
    pub parent_star: String,
    /// `StarType` of the system's main star; empty when not scanned.
    pub main_star: String,
    /// The system's position (ly), when known.
    pub position: Option<[f32; 3]>,
}

/// A species the body's conditions allow.
pub struct Possible {
    pub species: &'static Species,
    /// It grows only in some places: near earlier finds (unchecked without the system's
    /// position) or somewhere the rules can't tell.
    pub depends_on_location: bool,
}

/// The species `body` can carry, most often found first.
pub fn possible_species(body: &BodyFacts) -> Vec<Possible> {
    let mut out: Vec<Possible> = data()
        .species
        .iter()
        .filter(|s| s.habitats.iter().any(|h| h.holds(body)) && s.stars_allow(body))
        .filter_map(|s| {
            let unchecked_place = match (&s.near, body.position) {
                (Some(near), Some(pos)) => {
                    if !near.systems.iter().any(|p| distance(*p, pos) <= near.radius_ly) {
                        return None;
                    }
                    false
                }
                (Some(_), None) => true,
                (None, _) => false,
            };
            Some(Possible { species: s, depends_on_location: unchecked_place || s.location_bound })
        })
        .collect();
    out.sort_by(|a, b| b.species.finds.cmp(&a.species.finds).then_with(|| a.species.name.cmp(&b.species.name)));
    out
}

impl Species {
    /// Where it was found, most finds first.
    pub fn habitats(&self) -> Vec<&Habitat> {
        let mut out: Vec<&Habitat> = self.habitats.iter().collect();
        out.sort_by_key(|h| std::cmp::Reverse(h.finds));
        out
    }

    /// The colours it grows in, by their codex variants.
    pub fn variants(&self) -> Vec<Variant> {
        let head = self.symbol.trim_end_matches("_Name;");
        data()
            .variants
            .iter()
            .filter_map(|(symbol, colour)| {
                let code = symbol.strip_prefix(head)?.strip_prefix('_')?.strip_suffix("_Name;")?;
                Some(Variant { symbol, colour, code })
            })
            .collect()
    }

    /// The star rules in words: "the star it orbits is never M", "the system's main star is N"…
    pub fn star_needs(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(only) = &self.only_under {
            let star = if only.star == "main" { "the system's main star" } else { "the star it orbits" };
            out.push(format!("{star} is one of: {}", stars_in_words(&only.classes)));
        }
        if !self.not_under.is_empty() {
            out.push(format!("never found around {}", stars_in_words(&self.not_under)));
        }
        out
    }

    /// How far from earlier finds it was ever found, ly; it doesn't spread further.
    pub fn near_radius_ly(&self) -> Option<f32> {
        self.near.as_ref().map(|n| n.radius_ly)
    }

    /// Found only in some places, which the habitats can't tell apart.
    pub fn location_bound(&self) -> bool {
        self.location_bound
    }

    /// Whether this species belongs to the genus with codex symbol `genus`.
    pub fn in_genus(&self, genus: &str) -> bool {
        self.genus == genus
    }

    /// The genus' name.
    pub fn genus_name(&self) -> &'static str {
        genus_name(&self.genus).unwrap_or("")
    }

    /// The atmospheres it was found in, most finds first ("None" for airless).
    pub fn atmospheres(&self) -> Vec<&str> {
        self.ranked(|h| &h.atmosphere)
    }

    /// The body classes it was found on, most finds first.
    pub fn body_classes(&self) -> Vec<&str> {
        self.ranked(|h| &h.body)
    }

    fn ranked<'a>(&'a self, key: impl Fn(&'a Habitat) -> &'a String) -> Vec<&'a str> {
        let mut counts: Vec<(&str, u32)> = Vec::new();
        for h in &self.habitats {
            match counts.iter_mut().find(|(k, _)| *k == key(h).as_str()) {
                Some((_, n)) => *n += h.finds,
                None => counts.push((key(h), h.finds)),
            }
        }
        counts.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        counts.into_iter().map(|(k, _)| k).collect()
    }

    /// The surface gravity it was found at over all habitats, m/s².
    pub fn gravity_range(&self) -> Option<[f32; 2]> {
        span(self.habitats.iter().filter_map(|h| h.gravity))
    }

    /// The surface temperature it was found at over all habitats, K.
    pub fn temperature_range(&self) -> Option<[f32; 2]> {
        span(self.habitats.iter().filter_map(|h| h.temperature))
    }

    /// What else it needs, in words: volcanism, a sulphur dioxide trace, stars, places.
    pub fn needs(&self) -> Vec<String> {
        let mut out = Vec::new();
        let volcanic: Vec<&Vec<String>> = self.habitats.iter().filter_map(|h| match &h.volcanism {
            Some(Volcanism::OneOf(kinds)) => Some(kinds),
            _ => None,
        }).collect();
        if !volcanic.is_empty() && volcanic.len() == self.habitats.len() {
            let mut kinds: Vec<&str> = volcanic.iter().flat_map(|k| k.iter().map(String::as_str)).collect();
            kinds.sort();
            kinds.dedup();
            out.push(format!("volcanism ({})", kinds.join(", ")));
        } else if self.habitats.iter().all(|h| matches!(h.volcanism, Some(Volcanism::None(_)))) {
            out.push("no volcanism".into());
        }
        if self.habitats.iter().all(|h| h.so2_min.is_some()) {
            out.push("sulphur dioxide in the air".into());
        }
        if let Some(only) = &self.only_under {
            let star = if only.star == "main" { "main star" } else { "star" };
            out.push(format!("{star}: {}", stars_in_words(&only.classes)));
        }
        if !self.not_under.is_empty() {
            out.push(format!("never around {}", stars_in_words(&self.not_under)));
        }
        if let Some(near) = &self.near {
            out.push(format!("within {:.0} ly of earlier finds", near.radius_ly));
        } else if self.location_bound {
            out.push("only in some places".into());
        }
        out
    }

    fn stars_allow(&self, b: &BodyFacts) -> bool {
        let parent = star_class(&b.parent_star);
        if !parent.is_empty() && self.not_under.iter().any(|c| c == parent) {
            return false;
        }
        match &self.only_under {
            Some(only) => {
                let star = if only.star == "main" { &b.main_star } else { &b.parent_star };
                let class = star_class(star);
                class.is_empty() || only.classes.iter().any(|c| c == class)
            }
            None => true,
        }
    }
}

impl Habitat {
    /// `AtmosphereType` it was found in; "None" for airless bodies.
    pub fn atmosphere(&self) -> &str {
        &self.atmosphere
    }

    /// The body class (`PlanetClass`).
    pub fn body(&self) -> &str {
        &self.body
    }

    /// How many finds it was derived from.
    pub fn finds(&self) -> u32 {
        self.finds
    }

    /// m/s².
    pub fn gravity(&self) -> Option<[f32; 2]> {
        self.gravity
    }

    /// K.
    pub fn temperature(&self) -> Option<[f32; 2]> {
        self.temperature
    }

    /// Pa.
    pub fn pressure(&self) -> Option<[f32; 2]> {
        self.pressure
    }

    /// The volcanism it needs in words: "none", "water geysers or carbon dioxide geysers";
    /// `None` when it doesn't matter.
    pub fn volcanism(&self) -> Option<String> {
        match &self.volcanism {
            None => None,
            Some(Volcanism::None(_)) => Some("none".into()),
            Some(Volcanism::OneOf(kinds)) => Some(or_list(kinds)),
        }
    }

    /// Least sulphur dioxide share of the atmosphere it was found with, in percent.
    pub fn so2_min(&self) -> Option<f32> {
        self.so2_min
    }

    fn holds(&self, b: &BodyFacts) -> bool {
        let atmosphere = if b.atmosphere_type.is_empty() { "None" } else { &b.atmosphere_type };
        let within = |range: Option<[f32; 2]>, v: f32| range.is_none_or(|[lo, hi]| v >= lo && v <= hi);
        self.atmosphere == atmosphere
            && self.body == b.planet_class
            && within(self.gravity, b.surface_gravity)
            && within(self.temperature, b.surface_temperature)
            // A pressure the scan doesn't give can't rule anything out.
            && (b.surface_pressure <= 0.0 || within(self.pressure, b.surface_pressure))
            && self.so2_min.is_none_or(|min| b.so2_percent >= min)
            && match &self.volcanism {
                None => true,
                Some(Volcanism::None(_)) => volcanism_kind(&b.volcanism).is_empty(),
                Some(Volcanism::OneOf(kinds)) => {
                    kinds.contains(&volcanism_kind(&b.volcanism))
                }
            }
    }
}

/// A star type as the rules group them: giants with their class (K_OrangeGiant → K), dwarfs,
/// Wolf-Rayets and carbon stars by family (DAB → D, WC → W, CN → C), the rest as they are.
pub fn star_class(star_type: &str) -> &str {
    let mut chars = star_type.chars();
    match (chars.next(), chars.next()) {
        (Some('O' | 'B' | 'A' | 'F' | 'G' | 'K' | 'M'), None | Some('_')) => &star_type[..1],
        (Some('D' | 'W' | 'C'), _) => &star_type[..1],
        _ => star_type,
    }
}

/// "minor water geysers volcanism" → "water geysers"; no volcanism → "".
fn volcanism_kind(volcanism: &str) -> String {
    let v = volcanism.trim().to_lowercase();
    if v.is_empty() || v == "no volcanism" {
        return String::new();
    }
    let v = v.strip_prefix("major ").or_else(|| v.strip_prefix("minor ")).unwrap_or(&v);
    v.strip_suffix(" volcanism").unwrap_or(v).to_string()
}

/// Star classes as the reader knows them: "B or K stars", "neutron stars or black holes".
pub fn stars_in_words(classes: &[String]) -> String {
    let mut lettered: Vec<String> = Vec::new();
    let mut named: Vec<String> = Vec::new();
    for c in classes {
        let name = match c.as_str() {
            "N" => "neutron stars",
            "H" => "black holes",
            "D" => "white dwarfs",
            "W" => "Wolf-Rayet stars",
            "C" => "carbon stars",
            "AeBe" => "Herbig Ae/Be stars",
            "TTS" => "T Tauri stars",
            "SupermassiveBlackHole" => "supermassive black holes",
            other => {
                lettered.push(other.to_string());
                continue;
            }
        };
        named.push(name.to_string());
    }
    if !lettered.is_empty() {
        named.insert(0, format!("{} stars", or_list(&lettered)));
    }
    or_list(&named)
}

/// "A", "A or B", "A, B or C".
fn or_list(items: &[String]) -> String {
    match items.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, head)) => format!("{} or {last}", head.join(", ")),
        None => String::new(),
    }
}

fn span(ranges: impl Iterator<Item = [f32; 2]>) -> Option<[f32; 2]> {
    ranges.reduce(|a, b| [a[0].min(b[0]), a[1].max(b[1])])
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(body: &BodyFacts) -> Vec<&'static str> {
        possible_species(body).iter().map(|p| p.species.name.as_str()).collect()
    }

    #[test]
    fn the_data_loads() {
        assert!(all_species().len() > 100);
        assert!(all_species().iter().all(|s| !s.habitats.is_empty() && !s.name.starts_with('$')));
        assert_eq!(genus_name("$Codex_Ent_Stratum_Genus_Name;"), Some("Stratum"));
    }

    #[test]
    fn a_thin_co2_high_metal_world_can_carry_stratum_tectonicas() {
        let body = BodyFacts {
            planet_class: "High metal content body".into(),
            atmosphere_type: "CarbonDioxide".into(),
            surface_gravity: 2.5,
            surface_temperature: 180.0,
            surface_pressure: 3000.0,
            parent_star: "K".into(),
            main_star: "K".into(),
            ..Default::default()
        };
        let found = names(&body);
        assert!(found.contains(&"Stratum Tectonicas"), "{found:?}");
        // Far too warm for it, and it's off the list.
        let hot = BodyFacts { surface_temperature: 900.0, ..body };
        assert!(!names(&hot).contains(&"Stratum Tectonicas"));
    }

    #[test]
    fn electricae_pluma_follows_the_main_star() {
        let body = |main: &str| BodyFacts {
            planet_class: "Icy body".into(),
            atmosphere_type: "Argon".into(),
            surface_gravity: 1.0,
            surface_temperature: 100.0,
            parent_star: "M".into(),
            main_star: main.into(),
            ..Default::default()
        };
        assert!(names(&body("N")).contains(&"Electricae Pluma"));
        assert!(!names(&body("K")).contains(&"Electricae Pluma"));
    }

    #[test]
    fn brain_trees_grow_only_near_earlier_finds() {
        let tree = all_species().iter().find(|s| s.genus == "$Codex_Ent_Brancae_Name;" && s.near.is_some()).expect("a brain tree with a place");
        let near = tree.near.as_ref().unwrap();
        let at = near.systems[0];
        assert!(near.systems.iter().any(|p| distance(*p, at) <= near.radius_ly));
        assert!(!near.systems.iter().any(|p| distance(*p, [at[0] + 50_000.0, at[1], at[2]]) <= near.radius_ly));
    }

    #[test]
    fn codex_variants_find_their_species() {
        assert_eq!(species_of_codex("$Codex_Ent_Stratum_07_K_Name;").map(|s| s.name.as_str()), Some("Stratum Tectonicas"));
        assert_eq!(species_of_codex("$Codex_Ent_Bacterial_04_Antimony_Name;").map(|s| s.name.as_str()), Some("Bacterium Acies"));
        assert!(species_of_codex("$Codex_Ent_SeedABCD_01_Name;").is_some_and(|s| s.genus_name() == "Brain Tree"));
        assert!(species_of_codex("$Codex_Ent_IceGeysers_WaterGeysers_Name;").is_none());
    }

    #[test]
    fn species_describe_their_needs() {
        let fumerola = all_species().iter().find(|s| s.name == "Fumerola Aquatis").unwrap();
        assert!(fumerola.needs().iter().any(|n| n.starts_with("volcanism")), "{:?}", fumerola.needs());
        let pluma = all_species().iter().find(|s| s.name == "Electricae Pluma").unwrap();
        assert!(pluma.needs().iter().any(|n| n.starts_with("main star")), "{:?}", pluma.needs());
        assert_eq!(stars_in_words(&["B".into(), "K".into(), "H".into()]), "B or K stars or black holes");
        assert_eq!(stars_in_words(&["N".into()]), "neutron stars");
        assert!(pluma.atmospheres()[0] == "Argon");
    }

    #[test]
    fn species_know_their_colours() {
        let tectonicas = all_species().iter().find(|s| s.name == "Stratum Tectonicas").unwrap();
        let variants = tectonicas.variants();
        let m = variants.iter().find(|v| v.code == "M").expect("an M variant");
        assert_eq!(m.colour, "Green");
        assert!(m.from_star());
        let acies = all_species().iter().find(|s| s.name == "Bacterium Acies").unwrap();
        assert!(acies.variants().iter().any(|v| !v.from_star() && v.code == "Polonium"));
        assert!(variants.iter().all(|v| v.colour.chars().all(|c| c.is_ascii_alphabetic())));
    }

    #[test]
    fn star_types_group_into_classes() {
        assert_eq!(star_class("K_OrangeGiant"), "K");
        assert_eq!(star_class("DAB"), "D");
        assert_eq!(star_class("AeBe"), "AeBe");
        assert_eq!(star_class("A"), "A");
        assert_eq!(star_class("TTS"), "TTS");
        assert_eq!(star_class("MS"), "MS");
    }

    #[test]
    fn volcanism_reads_as_its_kind() {
        assert_eq!(volcanism_kind("minor water geysers volcanism"), "water geysers");
        assert_eq!(volcanism_kind(""), "");
        assert_eq!(volcanism_kind("rocky magma volcanism"), "rocky magma");
    }
}
