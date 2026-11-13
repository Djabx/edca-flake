//! Elite Dangerous' `GraphicsConfigurationOverride.xml`: values in it replace those of
//! the game's own graphics configuration, beyond what the in-game menu offers — more stars
//! in the galaxy map, bigger planet and sky textures, a recoloured HUD. The game reads it
//! at start and never writes it.
//!
//! EDCAS writes the overrides the player opted in to and leaves everything else in the
//! file as it was: entries it doesn't know, comments, and keys of options the player
//! never touched. Before its first write it keeps a copy of the original next to it.

use std::path::{Path, PathBuf};

use crate::settings::game_graphics::{GameGraphicsSettings, OverrideChoice};
use crate::settings::Settings;

pub const FILE_NAME: &str = "GraphicsConfigurationOverride.xml";
/// The file as it was before EDCAS first wrote it.
pub const BACKUP_SUFFIX: &str = ".edcas-original";

/// What an option's value is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Int { min: i64, max: i64 },
    Float { min: f32, max: f32 },
    /// A row of the HUD colour matrix: three numbers.
    Row,
}

/// One override EDCAS offers: a key of the game's configuration, written into every
/// quality level of its section so it holds whatever the game is set to.
#[derive(Debug)]
pub struct GraphicsOption {
    pub id: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub section: &'static str,
    pub levels: &'static [&'static str],
    pub key: &'static str,
    /// The game's own value at its highest level.
    pub game: &'static str,
    /// What an opt-in starts from.
    pub suggested: &'static str,
    pub kind: Kind,
}

const GALAXY_MAP: &[&str] = &["Low", "Medium", "High"];
const BACKGROUND: &[&str] = &["Low", "Medium", "High"];
const PLANETS: &[&str] = &["Low", "Medium", "High", "Ultra"];
const ENVMAP: &[&str] = &["Low", "High"];
const GUI_COLOUR: &[&str] = &["Default"];

