//! Picture effects (most after CeeJay.dk's SweetFX), on Elite's tonemapped picture (`scripts/ed-shader/edcas_post*.hlsl`,
//! README.md "Post effects", 2026-11-25): anti-aliasing (SMAA 1x) and sharpening (CAS, LumaSharpen), deband,
//! colour looks (fake HDR, levels, lift/gamma/gain, DPX, Technicolor, curves, vibrance), a Gaussian blur, lens
//! effects (chromatic aberration, vignette, film grain, lens flare, streaks, diffraction spikes, lens dirt), clarity
//! and Daltonize for colour-blind players. They run after Elite's tonemap draw, before its 2D overlays, and are all
//! off unless the player switches one on. Beside them, the supersampling downscale (`edcas_downscale.hlsl`) replaces
//! Elite's copy of the picture to the screen. (A tonemapper with AgX and Khronos PBR Neutral curves was tried and
//! removed on 2026-11-26: Elite's stars are made for its own curve; README.md "Post effects".)
//!
//! Each effect has one IniParams row, its strength: 0 off, 1 its usual strength, up to 3 (SMAA and
//! lift/gamma/gain are on or off: 1 or 0). The client writes them into the values file like the other looks
//! ([`crate::ground_tint`]); `edcas.ini` runs a pass only while one of its effects is above 0.

use crate::settings::game_graphics::{GameGraphicsSettings, MAX_LOOK_PERCENT};

/// One effect: its name in [`crate::settings::game_graphics::PostEffects::list`], the settings page's
/// labels (the switch; the strength, for an effect that has one) and its IniParams row.
pub struct Effect {
    pub name: &'static str,
    pub label: &'static str,
    pub strength_label: Option<&'static str>,
    pub row: &'static str,
    /// What it does, for the settings page's hover.
    pub about: &'static str,
    /// The settings page's group it stands in ([`GROUPS`]).
    pub group: &'static str,
    /// The strength slider's top, percent: where more stops changing the picture, or starts breaking it (the
    /// owner's game screenshots of 2026-11-26: monochrome is done at 100, levels clips above 150).
    pub max: u16,
}

impl Effect {
    /// The same effect with its own top strength.
    const fn top(mut self, max: u16) -> Effect {
        self.max = max;
        self
    }
}

/// The settings page's groups of the post effects, in their order on the page.
pub const AA: &str = "Anti-aliasing & Sharpness";
pub const COLOUR: &str = "Colour";
pub const FILM: &str = "Film Looks";
pub const LENS: &str = "Lens";
pub const ACCESS: &str = "Accessibility";
pub const GROUPS: [&str; 5] = [AA, COLOUR, FILM, LENS, ACCESS];

const fn effect(name: &'static str, label: &'static str, strength_label: &'static str, row: &'static str, about: &'static str, group: &'static str) -> Effect {
    Effect { name, label, strength_label: Some(strength_label), row, about, group, max: MAX_LOOK_PERCENT }
}

/// An effect that is on or off, without a strength.
const fn switch(name: &'static str, label: &'static str, row: &'static str, about: &'static str, group: &'static str) -> Effect {
    Effect { name, label, strength_label: None, row, about, group, max: 100 }
}

