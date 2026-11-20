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
/// quality level of its sections so it holds whatever the game is set to. A key that lies
/// deeper in a level is given with the blocks it is in: `Cascade/CullingBias/ExtentScale`.
/// A section without levels (the game names the quality in the section itself, as in
/// `SpotShadows_Ultra`) has the one level `""`.
#[derive(Debug)]
pub struct GraphicsOption {
    pub id: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    /// What it costs the graphics card, in work and in memory, as far as it is known: counted
    /// in a capture, reckoned from the sizes, taken from players' reports, or not measured.
    pub cost: &'static str,
    pub sections: &'static [&'static str],
    pub levels: &'static [&'static str],
    pub key: &'static str,
    /// The game's own value at its highest level.
    pub game: &'static str,
    /// What an opt-in starts from.
    pub suggested: &'static str,
    /// The values the settings offer to pick from, rising, the game's own and the suggested
    /// one among them; none where a value is typed (the HUD's colour rows).
    pub choices: &'static [&'static str],
    pub kind: Kind,
}

const GALAXY_MAP: &[&str] = &["Low", "Medium", "High"];
const BACKGROUND: &[&str] = &["Low", "Medium", "High"];
const PLANETS: &[&str] = &["Low", "Medium", "High", "Ultra"];
const ENVMAP: &[&str] = &["Low", "High"];
const GUI_COLOUR: &[&str] = &["Default"];
/// The sun's shadows: the game names the quality in the section and has a profile for each
/// situation in it. The two on a planet's surface.
const SUN_SHADOWS: &[&str] = &["DirectionalShadows_Low", "DirectionalShadows_Medium", "DirectionalShadows_High", "DirectionalShadows_Ultra"];
const ON_PLANET: &[&str] = &["Profile_Planet_Cockpit", "Profile_Planet_Humanoid"];
/// The lamps' shadows: the quality is in the section's name, and the values lie right in it.
const LAMP_SHADOWS: &[&str] = &["SpotShadows_Low", "SpotShadows_Medium", "SpotShadows_High", "SpotShadows_Ultra"];
const IN_SECTION: &[&str] = &[""];
/// The override file changes values the game's file has and adds none (players' finding), so a
/// key is written only into the levels that have it: the blur's radii are not in every bloom.
const BLOOM: &[&str] = &["Medium", "High", "Ultra"];
const BLOOM_HIGH: &[&str] = &["High", "Ultra"];
const BLOOM_ULTRA: &[&str] = &["Ultra"];
const VOLUMETRICS: &[&str] = &["Low", "Mid", "High", "Ultra"];
const DEBRIS: &[&str] = &["Off", "Low", "Medium", "High"];