/// The overrides offered, in the order the settings show them. Game values are those of
/// the game's `GraphicsConfiguration.xml` (Odyssey 4.x).
pub const OPTIONS: &[GraphicsOption] = &[
    GraphicsOption {
        id: "galaxy_map_stars",
        group: "Galaxy map",
        label: "Galaxy map stars",
        help: "Stars the galaxy map draws around the selected system. More fill the map; very high counts can stutter when the map opens.",
        section: "GalaxyMap",
        levels: GALAXY_MAP,
        key: "StarInstanceCount",
        game: "4000",
        suggested: "32000",
        kind: Kind::Int { min: 1_000, max: 128_000 },
    },
    GraphicsOption {
        id: "galaxy_map_nebulae",
        group: "Galaxy map",
        label: "Galaxy map nebulae",
        help: "Nebulae the galaxy map draws.",
        section: "GalaxyMap",
        levels: GALAXY_MAP,
        key: "NebulasCount",
        game: "30",
        suggested: "100",
        kind: Kind::Int { min: 0, max: 200 },
    },
    GraphicsOption {
        id: "milky_way_brightness",
        group: "Galaxy map",
        label: "Milky Way brightness",
        help: "How bright the galaxy's band of stars shows in the galaxy map.",
        section: "GalaxyMap",
        levels: GALAXY_MAP,
        key: "MilkywayInstancesBrightness",
        game: "1.0",
        suggested: "1.5",
        kind: Kind::Float { min: 0.1, max: 4.0 },
    },
    GraphicsOption {
        id: "milky_way_size",
        group: "Galaxy map",
        label: "Milky Way star size",
        help: "How big the galaxy's stars are drawn in the galaxy map.",
        section: "GalaxyMap",
        levels: GALAXY_MAP,
        key: "MilkywayInstancesSize",
        game: "1.0",
        suggested: "2.0",
        kind: Kind::Float { min: 0.25, max: 4.0 },
    },
    GraphicsOption {
        id: "local_dust",
        group: "Galaxy map",
        label: "Local dust brightness",
        help: "Dust clouds around the selected system in the galaxy map. The game shows none.",
        section: "GalaxyMap",
        levels: GALAXY_MAP,
        key: "LocalDustBrightness",
        game: "0.0",
        suggested: "0.1",
        kind: Kind::Float { min: 0.0, max: 1.0 },
    },
    GraphicsOption {
        id: "sky_texture",
        group: "Sky and planets",
        label: "Sky texture size",
        help: "Resolution of the galaxy in the background of space. 4096 is sharp on big screens.",
        section: "GalaxyBackground",
        levels: BACKGROUND,
        key: "TextureSize",
        game: "2048",
        suggested: "4096",
        kind: Kind::Int { min: 512, max: 8_192 },
    },
    GraphicsOption {
        id: "planet_textures",
        group: "Sky and planets",
        label: "Planet texture size",
        help: "Resolution of planets seen from space.",
        section: "Planets",
        levels: PLANETS,
        key: "TextureSize",
        game: "2560",
        suggested: "4096",
        kind: Kind::Int { min: 512, max: 8_192 },
    },
    GraphicsOption {
        id: "planet_work",
        group: "Sky and planets",
        label: "Planet work per frame",
        help: "How much planet surface the game builds each frame: higher loads detail sooner on approach and costs some frame rate.",
        section: "Planets",
        levels: PLANETS,
        key: "WorkPerFrame",
        game: "328",
        suggested: "512",
        kind: Kind::Int { min: 64, max: 2_048 },
    },
    GraphicsOption {
        id: "reflection_size",
        group: "Sky and planets",
        label: "Reflection map size",
        help: "Resolution of the reflections on ships and stations.",
        section: "Envmap",
        levels: ENVMAP,
        key: "TextureSize",
        game: "256",
        suggested: "1024",
        kind: Kind::Int { min: 128, max: 2_048 },
    },
    GraphicsOption {
        id: "hud_red",
        group: "HUD colour",
        label: "HUD red row",
        help: "How much of the HUD's red goes into red, green and blue. 1, 0, 0 leaves it as it is.",
        section: "GUIColour",
        levels: GUI_COLOUR,
        key: "MatrixRed",
        game: "1, 0, 0",
        suggested: "1, 0, 0",
        kind: Kind::Row,
    },
    GraphicsOption {
        id: "hud_green",
        group: "HUD colour",
        label: "HUD green row",
        help: "How much of the HUD's green goes into red, green and blue. 0, 1, 0 leaves it as it is.",
        section: "GUIColour",
        levels: GUI_COLOUR,
        key: "MatrixGreen",
        game: "0, 1, 0",
        suggested: "0, 1, 0",
        kind: Kind::Row,
    },
    GraphicsOption {
        id: "hud_blue",
        group: "HUD colour",
        label: "HUD blue row",
        help: "How much of the HUD's blue goes into red, green and blue. 0, 0, 1 leaves it as it is.",
        section: "GUIColour",
        levels: GUI_COLOUR,
        key: "MatrixBlue",
        game: "0, 0, 1",
        suggested: "0, 0, 1",
        kind: Kind::Row,
    },
];

pub fn option_by_label(label: &str) -> Option<&'static GraphicsOption> {
    OPTIONS.iter().find(|o| o.label == label)
}

impl Kind {
    /// The value as the file takes it, or why it can't be.
    pub fn normalize(self, input: &str) -> Result<String, String> {
        let input = input.trim();
        match self {
            Kind::Int { min, max } => match input.replace(['_', ',', '.', ' '], "").parse::<i64>() {
                Ok(n) if (min..=max).contains(&n) => Ok(n.to_string()),
                _ => Err(format!("Must be a whole number {min}–{max}")),
            },
            Kind::Float { min, max } => match input.replace(',', ".").parse::<f32>() {
                Ok(x) if (min..=max).contains(&x) => Ok(number(x)),
                _ => Err(format!("Must be a number {}–{}", number(min), number(max))),
            },
            Kind::Row => {
                let parts: Vec<Result<f32, _>> = input.split([',', ';', ' ']).filter(|p| !p.is_empty()).map(str::parse).collect();
                match parts.as_slice() {
                    [Ok(a), Ok(b), Ok(c)] if [a, b, c].iter().all(|x| (-5.0..=5.0).contains(*x)) => {
                        Ok(format!("{}, {}, {}", number(*a), number(*b), number(*c)))
                    }
                    _ => Err("Three numbers from -5 to 5, like 1, 0, 0".to_string()),
                }
            }
        }
    }

