//! The codex pages: the families codex entries are grouped into, with what EDCAS writes
//! about each, and readable titles for the entries.
//!
//! Surface life is grouped by genus; where each species grows comes from
//! [`crate::exobiology`]. Everything else is grouped by its codex symbol.

use crate::exobiology::species_of_codex;

/// A part of the codex.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    SurfaceLife,
    SpaceLife,
    Geology,
    Phenomena,
    Guardian,
    Thargoid,
}

impl Section {
    pub const ALL: [Section; 6] = [
        Section::SurfaceLife,
        Section::SpaceLife,
        Section::Geology,
        Section::Phenomena,
        Section::Guardian,
        Section::Thargoid,
    ];

    /// The section's page under `/codex/`.
    pub fn slug(self) -> &'static str {
        match self {
            Section::SurfaceLife => "surface-life",
            Section::SpaceLife => "space-life",
            Section::Geology => "geology",
            Section::Phenomena => "stellar-phenomena",
            Section::Guardian => "guardian",
            Section::Thargoid => "thargoid",
        }
    }

    /// The section with page `slug`.
    pub fn from_slug(slug: &str) -> Option<Section> {
        Section::ALL.into_iter().find(|s| s.slug() == slug)
    }

    pub fn title(self) -> &'static str {
        match self {
            Section::SurfaceLife => "Surface life",
            Section::SpaceLife => "Space life",
            Section::Geology => "Geology",
            Section::Phenomena => "Stellar phenomena",
            Section::Guardian => "Guardian",
            Section::Thargoid => "Thargoid",
        }
    }

    /// A short introduction for the codex index.
    pub fn intro(self) -> &'static str {
        match self {
            Section::SurfaceLife => {
                "Plants, fungi and bacteria on landable planets. A detailed surface scan names the genera on a body; \
                 three samples of one species, taken far enough apart, sell at Vista Genomics. Each genus page lists \
                 its species with the conditions commanders found them in."
            }
            Section::SpaceLife => {
                "Life that drifts in space: crystals, molluscs, pods and trees. The full-spectrum scanner lists them \
                 under notable stellar phenomena; fly there and scan them with the composition scanner to log them."
            }
            Section::Geology => {
                "Fumaroles, geysers, gas vents and lava spouts on bodies with volcanism. The detailed surface scan \
                 counts them as geological signals."
            }
            Section::Phenomena => {
                "Lagrange clouds and energy anomalies, found among the notable stellar phenomena of a system."
            }
            Section::Guardian => {
                "What is left of the Guardians, a civilisation that died out about a million years ago: ruins, \
                 structures and the beacons that lead to them."
            }
            Section::Thargoid => {
                "The Thargoids: their ships, the sites they built on planets, the barnacles they farm and the wrecks \
                 of their vessels."
            }
        }
    }
}

/// A group of codex entries with one page: a genus of surface life, or a kind of site.
pub struct Family {
    /// The page's address under `/codex/`.
    pub slug: &'static str,
    pub name: &'static str,
    pub section: Section,
    /// One line for the index.
    pub summary: &'static str,
    /// The article, as paragraphs.
    pub text: &'static [&'static str],
    /// Surface life: the genus' codex symbol.
    pub genus: Option<&'static str>,
    /// Surface life: how far from each other (m) its samples must be taken, as the genetic
    /// sampler shows it.
    pub colony_range_m: Option<u32>,
    /// Codex symbols (without `$Codex_Ent_`) of its entries, by prefix.
    prefixes: &'static [&'static str],
}

const fn genus(
    slug: &'static str,
    name: &'static str,
    symbol: &'static str,
    colony_range_m: Option<u32>,
    summary: &'static str,
    text: &'static [&'static str],
) -> Family {
    Family { slug, name, section: Section::SurfaceLife, summary, text, genus: Some(symbol), colony_range_m, prefixes: &[] }
}

const fn group(
    slug: &'static str,
    name: &'static str,
    section: Section,
    prefixes: &'static [&'static str],
    summary: &'static str,
    text: &'static [&'static str],
) -> Family {
    Family { slug, name, section, summary, text, genus: None, colony_range_m: None, prefixes }
}