/// The effects in the order the shaders apply them: deband, SMAA, sharpening,
/// the colour pass (clarity first), the blur, the lens pass (Daltonize last, over everything).
pub const EFFECTS: [Effect; 25] = [
    effect("deband", "Deband", "  Deband Strength (%)", "w602", "Smooths colour banding in skies, nebulae and glows", AA),
    switch("smaa", "SMAA Anti-aliasing", "x602", "Smooths the jagged edges of ships and stations", AA),
    effect("cas", "CAS Sharpening", "  CAS Strength (%)", "y602", "Sharpens fine detail without halos", AA),
    effect("luma_sharpen", "LumaSharpen", "  LumaSharpen Strength (%)", "z602", "Sharpens brightness detail, leaves colours alone", AA),
    effect("clarity", "Clarity", "  Clarity Strength (%)", "z611", "Mid-tone local contrast: surfaces and hulls gain depth", COLOUR).top(200),
    effect("fake_hdr", "Fake HDR", "  Fake HDR Strength (%)", "w600", "More local contrast and deeper colours", COLOUR).top(150),
    effect("levels", "Levels", "  Levels Strength (%)", "y603", "Deeper blacks and brighter whites", COLOUR).top(150),
    switch("lift_gamma_gain", "Lift Gamma Gain", "z603", "Colour grading of the shadows, mid-tones and highlights", COLOUR),
    effect("dpx", "DPX Film Look", "  DPX Strength (%)", "y600", "Cinema film look: a film curve and richer colours", FILM),
    effect("technicolor", "Technicolor", "  Technicolor Strength (%)", "w603", "Old colour film: paler, cooler colours", FILM).top(250),
    effect("curves", "Contrast Curves", "  Curves Strength (%)", "z600", "More contrast, colours kept", COLOUR).top(150),
    effect("vibrance", "Vibrance", "  Vibrance Strength (%)", "x601", "Livens up dull colours more than strong ones", COLOUR).top(200),
    effect("gaussian_blur", "Gaussian Blur", "  Blur Strength (%)", "z601", "Softens the whole picture, the HUD too", LENS).top(200),
    effect("chromatic_aberration", "Chromatic Aberration", "  Aberration Strength (%)", "x600", "Red and blue fringes towards the picture's borders, as a simple lens makes them", LENS).top(200),
    effect("vignette", "Vignette", "  Vignette Strength (%)", "x603", "Darkens the corners of the picture", LENS),
    effect("film_grain", "Film Grain", "  Grain Strength (%)", "y601", "Film grain, strongest in the mid-tones", FILM),
    effect("daltonize", "Daltonize (colour-blind aid)", "  Daltonize Strength (%)", "x607", "For colour-blind players: makes colours you mix up easier to tell apart", ACCESS).top(100),
    switch("white_balance", "White Balance", "z609", "The picture as if lit by warmer or cooler light", COLOUR),
    effect("bleach_bypass", "Bleach Bypass", "  Bleach Bypass Strength (%)", "x610", "Film developed without bleach: harsh contrast, muted colours", FILM).top(150),
    effect("monochrome", "Monochrome", "  Monochrome Strength (%)", "y610", "Black and white", FILM).top(100),
    effect("sepia", "Sepia", "  Sepia Strength (%)", "z610", "Old brown-toned photograph", FILM).top(100),
    effect("lens_flare", "Lens Flare", "  Lens Flare Strength (%)", "x609", "Ghosts and a halo from bright lights, as a camera lens makes them", LENS),
    effect("anamorphic_streaks", "Anamorphic Streaks", "  Streak Strength (%)", "y609", "Long horizontal streaks from bright lights, as a cinema lens makes them", LENS),
    effect("diffraction_spikes", "Diffraction Spikes", "  Spike Strength (%)", "x611", "Star-shaped spikes on bright lights, as a camera's aperture makes them", LENS),
    effect("lens_dirt", "Lens Dirt", "  Lens Dirt Strength (%)", "y611", "Smudges on the lens that glow when bright light passes through them", LENS),
];

/// The settings page's rows of lift/gamma/gain's values and Daltonize's type.
pub const LIFT_LABEL: &str = "  Lift (r, g, b)";
pub const GAMMA_LABEL: &str = "  Gamma (r, g, b)";
pub const GAIN_LABEL: &str = "  Gain (r, g, b)";
pub const COLOUR_BLINDNESS_LABEL: &str = "  Colour Blindness";
/// White Balance's colour temperature row and its range, in kelvin (6500 is neutral daylight).
pub const TEMPERATURE_LABEL: &str = "  Colour Temperature (K)";
pub const TEMPERATURE_RANGE: std::ops::RangeInclusive<u16> = 3000..=12000;