    /// The accepted range, for the settings.
    pub fn range(self) -> String {
        match self {
            Kind::Int { min, max } => format!("{}–{}", group_thousands(min), group_thousands(max)),
            Kind::Float { min, max } => format!("{}–{}", number(min), number(max)),
            Kind::Row => "red, green, blue".to_string(),
        }
    }
}

/// `1.5`, `2`, `0.1`: no trailing zeros.
fn number(x: f32) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".to_string() } else { s.to_string() }
}

fn group_thousands(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

// ─── Where the file is ───────────────────────────────────────────────────────

/// The game's `Options/Graphics` folder: the one set in the settings, or the one next to
/// the journal folder (`…/Saved Games/Frontier Developments/Elite Dangerous` →
/// `…/AppData/Local/Frontier Developments/Elite Dangerous/Options/Graphics`, in a Proton
/// prefix as on Windows). `None` when it can't be found.
pub fn graphics_dir(settings: &Settings) -> Option<PathBuf> {
    let set = settings.game_graphics.folder.trim();
    if !set.is_empty() {
        return Some(PathBuf::from(set));
    }
    let local = Path::new("AppData").join("Local").join("Frontier Developments").join("Elite Dangerous").join("Options").join("Graphics");
    let journal = PathBuf::from(settings.journal_reader.journal_directory.trim());
    let from_journal = journal.ancestors().nth(3).map(|user| user.join(&local)).filter(|_| {
        let names: Vec<String> = journal.iter().rev().take(3).map(|c| c.to_string_lossy().to_lowercase()).collect();
        names == ["elite dangerous", "frontier developments", "saved games"]
    });
    #[cfg(target_os = "windows")]
    let from_journal = from_journal.or_else(|| {
        std::env::var("LOCALAPPDATA").ok().map(|l| PathBuf::from(l).join("Frontier Developments").join("Elite Dangerous").join("Options").join("Graphics"))
    });
    from_journal.filter(|dir| dir.is_dir())
}

// ─── Reading and writing ─────────────────────────────────────────────────────

/// What a write did, for the settings to show.
#[derive(Debug, PartialEq)]
pub struct Written {
    pub path: PathBuf,
    /// Overrides EDCAS wrote.
    pub ours: usize,
    /// Other values in the file, left as they were.
    pub others: usize,
}

/// The overrides in the player's file EDCAS offers but the player hasn't touched yet,
/// opted in with the file's value: so turning management on keeps what they set by hand.
pub fn adopt_existing(settings: &mut GameGraphicsSettings, dir: &Path) -> Result<usize, String> {
    let path = dir.join(FILE_NAME);
    if !path.exists() {
        return Ok(0);
    }
    let doc = Doc::read(&path)?;
    let mut adopted = 0;
    for o in OPTIONS {
        if settings.options.contains_key(o.id) {
            continue;
        }
        if let Some(value) = o.levels.iter().rev().find_map(|level| doc.root.value(o.section, level, o.key)) {
            settings.options.insert(o.id.to_string(), OverrideChoice { enabled: true, value });
            adopted += 1;
        }
    }
    Ok(adopted)
}

/// Writes the player's overrides into the file in `dir`: opted-in ones set in every level,
/// opted-out ones removed; everything else stays. Keeps a copy of the original first.
pub fn write(settings: &GameGraphicsSettings, dir: &Path) -> Result<Written, String> {
    if !dir.is_dir() {
        return Err(format!("{} doesn't exist", dir.display()));
    }
    let path = dir.join(FILE_NAME);
    let mut doc = if path.exists() { Doc::read(&path)? } else { Doc::new() };
    let mut ours = 0;
    for o in OPTIONS {
        match settings.options.get(o.id) {
            Some(choice) if choice.enabled => {
                let value = o.kind.normalize(&choice.value).unwrap_or_else(|_| choice.value.trim().to_string());
                for level in o.levels {
                    doc.root.set(o.section, level, o.key, &value);
                }
                ours += 1;
            }
            Some(_) => {
                for level in o.levels {
                    doc.root.remove(o.section, level, o.key);
                }
            }
            None => {}
        }
    }
    let others = doc.root.leaves().into_iter().filter(|(section, _, key)| !OPTIONS.iter().any(|o| o.section == *section && o.key == *key)).count();
    let backup = backup_path(dir);
    if path.exists() && !backup.exists() {
        std::fs::copy(&path, &backup).map_err(|e| format!("Couldn't keep a copy of the original: {e}"))?;
    }
    let tmp = dir.join(format!("{FILE_NAME}.tmp"));
    std::fs::write(&tmp, doc.to_xml()).map_err(|e| format!("Couldn't write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Couldn't write {}: {e}", path.display()))?;
    Ok(Written { path, ours, others })
}

pub fn backup_path(dir: &Path) -> PathBuf {
    dir.join(format!("{FILE_NAME}{BACKUP_SUFFIX}"))
}

/// Puts the file back as it was before EDCAS wrote it: the kept copy, or, when there was
/// no file before, the file without EDCAS's overrides.
pub fn restore(settings: &GameGraphicsSettings, dir: &Path) -> Result<String, String> {
    let path = dir.join(FILE_NAME);
    let backup = backup_path(dir);
    if backup.exists() {
        std::fs::copy(&backup, &path).map_err(|e| format!("Couldn't restore {}: {e}", path.display()))?;
        std::fs::remove_file(&backup).map_err(|e| format!("Restored, but couldn't remove {}: {e}", backup.display()))?;
        return Ok("Restored the file as it was before EDCAS changed it".to_string());
    }
    if !path.exists() {
        return Ok("There is no override file to restore".to_string());
    }
    let mut doc = Doc::read(&path)?;
    for o in OPTIONS.iter().filter(|o| settings.options.get(o.id).is_some_and(|c| c.enabled)) {
        for level in o.levels {
            doc.root.remove(o.section, level, o.key);
        }
    }
    if doc.root.children.iter().any(|n| matches!(n, Node::Element(_))) {
        std::fs::write(&path, doc.to_xml()).map_err(|e| format!("Couldn't write {}: {e}", path.display()))?;
        Ok("Removed EDCAS's overrides; the rest of the file stays".to_string())
    } else {
        std::fs::remove_file(&path).map_err(|e| format!("Couldn't remove {}: {e}", path.display()))?;
        Ok("Removed the override file EDCAS had made".to_string())
    }
}

// ─── A small XML tree that keeps comments ────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Element(Element),
    Comment(String),
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
struct Element {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<Node>,
}

struct Doc {
    /// Comments before the root element.
    before: Vec<String>,
    root: Element,
}

impl Doc {
    fn new() -> Self {
        Doc { before: Vec::new(), root: Element::new("GraphicsConfig") }
    }

    fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{} isn't valid XML ({e}); EDCAS leaves it alone", path.display()))
    }

    fn parse(text: &str) -> Result<Self, String> {
        let doc = roxmltree::Document::parse(text).map_err(|e| e.to_string())?;
        let mut before = Vec::new();
        for node in doc.root().children() {
            if node.is_element() {
                break;
            }
            if let Some(c) = node.is_comment().then(|| node.text()).flatten() {
                before.push(c.to_string());
            }
        }
        Ok(Doc { before, root: Element::from_ro(doc.root_element()) })
    }

    fn to_xml(&self) -> String {
        let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n");
        for c in &self.before {
            out.push_str(&format!("<!--{c}-->\n"));
        }
        self.root.write(&mut out, 0);
        out
    }
}