const FAMILIES: &[Family] = &[
    // ── Surface life ─────────────────────────────────────────────────────────────────
    genus("aleoida", "Aleoida", "$Codex_Ent_Aleoids_Genus_Name;", Some(150),
        "Low rosettes of thick, pointed leaves, like an aloe.",
        &["Aleoida grow as a low rosette of fleshy, pointed leaves close to the ground. They keep to cold, \
           thin atmospheres of carbon dioxide or ammonia on rocky and metal-rich worlds, and stay on the \
           lighter planets.",
          "Their colour follows the class of the star the planet orbits. Gravis and Arcus are among the \
           better-paying species of any genus, so a body that lists Aleoida is worth landing on."]),
    genus("bacterium", "Bacterium", "$Codex_Ent_Bacterial_Genus_Name;", Some(500),
        "Flat, patchy films on the ground — the commonest life in the galaxy.",
        &["Bacteria form flat, patchy mats on the ground and are easy to miss from the air: look down, not \
           up. They grow in almost every thin atmosphere, from neon on frozen worlds to sulphur dioxide on \
           warm ones, and each atmosphere has its own species.",
          "Most species pay little, but Informem, Volu and Nebulus are worth the trip. The wide colony range \
           means more driving between samples than for most genera. Depending on the species, the colour \
           follows the star or an element in the planet's crust."]),
    genus("cactoida", "Cactoida", "$Codex_Ent_Cactoid_Genus_Name;", Some(300),
        "Ribbed upright columns with stubby arms, like a cactus.",
        &["Cactoida stand upright as ribbed columns with short arms, much like a desert cactus. They grow on \
           rocky and metal-rich worlds with thin carbon dioxide, ammonia or water atmospheres.",
          "Vermis, found in thin water and sulphur dioxide atmospheres, is one of the most valuable species \
           there is."]),
    genus("clypeus", "Clypeus", "$Codex_Ent_Clypeus_Genus_Name;", Some(150),
        "Shield-shaped cups on short stalks.",
        &["Clypeus raise flat, shield-shaped cups on short stalks. They need warmth — above about 185 K — and \
           grow on rocky and metal-rich worlds with thin carbon dioxide or water atmospheres.",
          "All three species pay well; Speculumi is among the most valuable in the codex."]),
    genus("concha", "Concha", "$Codex_Ent_Conchas_Genus_Name;", Some(150),
        "A single cupped shell opening upwards, like a clam.",
        &["Concha look like a single shell half-buried in the ground, opening upwards like a clam. They grow \
           in thin carbon dioxide, ammonia, nitrogen and water atmospheres.",
          "Their colour follows the star for some species and the crust's elements for others. Biconcavis, \
           found in thin nitrogen, pays best."]),
    genus("electricae", "Electricae", "$Codex_Ent_Electricae_Genus_Name;", Some(1000),
        "Thin glowing filaments radiating from a small base.",
        &["Electricae are bundles of thin, glowing filaments spreading from a small base, best seen at night. \
           They live on icy bodies with thin argon, neon or helium atmospheres.",
          "They grow only in some regions of the galaxy, and Pluma only under certain classes of the system's \
           main star, not the star the planet orbits. The 1 km colony range makes them the slowest genus to \
           sample."]),
    genus("fonticulua", "Fonticulua", "$Codex_Ent_Fonticulus_Genus_Name;", Some(500),
        "Rounded mounds of stacked, frilly plates.",
        &["Fonticulua build rounded mounds of stacked, frilly plates. They are the life of the coldest worlds: \
           icy and rocky-ice bodies with thin argon, neon, methane, nitrogen or oxygen atmospheres.",
          "Fluctus, which needs thin oxygen, is one of the best-paying species in the codex."]),
    genus("frutexa", "Frutexa", "$Codex_Ent_Shrubs_Genus_Name;", Some(150),
        "Low, bushy shrubs of branching stems.",
        &["Frutexa grow as low, branching shrubs. They take thin carbon dioxide, ammonia, sulphur dioxide and \
           water atmospheres on rocky and metal-rich worlds.",
          "Flammasis, in thin ammonia, pays best; the other species sit in the middle."]),
    genus("fumerola", "Fumerola", "$Codex_Ent_Fumerolas_Genus_Name;", Some(100),
        "Crusted mineral chimneys around volcanic vents.",
        &["Fumerola grow as crusted, chimney-like mounds around volcanic vents. They need volcanism: each \
           species keeps to its own kind of geysers or magma, on icy, rocky-ice and rocky worlds.",
          "Look for them next to geological signals. Their colour follows an element in the crust, and their \
           colony range is the shortest of all genera."]),
    genus("fungoida", "Fungoida", "$Codex_Ent_Fungoids_Genus_Name;", Some(300),
        "Clusters of puffball and mushroom caps.",
        &["Fungoida cluster as round caps and puffballs on thin stalks. They grow in thin atmospheres of argon, \
           ammonia, carbon dioxide, methane and water on rocky, rocky-ice and metal-rich worlds.",
          "Their colour follows an element in the crust. They are common and pay modestly."]),
    genus("osseus", "Osseus", "$Codex_Ent_Osseus_Genus_Name;", Some(800),
        "Bone-white discs, spirals and fans.",
        &["Osseus look like bone: pale discs, spirals and fans lying on the ground. They grow on rocky, \
           metal-rich and rocky-ice worlds in thin carbon dioxide, ammonia, methane, argon, nitrogen or water; \
           a few species need volcanism.",
          "Discus and Pellebantus pay well. With an 800 m colony range, plan the drive between samples."]),
    genus("recepta", "Recepta", "$Codex_Ent_Recepta_Genus_Name;", Some(150),
        "Spiky, angular pods in sulphur dioxide atmospheres.",
        &["Recepta grow as spiky, angular pods. EDCAS's own data shows all three species need a trace of \
           sulphur dioxide in the atmosphere, and all of them pay well.",
          "They grow on icy, rocky and metal-rich worlds, mostly in thin sulphur dioxide."]),
    genus("stratum", "Stratum", "$Codex_Ent_Stratum_Genus_Name;", Some(500),
        "Flat crusts and layered mats spreading over rock.",
        &["Stratum spread over rock as flat, layered crusts. They like it warmer than most life — above about \
           160 K — on rocky and metal-rich worlds with thin carbon dioxide, ammonia, sulphur dioxide, \
           oxygen or water atmospheres.",
          "Tectonicas, on metal-rich worlds, is the best-paying common species in the galaxy, which makes high \
           metal content bodies with thin atmospheres a favourite target."]),
    genus("tubus", "Tubus", "$Codex_Ent_Tubus_Genus_Name;", Some(800),
        "Clusters of upright hollow tubes.",
        &["Tubus raise clusters of upright, hollow tubes. They keep to rocky and metal-rich worlds with low \
           gravity and thin carbon dioxide or ammonia atmospheres.",
          "Cavas pays well; with an 800 m colony range, sampling takes a while."]),
    genus("tussock", "Tussock", "$Codex_Ent_Tussocks_Genus_Name;", Some(200),
        "Grass-like tufts of thin blades.",
        &["Tussocks grow as grass-like tufts of thin blades. They are the genus with the most species, and \
           each keeps to its own atmosphere and temperature band on rocky and metal-rich worlds.",
          "Their colour follows the star. Most species pay modestly, but Stigmasis, in thin sulphur dioxide, \
           pays as much as any species in the codex."]),
    genus("radicoida", "Radicoida", "$Codex_Ent_Ingensradices_Genus_Name;", None,
        "A giant root structure, logged only in a few places.",
        &["Radicoida Unica is a vast root structure, only found in a few places. It is a single species and \
           very rarely logged."]),
    genus("amphora-plant", "Amphora Plant", "$Codex_Ent_Vents_Name;", None,
        "Urn-shaped plants on hot, airless metal-rich worlds.",
        &["Amphora plants are urn-shaped growths on hot, airless metal-rich worlds with metallic magma or \
           silicate vapour volcanism. They are rarely logged."]),
    genus("anemone", "Anemone", "$Codex_Ent_Sphere_Name;", None,
        "Round, petalled growths; some glow.",
        &["Anemones are round growths with fleshy petals; the bioluminescent kinds glow in the dark. They grow \
           mostly on airless rocky and metal-rich worlds with volcanism.",
          "EDCAS's own data shows that several kinds follow the system's main star rather than the star the \
           planet orbits: they grow only where the main star is a hot B or O star."]),
    genus("bark-mound", "Bark Mound", "$Codex_Ent_Cone_Name;", None,
        "Cone-shaped mounds of bark-like material.",
        &["Bark mounds are cone-shaped mounds of bark-like material, mostly on airless worlds with \
           volcanism. They grow only in some regions of the galaxy."]),
    genus("brain-tree", "Brain Tree", "$Codex_Ent_Brancae_Name;", None,
        "Trees with a wrinkled, brain-like crown.",
        &["Brain trees carry a wrinkled, brain-like crown on a short trunk. They grow mostly on airless worlds \
           with volcanism.",
          "EDCAS's data shows each species stays close to its earlier finds: commanders found them within \
           500 light years of earlier finds of the same species."]),
    genus("crystalline-shards", "Crystalline Shards", "$Codex_Ent_Ground_Struct_Ice_Name;", None,
        "Fields of tall crystal spikes on frozen worlds.",
        &["Crystalline shards are fields of tall crystal spikes on cold worlds, mostly airless and with \
           volcanism."]),
    genus("sinuous-tubers", "Sinuous Tubers", "$Codex_Ent_Tube_Name;", None,
        "Twisting tube-like growths on volcanic worlds.",
        &["Sinuous tubers are twisting, tube-like growths on airless rocky worlds with volcanism. They grow \
           only in some regions of the galaxy."]),
    // ── Space life ───────────────────────────────────────────────────────────────────
    group("crystals", "Crystals", Section::SpaceLife, &["L_Cry_"],
        "Clusters of ice, metallic and silicate crystals floating in space.",
        &["Crystal clusters drift in space as growths of ice, metal or silicate, in many colours. They are \
           listed among a system's notable stellar phenomena, often near Lagrange clouds.",
          "Metallic crystals are the ones commanders log most; the colour is part of the codex entry."]),
    group("molluscs", "Molluscs", Section::SpaceLife, &["L_Org_Moll", "Small_Org_Moll"],
        "Floating organisms with soft bodies: gourd, umbrella and others.",
        &["Space molluscs are soft-bodied organisms that float in groups, some trailing tendrils. The codex \
           tells them apart by shape and colour; the gourd and umbrella molluscs are the ones logged most.",
          "Like other space life, they show among notable stellar phenomena."]),
    group("pods-and-trees", "Pods and trees", Section::SpaceLife, &["L_Seed_", "S_Seed_"],
        "Seed pods and the trees that release them.",
        &["Space pods are seed-like organisms floating in clusters; peduncle trees and their relatives anchor \
           them. The codex sorts them by type and colour.",
          "Peduncle pods and peduncle trees are the kinds commanders log most."]),
    group("mineral-spheres", "Mineral spheres", Section::SpaceLife, &["SPOI_"],
        "Solid and lattice mineral spheres and their companions.",
        &["Mineral spheres are round, solid or lattice-like structures. Commanders logged them under organic \
           structures in the codex, together with a few polyp-like and weed-like companions."]),
    group("other-space-life", "Other space life", Section::SpaceLife, &["L_Org_"],
        "Organisms in space the other groups don't cover.",
        &["Organisms that drift in space and fit none of the other groups."]),
    // ── Geology ──────────────────────────────────────────────────────────────────────
    group("fumaroles", "Fumaroles", Section::Geology, &["Fumarole_"],
        "Vents breathing hot gas: sulphur dioxide, silicate vapour or water.",
        &["Fumaroles are vents that breathe out hot gas and vapour. They sit on rocky and metal-rich worlds \
           whose volcanism gives them their kind: sulphur dioxide, silicate vapour or water.",
          "Like all geological sites they show as geological signals after a detailed surface scan. The rocks \
           around them are a good place to collect raw materials with an SRV, and Fumerola grows nearby."]),
    group("ice-fumaroles", "Ice fumaroles", Section::Geology, &["IceFumarole_"],
        "Vents on frozen worlds breathing water, nitrogen, methane, ammonia or CO₂.",
        &["Ice fumaroles are vents on icy worlds that release volatile ices as vapour: water, carbon dioxide, \
           ammonia, methane or nitrogen, depending on the body's cryovolcanism.",
          "They show as geological signals and are good places to look for Fumerola and for raw materials."]),
    group("geysers", "Geysers", Section::Geology, &["IceGeysers_", "Geysers_"],
        "Eruptions of water, nitrogen, methane, ammonia or CO₂.",
        &["Geysers throw jets of liquid and ice high above the surface at intervals. Ice geysers on frozen \
           worlds erupt water, carbon dioxide, ammonia, methane or nitrogen; warmer worlds have water geysers.",
          "Approach on foot or in an SRV and wait for an eruption — they are among the most spectacular sights \
           in the galaxy."]),
    group("gas-vents", "Gas vents", Section::Geology, &["Gas_Vents_"],
        "Openings that release gas continuously: sulphur dioxide, silicate vapour, water or CO₂.",
        &["Gas vents release a steady plume of gas from openings in the ground. The gas follows the body's \
           volcanism: sulphur dioxide, silicate vapour, water or carbon dioxide.",
          "They show as geological signals; the ground around them carries raw materials."]),
    group("lava-spouts", "Lava spouts", Section::Geology, &["Lava_Spouts_"],
        "Fountains of molten iron or silicate rock.",
        &["Lava spouts throw molten rock into the air: iron magma on metal-rich worlds, silicate magma on \
           rocky ones. They are the hottest geological sites, on bodies with magma volcanism."]),
    // ── Stellar phenomena ────────────────────────────────────────────────────────────
    group("lagrange-clouds", "Lagrange clouds", Section::Phenomena, &["Gas_Clds_"],
        "Coloured gas clouds held at Lagrange points; some storm.",
        &["Lagrange clouds are clouds of gas gathered at the stable Lagrange points between a planet and its \
           star or moon. Their colour gives them their name — Croceum, Rubicundum, Viride and others — and \
           storm clouds crackle with lightning.",
          "They show among a system's notable stellar phenomena and often host space life and anomalies."]),
    group("anomalies", "Anomalies", Section::Phenomena, &["L_Phn_"],
        "Unexplained energy phenomena in space, numbered by type.",
        &["Anomalies are unexplained energy phenomena: glowing orbs, particle clouds and clusters. The codex \
           numbers each kind, and the orb types carry Q-class names like Q04.",
          "They are found among notable stellar phenomena, usually near Lagrange clouds."]),
    // ── Guardian ─────────────────────────────────────────────────────────────────────
    group("guardian-sites", "Guardian sites", Section::Guardian,
        &["Guardian_Data_Logs", "Guardian_Terminal", "Guardian_Pylon", "Guardian_Sentinel", "Relic_Tower", "Guardian_Obelisk"],
        "Ruins and structures: obelisks, terminals, pylons, relic towers and sentinels.",
        &["The Guardians died out around a million years ago and left ruins and structures on planets in a \
           region of the Vela and Puppis constellations. Ruins hold obelisks whose records the codex keeps as \
           the Guardian Codex, and relic towers that hold relics; structures have data terminals, power \
           pylons and sentinels that still defend them.",
          "Data from the terminals and obelisks, with the relics and Guardian materials, buys modules and \
           weapons at the Guardian technology brokers."]),
    group("guardian-beacons", "Guardian beacons", Section::Guardian, &["Guardian_Beacons"],
        "Beacons in space that point to Guardian structures.",
        &["Guardian beacons are structures in space that hold data pointing the way to Guardian sites on \
           planets nearby. Scanning one reveals where to look."]),
    // ── Thargoid ─────────────────────────────────────────────────────────────────────
    group("thargoid-vessels", "Thargoid vessels", Section::Thargoid,
        &["Basilisk", "Berserker", "Cyclops", "Glaive", "Hydra", "Inciter", "Marauder", "Medusa", "Orthrus",
          "Regenerator", "Scavenger", "Scout", "Scythe"],
        "Interceptors, scouts, hunters and scavengers.",
        &["Thargoid ships come in families. Scouts — Marauder, Inciter, Berserker and Regenerator — are the \
           smaller craft; interceptors — Cyclops, Basilisk, Medusa, Hydra and Orthrus — are the large flower \
           ships; Glaive and Scythe hunters appeared later, and scavengers guard Thargoid sites on planets.",
          "Commanders log them by scanning them with a ship's scanner, usually while fighting them with \
           anti-xeno weapons."]),
    group("thargoid-wrecks", "Thargoid wrecks", Section::Thargoid, &["Wrecked_"],
        "Crashed interceptors and scouts on planets.",
        &["The wrecks of downed Thargoid interceptors and scouts lie on planets, sometimes guarded by \
           scavengers. They are a source of Thargoid materials and samples."]),
    group("thargoid-barnacles", "Barnacles and coral", Section::Thargoid, &["Thargoid_Barnacle", "Thargoid_Coral"],
        "Barnacles, mega barnacles and coral the Thargoids grow.",
        &["Barnacles are Thargoid growths on planets that produce meta-alloys. Mega barnacles grow larger and \
           sharper barbs, and some stand submerged; coral trees and roots spread around Thargoid sites."]),
    group("thargoid-sites", "Thargoid sites", Section::Thargoid, &["TG_", "Caustic_Generator"],
        "Devices, pods and generators at Thargoid surface sites.",
        &["Thargoid surface sites hold devices, pods, uplink devices and caustic generators. Activating them \
           with the right items reveals data and records the codex keeps."]),
    group("thargoid-spires", "Thargoid spires", Section::Thargoid, &["Thargoid_Tower"],
        "Tall spires the Thargoids built on planets during the war.",
        &["Thargoid spires rose on planets in systems the Thargoids held during their war with humanity. The \
           codex records them by height."]),
];