/// The supersampling downscale's switch and its description.
pub const DOWNSCALE_LABEL: &str = "Supersampling Downscale";
pub const DOWNSCALE_ABOUT: &str = "Smoother, steadier edges with supersampling above 1.0";

/// The description shown by a switch of the Post Effects topic, if the label is one.
pub fn about(label: &str) -> Option<&'static str> {
    if label == DOWNSCALE_LABEL {
        return Some(DOWNSCALE_ABOUT);
    }
    EFFECTS.iter().find(|e| e.label == label).map(|e| e.about)
}

/// The range of a lift, gamma or gain value (SweetFX's).
pub const LGG_RANGE: std::ops::RangeInclusive<f32> = 0.0..=2.0;

/// What the shaders get.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PostLook {
    /// Each effect's strength, in the order of [`EFFECTS`].
    pub strengths: [f32; 25],
    /// White Balance's colour temperature, kelvin (`w609`).
    pub temperature: f32,
    /// Lift, gamma, gain (rows 604 to 606).
    pub lgg: [[f32; 3]; 3],
    /// Daltonize's type ([`DALTONIZE_TYPE_VARIABLE`]).
    pub colour_blindness: f32,
    /// The supersampling downscale (`x608`).
    pub downscale: bool,
}

/// The variable `edcas.ini` copies into `y607` (Daltonize's type) at the lens pass. A variable, not an IniParam:
/// 3DMigoto fades IniParams over seconds, and a fade from protanopia (0) to tritanopia (2) showed deuteranopia (1)
/// on the way (code review 2026-11-26), as the ground type's would (`ground_tint`'s `GROUND_TYPE_VARIABLE`).
pub const DALTONIZE_TYPE_VARIABLE: &str = "$\\ShaderFixes\\edcas.ini\\daltonize_type";

/// Every post row goes to the game as a variable (`$post_<row>`) that `edcas.ini` copies into the row before the
/// passes run. A variable, not the IniParam itself: 3DMigoto fades IniParams from the values file over seconds
/// (`params_fade`, right for the ground's tint or a star's colour), and a picture setting must answer at once, or
/// off cannot be compared with on (the owner, 2026-11-26). `edcas.ini` declares the same names.
macro_rules! post_variables {
    ($($row:literal),* $(,)?) => {
        /// The post rows that have a variable.
        pub const POST_ROWS: &[&str] = &[$($row),*];

        /// The variable of a post row.
        fn variable(row: &str) -> &'static str {
            match row {
                $($row => concat!("$\\ShaderFixes\\edcas.ini\\post_", $row),)*
                _ => panic!("the post row {row} has no variable: add it to post_variables! and edcas.ini"),
            }
        }
    };
}

post_variables!(
    "w602", "x602", "y602", "z602", "z611", "w600", "y603", "z603", "y600", "w603", "z600", "x601",
    "z601", "x600", "x603", "y601", "x607", "z609", "x610", "y610", "z610", "x609", "y609", "x611",
    "y611", "x604", "y604", "z604", "x605", "y605", "z605", "x606", "y606", "z606", "w609", "x608"
);

const LGG_ROWS: [[&str; 3]; 3] = [["x604", "y604", "z604"], ["x605", "y605", "z605"], ["x606", "y606", "z606"]];

impl PostLook {
    pub const NONE: PostLook = PostLook { strengths: [0.0; 25], temperature: 5000.0, lgg: [[1.0; 3]; 3], colour_blindness: 0.0, downscale: false };