impl Element {
    fn new(name: &str) -> Self {
        Element { name: name.to_string(), attributes: Vec::new(), children: Vec::new() }
    }

    fn from_ro(node: roxmltree::Node) -> Self {
        let mut children = Vec::new();
        for child in node.children() {
            if child.is_element() {
                children.push(Node::Element(Element::from_ro(child)));
            } else if child.is_comment() {
                children.push(Node::Comment(child.text().unwrap_or_default().to_string()));
            } else if let Some(t) = child.text().map(str::trim).filter(|t| !t.is_empty()) {
                children.push(Node::Text(t.to_string()));
            }
        }
        Element {
            name: node.tag_name().name().to_string(),
            attributes: node.attributes().map(|a| (a.name().to_string(), a.value().to_string())).collect(),
            children,
        }
    }

    fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| if let Node::Element(e) = n { Some(e) } else { None })
    }

    fn elements_mut(&mut self) -> impl Iterator<Item = &mut Element> {
        self.children.iter_mut().filter_map(|n| if let Node::Element(e) = n { Some(e) } else { None })
    }

    fn text(&self) -> String {
        self.children.iter().filter_map(|n| if let Node::Text(t) = n { Some(t.as_str()) } else { None }).collect()
    }

    /// The value of `key` in `section`/`level`; the last one when the file has it twice,
    /// as that's the one the game ends up with.
    fn value(&self, section: &str, level: &str, key: &str) -> Option<String> {
        self.elements()
            .filter(|s| s.name == section)
            .flat_map(|s| s.elements().filter(|l| l.name == level))
            .flat_map(|l| l.elements().filter(|k| k.name == key))
            .last()
            .map(Element::text)
    }

    /// Sets `key` in every `section`/`level` of the file, adding them where missing.
    fn set(&mut self, section: &str, level: &str, key: &str, value: &str) {
        let mut found = false;
        for l in self.elements_mut().filter(|s| s.name == section).flat_map(|s| s.elements_mut().filter(|l| l.name == level)) {
            found = true;
            if l.elements().any(|k| k.name == key) {
                for k in l.elements_mut().filter(|k| k.name == key) {
                    k.children = vec![Node::Text(value.to_string())];
                }
            } else {
                l.children.push(Node::Element(Element::leaf(key, value)));
            }
        }
        if found {
            return;
        }
        let mut new_level = Element::new(level);
        new_level.children.push(Node::Element(Element::leaf(key, value)));
        if let Some(s) = self.elements_mut().find(|s| s.name == section) {
            s.children.push(Node::Element(new_level));
            return;
        }
        let mut s = Element::new(section);
        s.children.push(Node::Element(new_level));
        self.children.push(Node::Element(s));
    }

    /// Removes `key` from every `section`/`level`, then the levels and sections it leaves
    /// with nothing to override.
    fn remove(&mut self, section: &str, level: &str, key: &str) {
        for s in self.elements_mut().filter(|s| s.name == section) {
            let mut emptied = false;
            for l in s.elements_mut().filter(|l| l.name == level) {
                let before = l.children.len();
                l.children.retain(|n| !matches!(n, Node::Element(k) if k.name == key));
                emptied |= l.children.len() < before && l.elements().all(|k| k.name == "LocalisationName");
            }
            if emptied {
                s.children.retain(|n| !matches!(n, Node::Element(l) if l.name == level && l.elements().all(|k| k.name == "LocalisationName")));
            }
        }
        self.children.retain(|n| !matches!(n, Node::Element(s) if s.name == section && s.elements().next().is_none()));
    }

    fn leaf(name: &str, value: &str) -> Self {
        let mut e = Element::new(name);
        e.children.push(Node::Text(value.to_string()));
        e
    }

    /// Every value in the file, as (section, level, key).
    fn leaves(&self) -> Vec<(&str, &str, &str)> {
        let mut out = Vec::new();
        for s in self.elements() {
            for l in s.elements() {
                for k in l.elements().filter(|k| k.name != "LocalisationName") {
                    out.push((s.name.as_str(), l.name.as_str(), k.name.as_str()));
                }
            }
        }
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        let indent = "\t".repeat(depth);
        out.push_str(&indent);
        out.push('<');
        out.push_str(&self.name);
        for (k, v) in &self.attributes {
            out.push_str(&format!(" {k}=\"{}\"", escape(v)));
        }
        let only_text = self.children.iter().all(|n| matches!(n, Node::Text(_)));
        if self.children.is_empty() {
            out.push_str(" />\n");
        } else if only_text {
            out.push_str(&format!(">{}</{}>\n", escape(&self.text()), self.name));
        } else {
            out.push_str(">\n");
            for child in &self.children {
                match child {
                    Node::Element(e) => e.write(out, depth + 1),
                    Node::Comment(c) => out.push_str(&format!("{indent}\t<!--{c}-->\n")),
                    Node::Text(t) => out.push_str(&format!("{indent}\t{}\n", escape(t))),
                }
            }
            out.push_str(&format!("{indent}</{}>\n", self.name));
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(id: &str) -> Option<&'static GraphicsOption> {
        OPTIONS.iter().find(|o| o.id == id)
    }

    /// The opted-in overrides, by option id.
    fn enabled(settings: &GameGraphicsSettings) -> std::collections::BTreeMap<&'static str, String> {
        OPTIONS
            .iter()
            .filter_map(|o| settings.options.get(o.id).filter(|c| c.enabled).map(|c| (o.id, c.value.clone())))
            .collect()
    }

    const PLAYERS: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!-- my tweaks -->