/// The overrides offered, in the order the settings show them. Game values are those of
/// the game's `GraphicsConfiguration.xml` (Odyssey 4.x).
pub const OPTIONS: &[GraphicsOption] = &[
    GraphicsOption {
        id: "galaxy_map_stars",
        group: "Galaxy map",
        label: "Galaxy map stars",
        help: "Stars the galaxy map draws around the selected system. More fill the map; very high counts can stutter when the map opens.",
        cost: "GPU: only while the galaxy map is open. Very high counts stutter as it opens. VRAM: not measured, small.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "StarInstanceCount",
        game: "4000",
        suggested: "32000",
        choices: &["4000", "8000", "16000", "32000", "64000", "128000"],
        kind: Kind::Int { min: 1_000, max: 128_000 },
    },
    GraphicsOption {
        id: "galaxy_map_nebulae",
        group: "Galaxy map",
        label: "Galaxy map nebulae",
        help: "Nebulae the galaxy map draws.",
        cost: "GPU: only while the galaxy map is open. VRAM: not measured.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "NebulasCount",
        game: "30",
        suggested: "100",
        choices: &["30", "50", "100", "200"],
        kind: Kind::Int { min: 0, max: 200 },
    },
    GraphicsOption {
        id: "galaxy_map_background_nebulae",
        group: "Galaxy map",
        label: "Background nebulae",
        help: "Nebulae the galaxy map draws in the distance, behind the area you look at (the game's NebulasInBackgroundCount).",
        cost: "GPU: only while the galaxy map is open. VRAM: not measured.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "NebulasInBackgroundCount",
        game: "40",
        suggested: "100",
        choices: &["40", "70", "100", "200"],
        kind: Kind::Int { min: 0, max: 200 },
    },
    GraphicsOption {
        id: "galaxy_map_nebulae_low_res",
        group: "Galaxy map",
        label: "Nebulae in low detail",
        help: "How many of the galaxy map's nebulae are drawn in low detail (the game's LowResNebulasCount).",
        cost: "GPU: only while the galaxy map is open. VRAM: not measured.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "LowResNebulasCount",
        game: "16",
        suggested: "50",
        choices: &["16", "32", "50", "100"],
        kind: Kind::Int { min: 0, max: 200 },
    },
    GraphicsOption {
        id: "galaxy_map_nebulae_high_res",
        group: "Galaxy map",
        label: "Nebulae in high detail",
        help: "How many of the galaxy map's nebulae are drawn in high detail (the game's HighResNebulasCount). The game draws one.",
        cost: "GPU: only while the galaxy map is open. VRAM: not measured; a nebula in high detail takes more than one in low detail.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "HighResNebulasCount",
        game: "1",
        suggested: "50",
        choices: &["1", "4", "16", "50"],
        kind: Kind::Int { min: 0, max: 200 },
    },
    GraphicsOption {
        id: "galaxy_map_nebula_samples_low",
        group: "Galaxy map",
        label: "Nebula samples, low detail",
        help: "Samples the galaxy map takes through a nebula in low detail (the game's LowResSamplesCount). More are smoother and slower.",
        cost: "GPU: only while the galaxy map is open. The work goes with the count; players report that above about 100 it rises fast for little gain. VRAM: none.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "LowResSamplesCount",
        game: "35",
        suggested: "276",
        choices: &["35", "64", "128", "276"],
        kind: Kind::Int { min: 8, max: 1_024 },
    },
    GraphicsOption {
        id: "galaxy_map_nebula_samples_high",
        group: "Galaxy map",
        label: "Nebula samples, high detail",
        help: "Samples the galaxy map takes through a nebula in high detail (the game's HighResSamplesCount). More are smoother and slower.",
        cost: "GPU: only while the galaxy map is open. The work goes with the count; players report that above about 100 it rises fast for little gain. VRAM: none.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "HighResSamplesCount",
        game: "138",
        suggested: "552",
        choices: &["138", "276", "552"],
        kind: Kind::Int { min: 8, max: 2_048 },
    },
    GraphicsOption {
        id: "galaxy_map_nebula_size_low",
        group: "Galaxy map",
        label: "Nebula size, low detail",
        help: "Resolution of a nebula in low detail in the galaxy map (the game's LowResNebulaDimensions). A report from 2016 says changing it made nebulae vanish; players change it in Odyssey.",
        cost: "GPU: only while the galaxy map is open. VRAM: not measured; if a nebula is kept as a volume, each doubling takes eight times the memory.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "LowResNebulaDimensions",
        game: "64",
        suggested: "128",
        choices: &["64", "128", "256"],
        kind: Kind::Int { min: 16, max: 512 },
    },
    GraphicsOption {
        id: "galaxy_map_nebula_size_high",
        group: "Galaxy map",
        label: "Nebula size, high detail",
        help: "Resolution of a nebula in high detail in the galaxy map (the game's HighResNebulaDimensions). A report from 2016 says changing it made nebulae vanish; players change it in Odyssey.",
        cost: "GPU: only while the galaxy map is open. VRAM: not measured; if a nebula is kept as a volume, each doubling takes eight times the memory.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "HighResNebulaDimensions",
        game: "256",
        suggested: "512",
        choices: &["256", "512", "1024"],
        kind: Kind::Int { min: 64, max: 2_048 },
    },
    GraphicsOption {
        id: "milky_way_points",
        group: "Galaxy map",
        label: "Milky Way points",
        help: "Points the galaxy map draws the Milky Way's glow from (the game's MilkyWayInstancesCount). 0 takes the glow away.",
        cost: "GPU: only while the galaxy map is open. VRAM: not measured, small.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "MilkyWayInstancesCount",
        game: "16000",
        suggested: "64000",
        choices: &["0", "16000", "32000", "64000"],
        kind: Kind::Int { min: 0, max: 128_000 },
    },
    GraphicsOption {
        id: "milky_way_brightness",
        group: "Galaxy map",
        label: "Milky Way brightness",
        help: "How bright the galaxy's band of stars shows in the galaxy map.",
        cost: "GPU: none. VRAM: none.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "MilkywayInstancesBrightness",
        game: "1.0",
        suggested: "1.5",
        choices: &["0.5", "1.0", "1.5", "2", "3"],
        kind: Kind::Float { min: 0.1, max: 4.0 },
    },
    GraphicsOption {
        id: "milky_way_size",
        group: "Galaxy map",
        label: "Milky Way star size",
        help: "How big the galaxy's stars are drawn in the galaxy map.",
        cost: "GPU: only while the galaxy map is open. Larger stars cover more pixels. VRAM: none.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "MilkywayInstancesSize",
        game: "1.0",
        suggested: "2.0",
        choices: &["0.25", "0.5", "1.0", "2.0", "3"],
        kind: Kind::Float { min: 0.25, max: 4.0 },
    },
    GraphicsOption {
        id: "local_dust",
        group: "Galaxy map",
        label: "Local dust brightness",
        help: "Dust clouds around the selected system in the galaxy map. The game shows none.",
        cost: "GPU: none to speak of. VRAM: none.",
        sections: &["GalaxyMap"],
        levels: GALAXY_MAP,
        key: "LocalDustBrightness",
        game: "0.0",
        suggested: "0.1",
        choices: &["0.0", "0.05", "0.1", "0.2", "0.5"],
        kind: Kind::Float { min: 0.0, max: 1.0 },
    },
    GraphicsOption {
        id: "sky_texture",
        group: "Sky and planets",
        label: "Sky texture size",
        help: "Resolution of the galaxy in the background of space. 4096 is sharp on big screens.",
        cost: "GPU: none while flying; the sky is drawn anew at each jump, which takes longer with a larger one. VRAM: about 12 MB at 2048, 48 MB at 4096 (counted in a capture), 190 MB at 8192.",
        sections: &["GalaxyBackground"],
        levels: BACKGROUND,
        key: "TextureSize",
        game: "2048",
        suggested: "4096",
        choices: &["2048", "4096", "8192"],
        kind: Kind::Int { min: 512, max: 8_192 },
    },
    GraphicsOption {
        id: "planet_textures",
        group: "Sky and planets",
        label: "Planet texture size",
        help: "Resolution of planets seen from space.",
        cost: "GPU: none once they are made; making them on approach takes longer. VRAM: goes with the square of the size, 4096 takes 2.6 times what the game's 2560 does; megabytes not measured.",
        sections: &["Planets"],
        levels: PLANETS,
        key: "TextureSize",
        game: "2560",
        suggested: "4096",
        choices: &["2560", "4096", "8192"],
        kind: Kind::Int { min: 512, max: 8_192 },
    },
    GraphicsOption {
        id: "planet_work",
        group: "Sky and planets",
        label: "Planet work per frame",
        help: "How much planet surface the game builds each frame: higher loads detail sooner on approach and costs some frame rate.",
        cost: "GPU and CPU: more work each frame while a planet's surface loads; too high stutters. VRAM: none.",
        sections: &["Planets"],
        levels: PLANETS,
        key: "WorkPerFrame",
        game: "328",
        suggested: "512",
        choices: &["328", "512", "768", "1024"],
        kind: Kind::Int { min: 64, max: 2_048 },
    },
    GraphicsOption {
        id: "planet_atmosphere_steps",
        group: "Sky and planets",
        label: "Atmosphere steps",
        help: "Samples the game takes through a planet's air seen from space (the game's AtmosphereSteps). More give a smoother haze at the planet's edge.",
        cost: "GPU: one more sample per step for each pixel of air: 7 for the game's 6 is a sixth more work in that pass. VRAM: none.",
        sections: &["Planets"],
        levels: PLANETS,
        key: "AtmosphereSteps",
        game: "6",
        suggested: "7",
        choices: &["6", "7", "8", "10"],
        kind: Kind::Int { min: 2, max: 16 },
    },
    GraphicsOption {
        id: "reflection_size",
        group: "Sky and planets",
        label: "Reflection map size",
        help: "Resolution of the reflections on ships and stations.",
        cost: "GPU: more to draw each time a reflection map is renewed. VRAM: about 3 MB at the game's 256, 40 MB at 1024 (reckoned from a capture), 160 MB at 2048.",
        sections: &["Envmap"],
        levels: ENVMAP,
        key: "TextureSize",
        game: "256",
        suggested: "1024",
        choices: &["256", "512", "1024", "2048"],
        kind: Kind::Int { min: 128, max: 2_048 },
    },
    GraphicsOption {
        id: "reflection_mips",
        group: "Sky and planets",
        label: "Reflection map levels",
        help: "Blur levels of the reflection map (the game's NumMips), for rough surfaces. Goes with the reflection map size: a map of 1024 has 11 levels, one of 2048 has 12.",
        cost: "GPU: none to speak of. VRAM: all levels together add a third to the map at most.",
        sections: &["Envmap"],
        levels: ENVMAP,
        key: "NumMips",
        game: "8",
        suggested: "11",
        choices: &["8", "9", "10", "11", "12"],
        kind: Kind::Int { min: 1, max: 16 },
    },
    // The lit rectangle round the camera at a low sun (2026-11-18, `ed_frame35410.rdc`: in a crater
    // whose wall hides the sun the ground is lit in a rectangle with a square corner round the camera
    // and shadowed outside it, in the picture and in the game's own shadow texture alike, and what is
    // lit changes as the camera tilts). The game draws the sun's shadows in six nested boxes round the
    // view and puts into each only the terrain inside a volume round it: `CullingBias`, a distance
    // towards the sun and a width in box sizes (at Ultra 12 500 m on foot, 15 000 m in the cockpit,
    // 8 boxes wide; at Low 5 000 m and 3). Read from the game's file; that these two are what leaves
    // the wall out, and that the game takes them from the override file, is not tested.
    GraphicsOption {
        id: "sun_shadow_distance",
        group: "Sun shadows on planets (experimental)",
        label: "Shadow caster distance (m)",
        help: "How far towards the sun the game looks for terrain that casts a shadow near you. At a low sun a crater wall or ridge farther off casts none: the ground round the camera is lit in a rectangle that changes as you look around. Not tested yet whether a higher value cures it; more terrain is drawn into the shadows, which costs frame rate.",
        cost: "GPU: more terrain is drawn into the six shadow maps each frame; how much is not measured. VRAM: none.",
        sections: SUN_SHADOWS,
        levels: ON_PLANET,
        key: "Cascade/CullingBias/FixedOffset",
        game: "15000",
        suggested: "50000",
        choices: &["15000", "25000", "50000", "100000"],
        kind: Kind::Int { min: 1_000, max: 200_000 },
    },
    GraphicsOption {
        id: "sun_shadow_width",
        group: "Sun shadows on planets (experimental)",
        label: "Shadow caster width (boxes)",
        help: "How wide round each of its shadow boxes the game looks for terrain that casts a shadow into it, in box sizes. Goes with the distance towards the sun; not tested yet either, and it costs frame rate too.",
        cost: "GPU: more terrain is drawn into the six shadow maps each frame; how much is not measured. VRAM: none.",
        sections: SUN_SHADOWS,
        levels: ON_PLANET,
        key: "Cascade/CullingBias/ExtentScale",
        game: "8",
        suggested: "32",
        choices: &["8", "16", "32", "64"],
        kind: Kind::Int { min: 1, max: 128 },
    },
    GraphicsOption {
        id: "lamp_shadow_size",
        group: "Lamp shadows",
        label: "Lamp shadow size",
        help: "Resolution of the shadow each lamp casts: ship lights, lamps of stations and settlements (the game's CacheShadowSize). Larger are sharper. All of them share the atlas below: one of 8192 holds 256 shadows of 512, 64 of 1024 or 16 of 2048.",
        cost: "GPU: four times the pixels to draw for a lamp's shadow with each doubling. VRAM: none by itself, the atlas holds them. Players report that a 4 GB card managed 1024, not 2048.",
        sections: LAMP_SHADOWS,
        levels: IN_SECTION,
        key: "CacheShadowSize",
        game: "512",
        suggested: "1024",
        choices: &["512", "1024", "2048"],
        kind: Kind::Int { min: 128, max: 4_096 },
    },
    GraphicsOption {
        id: "lamp_shadow_atlas",
        group: "Lamp shadows",
        label: "Lamp shadow atlas",
        help: "The one texture all lamp shadows are kept in (the game's CacheShadowAtlasSize: 3072 to 8192 by its shadow setting). A larger one holds more shadows, or larger ones.",
        cost: "GPU: none by itself. VRAM: 64 MB at 4096, 256 MB at 8192 (counted in a capture), 1 GB at 16384.",
        sections: LAMP_SHADOWS,
        levels: IN_SECTION,
        key: "CacheShadowAtlasSize",
        game: "8192",
        suggested: "8192",
        choices: &["4096", "8192", "16384"],
        kind: Kind::Int { min: 1_024, max: 16_384 },
    },
    GraphicsOption {
        id: "bloom_size",
        group: "Bloom",
        label: "Bloom size",
        help: "How large the glow round bright lights is (the game's GlareScale). Lower gives less bloom.",
        cost: "GPU: none, the same passes run. VRAM: none.",
        sections: &["Bloom"],
        levels: BLOOM,
        key: "GlareScale",
        game: "0.04",
        suggested: "0.025",
        choices: &["0.01", "0.025", "0.04", "0.07", "0.1"],
        kind: Kind::Float { min: 0.0, max: 0.2 },
    },
    GraphicsOption {
        id: "bloom_blur",
        group: "Bloom",
        label: "Bloom blur",
        help: "How wide the glow is blurred (the game's FilterRadius; bloom at High and Ultra). Lower keeps it close to the light.",
        cost: "GPU: not measured; no pass is added. VRAM: none.",
        sections: &["Bloom"],
        levels: BLOOM_HIGH,
        key: "FilterRadius",
        game: "3.0",
        suggested: "1.0",
        choices: &["1.0", "2", "3.0", "4"],
        kind: Kind::Float { min: 0.5, max: 8.0 },
    },
    GraphicsOption {
        id: "bloom_blur_wide",
        group: "Bloom",
        label: "Bloom wide blur",
        help: "How wide the glow's outer haze is blurred (the game's FilterRadiusWide; bloom at Ultra only).",
        cost: "GPU: not measured; no pass is added. VRAM: none.",
        sections: &["Bloom"],
        levels: BLOOM_ULTRA,
        key: "FilterRadiusWide",
        game: "4.0",
        suggested: "3.5",
        choices: &["3", "3.5", "4.0", "5"],
        kind: Kind::Float { min: 0.5, max: 10.0 },
    },
    GraphicsOption {
        id: "fog_steps",
        group: "Fog and debris",
        label: "Fog steps",
        help: "Steps the game takes through fog and lit haze for each pixel, in stations, settlements and on some planets (the game's StepsPerPixel: 24 at Ultra, 32 at High, 64 at half size below). Fewer are faster and grainier.",
        cost: "GPU: the fog pass's work goes with the count: 16 for Ultra's 24 is a third less. How much of a frame that pass is, is not measured. VRAM: none.",
        sections: &["Volumetrics"],
        levels: VOLUMETRICS,
        key: "StepsPerPixel",
        game: "24",
        suggested: "16",
        choices: &["8", "16", "24", "32", "64"],
        kind: Kind::Int { min: 4, max: 128 },
    },
    GraphicsOption {
        id: "debris_limit",
        group: "Fog and debris",
        label: "Debris pieces",
        help: "The most pieces of debris the game keeps at once, as from wrecks and mining (the game's DebrisLimit: 100 to 3000 by its debris setting).",
        cost: "GPU and CPU: fewer pieces to move and draw where there is debris, nothing elsewhere; not measured. VRAM: none to speak of.",
        sections: &["Debris"],
        levels: DEBRIS,
        key: "DebrisLimit",
        game: "3000",
        suggested: "2000",
        choices: &["500", "1000", "2000", "3000", "5000"],
        kind: Kind::Int { min: 0, max: 10_000 },
    },
    GraphicsOption {
        id: "hud_red",
        group: "HUD colour",
        label: "HUD red row",
        help: "How much of the HUD's red goes into red, green and blue. 1, 0, 0 leaves it as it is.",
        cost: "GPU: none. VRAM: none.",
        sections: &["GUIColour"],
        levels: GUI_COLOUR,
        key: "MatrixRed",
        game: "1, 0, 0",
        suggested: "1, 0, 0",
        choices: &[],
        kind: Kind::Row,
    },
    GraphicsOption {
        id: "hud_green",
        group: "HUD colour",
        label: "HUD green row",
        help: "How much of the HUD's green goes into red, green and blue. 0, 1, 0 leaves it as it is.",
        cost: "GPU: none. VRAM: none.",
        sections: &["GUIColour"],
        levels: GUI_COLOUR,
        key: "MatrixGreen",
        game: "0, 1, 0",
        suggested: "0, 1, 0",
        choices: &[],
        kind: Kind::Row,
    },
    GraphicsOption {
        id: "hud_blue",
        group: "HUD colour",
        label: "HUD blue row",
        help: "How much of the HUD's blue goes into red, green and blue. 0, 0, 1 leaves it as it is.",
        cost: "GPU: none. VRAM: none.",
        sections: &["GUIColour"],
        levels: GUI_COLOUR,
        key: "MatrixBlue",
        game: "0, 0, 1",
        suggested: "0, 0, 1",
        choices: &[],
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

/// `1.5`, `2`, `0.1`, `0.0225`: no trailing zeros.
fn number(x: f32) -> String {
    let s = format!("{x:.4}");
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
        if let Some(value) = o.sections.iter().rev().find_map(|section| o.levels.iter().rev().find_map(|level| doc.root.value(section, level, o.key))) {
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
                for section in o.sections {
                    for level in o.levels {
                        doc.root.set(section, level, o.key, &value);
                    }
                }
                ours += 1;
            }
            Some(_) => {
                for section in o.sections {
                    for level in o.levels {
                        doc.root.remove(section, level, o.key);
                    }
                }
            }
            None => {}
        }
    }
    let others = doc.root.leaves().into_iter().filter(|(section, _, key)| !OPTIONS.iter().any(|o| o.sections.contains(&section.as_str()) && o.key == *key)).count();
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
        for section in o.sections {
            for level in o.levels {
                doc.root.remove(section, level, o.key);
            }
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
    /// as that's the one the game ends up with. `key` names the blocks it lies in, if any
    /// (`Cascade/CullingBias/ExtentScale`).
    fn value(&self, section: &str, level: &str, key: &str) -> Option<String> {
        let mut at: Vec<&Element> = self.elements().filter(|s| s.name == section).collect();
        if !level.is_empty() {
            at = at.into_iter().flat_map(|s| s.elements().filter(|l| l.name == level)).collect();
        }
        for name in key.split('/') {
            at = at.into_iter().flat_map(|e| e.elements().filter(move |k| k.name == name)).collect();
        }
        at.last().map(|e| e.text())
    }

    /// Sets `key` in every `section`/`level` of the file, adding them, and the blocks the
    /// key lies in, where missing.
    fn set(&mut self, section: &str, level: &str, key: &str, value: &str) {
        let path: Vec<&str> = key.split('/').collect();
        if !self.elements().any(|s| s.name == section) {
            self.children.push(Node::Element(Element::new(section)));
        }
        if level.is_empty() {
            for s in self.elements_mut().filter(|s| s.name == section) {
                s.set_in(&path, value);
            }
            return;
        }
        if !self.elements().filter(|s| s.name == section).any(|s| s.elements().any(|l| l.name == level)) {
            if let Some(s) = self.elements_mut().find(|s| s.name == section) {
                s.children.push(Node::Element(Element::new(level)));
            }
        }
        for l in self.elements_mut().filter(|s| s.name == section).flat_map(|s| s.elements_mut().filter(|l| l.name == level)) {
            l.set_in(&path, value);
        }
    }

    /// Sets the value at `path` below this element, in every block of its names.
    fn set_in(&mut self, path: &[&str], value: &str) {
        let Some((name, rest)) = path.split_first() else { return };
        if !self.elements().any(|k| k.name == *name) {
            self.children.push(Node::Element(Element::new(name)));
        }
        for k in self.elements_mut().filter(|k| k.name == *name) {
            if rest.is_empty() {
                k.children = vec![Node::Text(value.to_string())];
            } else {
                k.set_in(rest, value);
            }
        }
    }

    /// Removes `key` from every `section`/`level`, then the blocks, levels and sections it
    /// leaves with nothing to override.
    fn remove(&mut self, section: &str, level: &str, key: &str) {
        let path: Vec<&str> = key.split('/').collect();
        for s in self.elements_mut().filter(|s| s.name == section) {
            if level.is_empty() {
                s.remove_in(&path);
                continue;
            }
            let mut emptied = false;
            for l in s.elements_mut().filter(|l| l.name == level) {
                emptied |= l.remove_in(&path) && l.elements().all(|k| k.name == "LocalisationName");
            }
            if emptied {
                s.children.retain(|n| !matches!(n, Node::Element(l) if l.name == level && l.elements().all(|k| k.name == "LocalisationName")));
            }
        }
        self.children.retain(|n| !matches!(n, Node::Element(s) if s.name == section && s.elements().next().is_none()));
    }

    /// Removes the value at `path` below this element and the blocks of the path that are
    /// left empty by it. Whether anything went.
    fn remove_in(&mut self, path: &[&str]) -> bool {
        let Some((name, rest)) = path.split_first() else { return false };
        let before = self.children.len();
        if rest.is_empty() {
            self.children.retain(|n| !matches!(n, Node::Element(k) if k.name == *name));
            return self.children.len() < before;
        }
        let mut went = false;
        for k in self.elements_mut().filter(|k| k.name == *name) {
            went |= k.remove_in(rest);
        }
        if went {
            self.children.retain(|n| !matches!(n, Node::Element(k) if k.name == *name && k.children.is_empty()));
        }
        went
    }

    /// Every value in the file, as (section, level, key), the key with the blocks it lies in.
    fn leaves(&self) -> Vec<(String, String, String)> {
        fn below(e: &Element, prefix: &str, out: &mut Vec<String>) {
            for k in e.elements().filter(|k| k.name != "LocalisationName") {
                let key = if prefix.is_empty() { k.name.clone() } else { format!("{prefix}/{}", k.name) };
                if k.elements().next().is_some() {
                    below(k, &key, out);
                } else {
                    out.push(key);
                }
            }
        }
        let mut out = Vec::new();
        for s in self.elements() {
            for l in s.elements() {
                // A value right in a section that has no levels.
                if l.elements().next().is_none() && l.name != "LocalisationName" {
                    out.push((s.name.clone(), String::new(), l.name.clone()));
                    continue;
                }
                let mut keys = Vec::new();
                below(l, "", &mut keys);
                out.extend(keys.into_iter().map(|key| (s.name.clone(), l.name.clone(), key)));
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
			<MaxThreshold>25.00</MaxThreshold>
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
    fn every_option_stands_alone_and_suggests_a_value_it_takes() {
        let mut ids = std::collections::BTreeSet::new();
        let mut labels = std::collections::BTreeSet::new();
        let mut places = std::collections::BTreeSet::new();
        for o in OPTIONS {
            assert!(ids.insert(o.id), "{} twice", o.id);
            // The settings find an option by its label.
            assert!(labels.insert(o.label), "{} twice", o.label);
            for section in o.sections {
                assert!(places.insert((*section, o.key)), "{section}/{} twice", o.key);
            }
            assert!(!o.help.is_empty() && o.cost.contains("GPU") && o.cost.contains("VRAM"), "{}: what it does and what it costs", o.id);
            assert!(o.kind.normalize(o.suggested).is_ok(), "{}: suggested {}", o.id, o.suggested);
            // What can be picked: rising, each a value the rule takes, the game's and the suggested among them.
            if !o.choices.is_empty() {
                let picks: Vec<String> = o.choices.iter().map(|c| o.kind.normalize(c).unwrap_or_else(|e| panic!("{}: {c}: {e}", o.id))).collect();
                let numbers: Vec<f64> = picks.iter().map(|c| c.parse().unwrap()).collect();
                assert!(numbers.windows(2).all(|w| w[0] < w[1]), "{}: {picks:?} rise", o.id);
                for must in [o.game, o.suggested] {
                    assert!(picks.contains(&o.kind.normalize(must).unwrap()), "{}: {must} is not in {picks:?}", o.id);
                }
            }
            assert!(o.kind.normalize(o.game).is_ok(), "{}: the game's {}", o.id, o.game);
        }
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
        assert_eq!(doc.root.value("Bloom", "Ultra", "MaxThreshold").as_deref(), Some("25.00"));
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
    fn a_key_deep_in_a_level_is_set_beside_what_is_there_and_goes_again() {
        // The player's own shadow tweak: one cascade's size, in one profile of one quality.
        let players = r#"<?xml version="1.0" encoding="UTF-8" ?>
<GraphicsConfig>
	<DirectionalShadows_Ultra>
		<Profile_Planet_Humanoid>
			<Cascade>
				<CellResolution>4096</CellResolution>
			</Cascade>
		</Profile_Planet_Humanoid>
	</DirectionalShadows_Ultra>
</GraphicsConfig>
"#;
        let dir = dir_with(Some(players));
        let mut s = GameGraphicsSettings { manage: true, ..Default::default() };
        s.options.insert("sun_shadow_distance".into(), OverrideChoice { enabled: true, value: "50,000".into() });
        s.options.insert("sun_shadow_width".into(), OverrideChoice { enabled: true, value: "32".into() });
        let written = write(&s, &dir).unwrap();
        assert_eq!((written.ours, written.others), (2, 1));
        let doc = Doc::read(&dir.join(FILE_NAME)).unwrap();
        for quality in ["Low", "Medium", "High", "Ultra"] {
            for profile in ["Profile_Planet_Cockpit", "Profile_Planet_Humanoid"] {
                let section = format!("DirectionalShadows_{quality}");
                assert_eq!(doc.root.value(&section, profile, "Cascade/CullingBias/FixedOffset").as_deref(), Some("50000"), "{section} {profile}");
                assert_eq!(doc.root.value(&section, profile, "Cascade/CullingBias/ExtentScale").as_deref(), Some("32"), "{section} {profile}");
            }
            // The profiles in space are not written.
            assert_eq!(doc.root.value(&format!("DirectionalShadows_{quality}"), "Profile_Space_Cockpit", "Cascade/CullingBias/ExtentScale"), None);
        }
        // The player's value stays in the block EDCAS wrote into, and that block is there once.
        assert_eq!(doc.root.value("DirectionalShadows_Ultra", "Profile_Planet_Humanoid", "Cascade/CellResolution").as_deref(), Some("4096"));
        let text = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert_eq!(text.matches("<DirectionalShadows_Ultra>").count(), 1);
        assert_eq!(text.matches("<Cascade>").count(), 8);

        // A file written by hand with these values is taken over as it is.
        let mut fresh = GameGraphicsSettings { manage: true, ..Default::default() };
        assert_eq!(adopt_existing(&mut fresh, &dir), Ok(2));
        assert_eq!(enabled(&fresh)["sun_shadow_width"], "32");

        // Switched off, every block EDCAS made goes; the player's stays.
        for choice in s.options.values_mut() {
            choice.enabled = false;
        }
        write(&s, &dir).unwrap();
        let doc = Doc::read(&dir.join(FILE_NAME)).unwrap();
        assert_eq!(doc.root.leaves(), vec![("DirectionalShadows_Ultra".to_string(), "Profile_Planet_Humanoid".to_string(), "Cascade/CellResolution".to_string())]);
        assert_eq!(doc.root.elements().count(), 1);
        assert!(restore(&s, &dir).is_ok());
        assert_eq!(std::fs::read_to_string(dir.join(FILE_NAME)).unwrap(), players);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_value_right_in_a_section_and_one_only_some_levels_have() {
        let dir = dir_with(None);
        let mut s = GameGraphicsSettings { manage: true, ..Default::default() };
        s.options.insert("lamp_shadow_size".into(), OverrideChoice { enabled: true, value: "1024".into() });
        s.options.insert("bloom_blur_wide".into(), OverrideChoice { enabled: true, value: "3.5".into() });
        s.options.insert("bloom_size".into(), OverrideChoice { enabled: true, value: "0.0225".into() });
        let written = write(&s, &dir).unwrap();
        assert_eq!((written.ours, written.others), (3, 0));
        let doc = Doc::read(&dir.join(FILE_NAME)).unwrap();
        for quality in ["Low", "Medium", "High", "Ultra"] {
            assert_eq!(doc.root.value(&format!("SpotShadows_{quality}"), "", "CacheShadowSize").as_deref(), Some("1024"), "{quality}");
        }
        // The wide blur is in the Ultra bloom only; the size keeps its four decimals.
        assert_eq!(doc.root.value("Bloom", "Ultra", "FilterRadiusWide").as_deref(), Some("3.5"));
        assert_eq!(doc.root.value("Bloom", "High", "FilterRadiusWide"), None);
        assert_eq!(doc.root.value("Bloom", "Medium", "GlareScale").as_deref(), Some("0.0225"));
        let text = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert!(text.contains("\t<SpotShadows_Ultra>\n\t\t<CacheShadowSize>1024</CacheShadowSize>\n\t</SpotShadows_Ultra>"), "{text}");

        let mut fresh = GameGraphicsSettings { manage: true, ..Default::default() };
        assert_eq!(adopt_existing(&mut fresh, &dir), Ok(3));
        assert_eq!(enabled(&fresh)["lamp_shadow_size"], "1024");
        restore(&s, &dir).unwrap();
        assert!(!dir.join(FILE_NAME).exists());
        std::fs::remove_dir_all(dir).unwrap();
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