/// Every family, in page order.
pub fn families() -> &'static [Family] {
    FAMILIES
}

/// The family with page `slug`.
pub fn family(slug: &str) -> Option<&'static Family> {
    FAMILIES.iter().find(|f| f.slug == slug)
}

/// The family a codex entry belongs to: the genus of a surface species, or the family whose
/// symbol prefix fits best.
pub fn family_of(symbol: &str) -> Option<&'static Family> {
    if let Some(species) = species_of_codex(symbol) {
        return FAMILIES.iter().find(|f| f.genus == Some(species.genus.as_str()));
    }
    let core = core(symbol);
    FAMILIES
        .iter()
        .flat_map(|f| f.prefixes.iter().map(move |p| (f, *p)))
        .filter(|(_, p)| core.starts_with(p))
        .max_by_key(|(_, p)| p.len())
        .map(|(f, _)| f)
}

/// `$Codex_Ent_IceGeysers_NitrogenGeysers_Name;` → `IceGeysers_NitrogenGeysers`.
fn core(symbol: &str) -> &str {
    symbol.trim_start_matches('$').trim_start_matches("Codex_Ent_").trim_end_matches(';').trim_end_matches("_Name")
}

/// A codex entry's address on the web: `$Codex_Ent_Cyclops_Name;` → `Cyclops`.
pub fn entry_key(symbol: &str) -> &str {
    core(symbol)
}