    /// What the settings ask for: an effect that is off is 0; one without a strength is 1 when on.
    pub fn of(g: &GameGraphicsSettings) -> Self {
        let p = &g.post;
        let list = p.list();
        let mut strengths = [0.0; 25];
        for (s, e) in strengths.iter_mut().zip(&EFFECTS) {
            let setting = list.iter().find(|(name, _)| *name == e.name).map(|(_, x)| *x).unwrap_or_default();
            *s = match (setting.on, e.strength_label) {
                (false, _) => 0.0,
                (true, None) => 1.0,
                (true, Some(_)) => f32::from(setting.strength.min(e.max)) / 100.0,
            };
        }
        let clamp = |v: [f32; 3]| v.map(|x| x.clamp(*LGG_RANGE.start(), *LGG_RANGE.end()));
        PostLook {
            strengths,
            lgg: [clamp(p.lift), clamp(p.gamma), clamp(p.gain)],
            temperature: f32::from(p.colour_temperature.clamp(*TEMPERATURE_RANGE.start(), *TEMPERATURE_RANGE.end())),
            colour_blindness: f32::from(p.colour_blindness.index()),
            downscale: p.supersampling_downscale,
        }
    }

    /// The values file's lines: every row always, so an effect switched off is written as 0.
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        let mut ours: Vec<_> = EFFECTS.iter().zip(&self.strengths).map(|(e, s)| (variable(e.row), format!("{s:.3}"))).collect();
        for (names, values) in LGG_ROWS.iter().zip(&self.lgg) {
            ours.extend(names.iter().zip(values).map(|(n, v)| (variable(n), format!("{v:.3}"))));
        }
        ours.push((variable("w609"), format!("{:.0}", self.temperature)));
        ours.push((DALTONIZE_TYPE_VARIABLE, format!("{:.0}", self.colour_blindness)));
        ours.push((variable("x608"), if self.downscale { "1" } else { "0" }.to_string()));
        ours
    }
}

/// Three numbers as the settings page shows them.
pub fn rgb_text(v: [f32; 3]) -> String {
    format!("{:.2}, {:.2}, {:.2}", v[0], v[1], v[2])
}