<GraphicsConfig>
    <GalaxyMap>
    	<High>
    		<LocalisationName>$QUALITY_HIGH; Custom</LocalisationName>
    		<NebulasCount>100</NebulasCount>
    		<MilkyWayInstancesOffscreenRTEnabled
            >false</MilkyWayInstancesOffscreenRTEnabled>
    		<StarInstanceCount>52000</StarInstanceCount>
    	</High>
    </GalaxyMap>
	<!-- sharper sky -->
	<GalaxyBackground>
		<High>
			<TextureSize>4096</TextureSize>
		</High>
	</GalaxyBackground>
	<Bloom>
		<Ultra>
			<GlareScale>0.10</GlareScale>
		</Ultra>
	</Bloom>
</GraphicsConfig>
"#;

    fn dir_with(text: Option<&str>) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("edcas-gfx-{}-{}", std::process::id(), rand_id()));
        std::fs::create_dir_all(&dir).unwrap();
        if let Some(t) = text {
            std::fs::write(dir.join(FILE_NAME), t).unwrap();
        }
        dir
    }

    fn rand_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        N.fetch_add(1, Ordering::Relaxed)
    }

    #[test]
    fn values_are_checked_and_tidied() {
        let stars = option("galaxy_map_stars").unwrap().kind;
        assert_eq!(stars.normalize("52,000"), Ok("52000".to_string()));
        assert!(stars.normalize("500").is_err());
        let brightness = option("milky_way_brightness").unwrap().kind;
        assert_eq!(brightness.normalize("1,50"), Ok("1.5".to_string()));
        assert_eq!(Kind::Row.normalize("0.2 0.5 1"), Ok("0.2, 0.5, 1".to_string()));
        assert!(Kind::Row.normalize("1, 0").is_err());
        assert_eq!(stars.range(), "1,000–128,000");
    }

    #[test]
    fn the_players_own_values_are_adopted_and_kept() {
        let dir = dir_with(Some(PLAYERS));
        let mut s = GameGraphicsSettings { manage: true, ..Default::default() };
        assert_eq!(adopt_existing(&mut s, &dir), Ok(3));
        let on = enabled(&s);
        assert_eq!(on["galaxy_map_stars"], "52000");
        assert_eq!(on["galaxy_map_nebulae"], "100");
        assert_eq!(on["sky_texture"], "4096");

        // Raise the stars, opt out of the nebulae, opt in to planet textures.
        s.options.get_mut("galaxy_map_stars").unwrap().value = "64000".into();
        s.options.get_mut("galaxy_map_nebulae").unwrap().enabled = false;
        s.options.insert("planet_textures".into(), OverrideChoice { enabled: true, value: "4096".into() });
        let written = write(&s, &dir).unwrap();
        assert_eq!((written.ours, written.others), (3, 2));

        let text = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        let doc = Doc::parse(&text).unwrap();
        for level in ["Low", "Medium", "High"] {
            assert_eq!(doc.root.value("GalaxyMap", level, "StarInstanceCount").as_deref(), Some("64000"));
        }
        assert_eq!(doc.root.value("GalaxyMap", "High", "NebulasCount"), None);
        assert_eq!(doc.root.value("Planets", "Ultra", "TextureSize").as_deref(), Some("4096"));
        // What EDCAS doesn't manage stays, comments included.
        assert_eq!(doc.root.value("Bloom", "Ultra", "GlareScale").as_deref(), Some("0.10"));
        assert_eq!(doc.root.value("GalaxyMap", "High", "MilkyWayInstancesOffscreenRTEnabled").as_deref(), Some("false"));
        assert!(text.contains("<!-- my tweaks -->") && text.contains("<!-- sharper sky -->"));
        // The original is kept, once.
        assert_eq!(std::fs::read_to_string(backup_path(&dir)).unwrap(), PLAYERS);

        assert!(restore(&s, &dir).is_ok());
        assert_eq!(std::fs::read_to_string(dir.join(FILE_NAME)).unwrap(), PLAYERS);
        assert!(!backup_path(&dir).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_file_edcas_made_goes_away_on_restore() {
        let dir = dir_with(None);
        let mut s = GameGraphicsSettings { manage: true, ..Default::default() };
        s.options.insert("hud_red".into(), OverrideChoice { enabled: true, value: "0 0 1".into() });
        write(&s, &dir).unwrap();
        let doc = Doc::read(&dir.join(FILE_NAME)).unwrap();
        assert_eq!(doc.root.value("GUIColour", "Default", "MatrixRed").as_deref(), Some("0, 0, 1"));
        assert!(!backup_path(&dir).exists());
        restore(&s, &dir).unwrap();
        assert!(!dir.join(FILE_NAME).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn opting_out_leaves_no_empty_blocks() {
        let mut root = Element::new("GraphicsConfig");
        root.set("Envmap", "High", "TextureSize", "1024");
        assert_eq!(root.value("Envmap", "High", "TextureSize").as_deref(), Some("1024"));
        root.remove("Envmap", "High", "TextureSize");
        assert!(root.children.is_empty());
    }

    #[test]
    fn the_graphics_folder_is_found_from_the_journal_folder() {
        let base = std::env::temp_dir().join(format!("edcas-gfx-home-{}-{}", std::process::id(), rand_id()));
        let journal = base.join("Saved Games").join("Frontier Developments").join("Elite Dangerous");
        let graphics = base.join("AppData/Local/Frontier Developments/Elite Dangerous/Options/Graphics");
        std::fs::create_dir_all(&journal).unwrap();
        std::fs::create_dir_all(&graphics).unwrap();
        let mut settings = Settings::template_defaults();
        settings.journal_reader.journal_directory = journal.to_string_lossy().into_owned();
        assert_eq!(graphics_dir(&settings), Some(graphics.clone()));
        settings.game_graphics.folder = "/somewhere/else".into();
        assert_eq!(graphics_dir(&settings), Some(PathBuf::from("/somewhere/else")));
        std::fs::remove_dir_all(base).unwrap();
    }
}