/// The codex entry at a web address: `Cyclops` → `$Codex_Ent_Cyclops_Name;`.
pub fn entry_symbol(key: &str) -> String {
    format!("$Codex_Ent_{key}_Name;")
}

/// A codex entry's title: "Nitrogen Ice Geyser", "Rubicundum Lagrange Cloud", "Cyclops
/// Interceptor", "Stratum Paleas – Green"; the symbol in words where EDCAS knows no
/// better.
pub fn entry_title(symbol: &str) -> String {
    if let Some(species) = species_of_codex(symbol) {
        return match species.variants().iter().find(|v| v.symbol == symbol) {
            Some(v) => format!("{} – {}", species.name, v.colour),
            None => species.name.clone(),
        };
    }
    let core = core(symbol);
    let after = |prefix: &str| core.strip_prefix(prefix);
    let site = |prefix: &str, kind: &str| after(prefix).and_then(material).map(|m| format!("{m} {kind}"));
    let known = site("Fumarole_", "Fumarole")
        .or_else(|| site("IceFumarole_", "Ice Fumarole"))
        .or_else(|| site("IceGeysers_", "Ice Geyser"))
        .or_else(|| site("Geysers_", "Geyser"))
        .or_else(|| site("Gas_Vents_", "Gas Vent"))
        .or_else(|| site("Lava_Spouts_", "Lava Spout"))
        .or_else(|| after("Gas_Clds_").and_then(lagrange_cloud))
        .or_else(|| after("L_Phn_Part_").and_then(anomaly))
        .or_else(|| after("L_Cry_").and_then(crystals))
        .or_else(|| after("L_Seed_Pln01_V1_").and_then(|c| latin(c, PEDUNCLE_TREE)).map(|c| format!("{c} Peduncle Tree")))
        .or_else(|| after("S_Seed_SdTp01_").and_then(|c| latin(c, PEDUNCLE_POD)).map(|c| format!("{c} Peduncle Pod")))
        .or_else(|| NAMED.iter().find(|(k, _)| *k == core).map(|(_, v)| v.to_string()));
    known.unwrap_or_else(|| super::entry_words(symbol))
}