/// Three numbers in [`LGG_RANGE`], separated by commas or spaces; one number stands for all three.
pub fn parse_rgb(text: &str) -> Option<[f32; 3]> {
    let parts: Vec<f32> = text.split(|c: char| c == ',' || c.is_whitespace()).filter(|p| !p.is_empty()).map(str::parse).collect::<Result<_, _>>().ok()?;
    let v = match parts.as_slice() {
        [a] => [*a; 3],
        [a, b, c] => [*a, *b, *c],
        _ => return None,
    };
    v.iter().all(|x| LGG_RANGE.contains(x)).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::game_graphics::ColourBlindness;

    #[test]
    fn every_effect_is_off_by_default() {
        let g = GameGraphicsSettings::default();
        assert_eq!(PostLook::of(&g), PostLook::NONE);
        let rows = PostLook::NONE.assignments();
        assert!(EFFECTS.iter().all(|e| rows.contains(&(variable(e.row), "0.000".to_string()))), "{rows:?}");
        assert!(rows.contains(&(variable("x608"), "0".to_string())));
    }

    #[test]
    fn an_effect_switched_on_writes_its_strength_to_its_own_row() {
        let mut g = GameGraphicsSettings::default();
        let grain = g.post.get_mut("film_grain").unwrap();
        grain.on = true;
        grain.strength = 150;
        g.post.get_mut("dpx").unwrap().strength = 250; // still off
        g.post.get_mut("smaa").unwrap().on = true;
        g.post.colour_blindness = ColourBlindness::Tritanopia;
        let rows = PostLook::of(&g).assignments();
        assert!(rows.contains(&(variable("y601"), "1.500".to_string())), "{rows:?}");
        assert!(rows.contains(&(variable("y600"), "0.000".to_string())), "{rows:?}");
        assert!(rows.contains(&(variable("x602"), "1.000".to_string())), "SMAA has no strength: on is 1 {rows:?}");
        assert!(rows.contains(&(DALTONIZE_TYPE_VARIABLE, "2".to_string())), "a variable, unfaded: {rows:?}");
        assert!(!rows.iter().any(|(r, _)| *r == "y607"), "the ini sets y607 from the variable: {rows:?}");
    }

    #[test]
    fn no_value_goes_as_a_faded_ini_param_and_the_ini_copies_each_variable_into_its_row() {
        let ini = include_str!("../shader_mods/edcas.ini");
        let rows = PostLook::NONE.assignments();
        assert!(rows.iter().all(|(name, _)| name.starts_with('$')), "an IniParam would fade over seconds: {rows:?}");
        assert_eq!(rows.len(), POST_ROWS.len() + 1, "every post row and Daltonize's type");
        for row in POST_ROWS {
            assert_eq!(variable(row), format!("$\\ShaderFixes\\edcas.ini\\post_{row}"));
            assert!(rows.iter().any(|(name, _)| *name == variable(row)), "{row} is written");
            assert!(ini.contains(&format!("\nglobal $post_{row} = 0\n")), "edcas.ini declares $post_{row}");
            assert!(ini.contains(&format!("\npre {row} = $post_{row}\n")), "edcas.ini copies $post_{row} into {row}");
        }
    }

    #[test]
    fn a_strength_stops_at_the_effects_own_top() {
        let mut g = GameGraphicsSettings::default();
        for name in ["monochrome", "levels", "vibrance", "vignette"] {
            let e = g.post.get_mut(name).unwrap();
            e.on = true;
            e.strength = 300;
        }
        let rows = PostLook::of(&g).assignments();
        assert!(rows.contains(&(variable("y610"), "1.000".to_string())), "monochrome is done at 100 %: {rows:?}");
        assert!(rows.contains(&(variable("y603"), "1.500".to_string())), "levels clips above 150 %: {rows:?}");
        assert!(rows.contains(&(variable("x601"), "2.000".to_string())), "{rows:?}");
        assert!(rows.contains(&(variable("x603"), "3.000".to_string())), "the vignette keeps the full range: {rows:?}");
        assert!(EFFECTS.iter().all(|e| (100..=MAX_LOOK_PERCENT).contains(&e.max)), "100 % is always in reach");
    }

    #[test]
    fn the_list_and_the_effects_name_the_same_effects_in_the_same_order() {
        let g = GameGraphicsSettings::default();
        let names: Vec<_> = g.post.list().iter().map(|(n, _)| *n).collect();
        assert_eq!(names, EFFECTS.iter().map(|e| e.name).collect::<Vec<_>>());
        let mut rows: Vec<_> = PostLook::NONE.assignments().into_iter().map(|(r, _)| r).collect();
        let n = rows.len();
        rows.sort_unstable();
        rows.dedup();
        assert_eq!(rows.len(), n, "each value its own row");
    }

    #[test]
    fn the_descriptions_are_short_and_name_no_other_tool() {
        for about in EFFECTS.iter().map(|e| e.about).chain([DOWNSCALE_ABOUT]) {
            assert!(about.len() <= 80, "one short line: {about}");
            assert!(!about.to_ascii_lowercase().contains("reshade"), "{about}");
        }
        assert_eq!(about("Vignette"), Some("Darkens the corners of the picture"));
        assert_eq!(about(DOWNSCALE_LABEL), Some(DOWNSCALE_ABOUT));
        assert_eq!(about("Planet Clouds"), None);
    }

    #[test]
    fn lift_gamma_gain_values_read_as_one_or_three_numbers_in_range() {
        assert_eq!(parse_rgb("1.1, 0.9,1"), Some([1.1, 0.9, 1.0]));
        assert_eq!(parse_rgb("0.8"), Some([0.8; 3]));
        assert_eq!(parse_rgb("1 2"), None);
        assert_eq!(parse_rgb("2.5"), None);
        assert_eq!(rgb_text([1.0, 0.5, 2.0]), "1.00, 0.50, 2.00");
    }
}