/// Entries with a name of their own.
const NAMED: &[(&str, &str)] = &[
    ("Small_Org_Moll01_V1_Def", "Albulum Gourd Mollusc"),
    ("Small_Org_Moll01_V1_Y", "Croceum Gourd Mollusc"),
    ("L_Org_Moll03_V3_Earth", "Viride Umbrella Mollusc"),
    ("SPOI_Ball", "Solid Mineral Spheres"),
    ("SPOI_Ball_Lattice", "Lattice Mineral Spheres"),
    ("Guardian_Beacons", "Guardian Beacon"),
    ("Guardian_Data_Logs", "Guardian Codex"),
    ("Guardian_Pylon", "Guardian Pylon"),
    ("Guardian_Sentinel", "Guardian Sentinel"),
    ("Guardian_Terminal", "Guardian Data Terminal"),
    ("Relic_Tower", "Guardian Relic Tower"),
    ("Basilisk", "Basilisk Interceptor"),
    ("Cyclops", "Cyclops Interceptor"),
    ("Hydra", "Hydra Interceptor"),
    ("Medusa", "Medusa Interceptor"),
    ("Orthrus", "Orthrus Interceptor"),
    ("Berserker", "Berserker Scout"),
    ("Inciter", "Inciter Scout"),
    ("Marauder", "Marauder Scout"),
    ("Regenerator", "Regenerator Scout"),
    ("Scythe", "Scythe Hunter"),
    ("Glaive", "Glaive Hunter"),
    ("Scavenger", "Scavenger"),
    ("Scavengers", "Scavengers"),
    ("Caustic_Generator", "Caustic Generator"),
    ("TG_DataScan", "Thargoid Uplink Device"),
    ("TG_Pod", "Thargoid Pod"),
    ("TG_Transmitter", "Thargoid Device"),
    ("Thargoid_Barnacle_01", "Barnacle"),
    ("Thargoid_Barnacle_Matrix", "Mega Barnacle"),
    ("Thargoid_Barnacle_Matrix_Submerged", "Submerged Mega Barnacle"),
    ("Thargoid_Barnacle_Spikes", "Barnacle Barbs"),
    ("Thargoid_Coral_Root", "Coral Root"),
    ("Thargoid_Coral_Tree", "Coral Tree"),
    ("Thargoid_Tower", "Thargoid Spire"),
    ("Thargoid_Tower_Low", "Thargoid Spire (low)"),
    ("Thargoid_Tower_Med", "Thargoid Spire (medium)"),
    ("Thargoid_Tower_High", "Thargoid Spire (high)"),
    ("Thargoid_Tower_ExtraHigh", "Thargoid Spire (extra high)"),
    ("Wrecked_Interceptor", "Interceptor Shipwreck"),
    ("Wrecked_Scout", "Scout Shipwreck"),
];

/// "NitrogenGeysers" → "Nitrogen", "IronMagma" → "Iron Magma".
fn material(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "WaterGeysers" => "Water",
        "CarbonDioxideGeysers" => "Carbon Dioxide",
        "AmmoniaGeysers" => "Ammonia",
        "MethaneGeysers" => "Methane",
        "NitrogenGeysers" => "Nitrogen",
        "SilicateVapourGeysers" => "Silicate Vapour",
        "SulphurDioxideMagma" => "Sulphur Dioxide",
        "IronMagma" => "Iron Magma",
        "SilicateMagma" => "Silicate Magma",
        _ => return None,
    })
}

fn lagrange_cloud(rest: &str) -> Option<String> {
    let (colour, storm) = match rest.strip_suffix("_Storm") {
        Some(c) => (c, true),
        None => (rest, false),
    };
    let name = match colour {
        "Blue" => "Caeruleum",
        "Green" => "Viride",
        "Light" => "Proto",
        "Orange" => "Luteolum",
        "Pink" => "Roseum",
        "Red" => "Rubicundum",
        "Yellow" => "Croceum",
        _ => return None,
    };
    Some(if storm { format!("{name} Lagrange Storm Cloud") } else { format!("{name} Lagrange Cloud") })
}

/// `Orb_004` → "Q04 Anomaly"; `Cld_011` → "Particle Cloud Anomaly 11".
fn anomaly(rest: &str) -> Option<String> {
    let (kind, n) = rest.split_once('_')?;
    let n: u32 = n.parse().ok()?;
    Some(match kind {
        "Orb" => format!("Q{n:02} Anomaly"),
        "Cld" => format!("Particle Cloud Anomaly {n}"),
        "Clus" => format!("Particle Cluster Anomaly {n}"),
        "Eng" => format!("Energy Anomaly {n}"),
        _ => return None,
    })
}

fn crystals(rest: &str) -> Option<String> {
    let (kind, colour) = rest.split_once('_')?;
    let kind = match kind {
        "IcCry" => "Ice Crystals",
        "MetCry" => "Metallic Crystals",
        "QtzCry" => "Silicate Crystals",
        _ => return None,
    };
    Some(format!("{} {kind}", latin(colour, CRYSTAL)?))
}

const CRYSTAL: &[(&str, &str)] = &[
    ("Bl", "Lindigoticum"), ("Gr", "Prasinum"), ("Pk", "Roseum"), ("Pur", "Purpureum"),
    ("Red", "Rubeum"), ("Wte", "Albidum"), ("Yw", "Flavum"),
];
const PEDUNCLE_TREE: &[(&str, &str)] =
    &[("Bl", "Caeruleum"), ("Def", "Albidum"), ("Gr", "Viride"), ("Pur", "Ostrinum"), ("Red", "Rubellum")];
const PEDUNCLE_POD: &[(&str, &str)] =
    &[("Bl", "Caeruleum"), ("Def", "Candidum"), ("Gr", "Gypseeum"), ("Pur", "Purpureum"), ("Red", "Rufum")];

fn latin(code: &str, names: &[(&str, &'static str)]) -> Option<&'static str> {
    names.iter().find(|(k, _)| *k == code).map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exobiology::all_genera;

    #[test]
    fn section_pages_share_no_address_with_an_article() {
        for section in Section::ALL {
            assert_eq!(Section::from_slug(section.slug()), Some(section));
            assert!(family(section.slug()).is_none(), "{} is also an article", section.slug());
        }
    }

    #[test]
    fn every_genus_has_a_page() {
        for (symbol, name) in all_genera() {
            assert!(FAMILIES.iter().any(|f| f.genus == Some(symbol)), "no page for {name}");
        }
        let mut slugs: Vec<&str> = FAMILIES.iter().map(|f| f.slug).collect();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), FAMILIES.len(), "slugs are unique");
    }

    #[test]
    fn entries_find_their_family() {
        let slug = |s: &str| family_of(s).map(|f| f.slug);
        assert_eq!(slug("$Codex_Ent_Stratum_07_M_Name;"), Some("stratum"));
        assert_eq!(slug("$Codex_Ent_SeedABCD_01_Name;"), Some("brain-tree"));
        assert_eq!(slug("$Codex_Ent_IceGeysers_NitrogenGeysers_Name;"), Some("geysers"));
        assert_eq!(slug("$Codex_Ent_IceFumarole_WaterGeysers_Name;"), Some("ice-fumaroles"));
        assert_eq!(slug("$Codex_Ent_L_Org_Moll03_V3_Earth_Name;"), Some("molluscs"));
        assert_eq!(slug("$Codex_Ent_L_Org_PltFun_V1_Bl_Name;"), Some("other-space-life"));
        assert_eq!(slug("$Codex_Ent_Thargoid_Barnacle_Matrix_Name;"), Some("thargoid-barnacles"));
        assert_eq!(slug("$Codex_Ent_Thargoid_Tower_High_Name;"), Some("thargoid-spires"));
        assert_eq!(slug("$Codex_Ent_Relic_Tower_Name;"), Some("guardian-sites"));
        assert_eq!(slug("$Codex_Ent_Something_New_Name;"), None);
    }

    #[test]
    fn entries_read_as_titles() {
        assert_eq!(entry_title("$Codex_Ent_IceGeysers_NitrogenGeysers_Name;"), "Nitrogen Ice Geyser");
        assert_eq!(entry_title("$Codex_Ent_Fumarole_SulphurDioxideMagma_Name;"), "Sulphur Dioxide Fumarole");
        assert_eq!(entry_title("$Codex_Ent_Lava_Spouts_IronMagma_Name;"), "Iron Magma Lava Spout");
        assert_eq!(entry_title("$Codex_Ent_Gas_Clds_Red_Storm_Name;"), "Rubicundum Lagrange Storm Cloud");
        assert_eq!(entry_title("$Codex_Ent_L_Phn_Part_Orb_004_Name;"), "Q04 Anomaly");
        assert_eq!(entry_title("$Codex_Ent_L_Cry_MetCry_Gr_Name;"), "Prasinum Metallic Crystals");
        assert_eq!(entry_title("$Codex_Ent_S_Seed_SdTp01_Def_Name;"), "Candidum Peduncle Pod");
        assert_eq!(entry_title("$Codex_Ent_Wrecked_Interceptor_Name;"), "Interceptor Shipwreck");
        assert_eq!(entry_title("$Codex_Ent_Something_New_Name;"), "Something New");
        assert_eq!(entry_title("$Codex_Ent_Stratum_07_M_Name;"), "Stratum Tectonicas – Green");
    }
}
