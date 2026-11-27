use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The game's `GraphicsConfigurationOverride.xml`, as EDCAS manages it: whether it may
/// write the file at all, where the game keeps it, and the overrides the player opted in
/// to (by option id, see [`crate::game_graphics::OPTIONS`]).
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct GameGraphicsSettings {
    /// EDCAS writes the opted-in overrides into the game's file. Off: it leaves the file
    /// alone.
    #[serde(default)]
    pub manage: bool,
    /// The game's `Options/Graphics` folder; empty to find it next to the journal folder.
    #[serde(default)]
    pub folder: String,
    /// The overrides the player touched: opted in or out, and the value they set.
    #[serde(default)]
    pub options: BTreeMap<String, OverrideChoice>,
    /// Tint the ground of the planet the commander is at after its makeup, through
    /// 3DMigoto (see [`crate::ground_tint`]).
    #[serde(default)]
    pub ground_tint: bool,
    /// How far the tint goes, in percent.
    #[serde(default = "default_tint_strength")]
    pub ground_tint_strength: u8,
    /// After each planet's makeup, or one colour for every planet.
    #[serde(default)]
    pub ground_tint_mode: TintMode,
    /// The colour of [`TintMode::Fixed`]: a name, `#rrggbb` or `r,g,b`.
    #[serde(default = "default_tint_colour")]
    pub ground_tint_colour: String,
    /// Ground detail under foot, after what the ground is made of (rock, ice, lava, sand ...):
    /// craters, ripples, cracks and stones, through 3DMigoto (see [`crate::ground_tint`]).
    #[serde(default = "default_true")]
    pub ground_detail: bool,
    /// How strong the ground detail's colour and relief are, in percent.
    #[serde(default = "default_full_percent")]
    pub ground_detail_strength: u16,
    /// Clouds in the sky of the planet the commander is at, worked out from its atmosphere,
    /// through 3DMigoto (see [`crate::planet_clouds`]).
    #[serde(default)]
    pub planet_clouds: bool,
    /// How visible the clouds are, in percent.
    #[serde(default = "default_tint_strength")]
    pub planet_clouds_strength: u8,
    /// Draw the clouds as a thick ray-marched slab (costs frames) instead of a thin shell.
    #[serde(default = "default_true")]
    pub planet_clouds_volumetric: bool,
    /// The clouds' shadow on the ground under them (only with the volumetric clouds; costs frames:
    /// a pass over the whole picture before the game lights it, see `edcas_cloud_shade.hlsl`).
    #[serde(default = "default_true")]
    pub planet_cloud_shadows: bool,
    /// Ground fog on planets where the air near the ground gets cold enough overnight to fog
    /// up (the same atmosphere forecast as the clouds, see [`crate::planet_clouds`]).
    #[serde(default = "default_true")]
    pub planet_fog: bool,
    /// How dense the ground fog is, as a percent of what the forecast works out for the planet.
    #[serde(default = "default_full_percent")]
    pub planet_fog_density: u16,
    /// How deep the ground fog is, in metres.
    #[serde(default = "default_fog_depth")]
    pub planet_fog_depth: u16,
    /// Wind-blown dust on planets with air: sand streaming along the ground, billows in gusts
    /// (see [`crate::planet_dust`]).
    #[serde(default = "default_true")]
    pub planet_dust: bool,
    /// How thick the dust is, as a percent of what the planet's air works out to.
    #[serde(default = "default_full_percent")]
    pub planet_dust_strength: u16,
    /// How far away the dust drifting over the land is drawn, in kilometres.
    #[serde(default = "default_dust_reach")]
    pub planet_dust_reach_km: u16,
    /// How thick the cloud slab is, as a percent of the forecast's thickness (only drawn while
    /// the clouds are volumetric; 0 is the thin shell).
    #[serde(default = "default_full_percent")]
    pub planet_cloud_thickness: u16,
    /// How soft the clouds' edge is, as a percent of the shader's usual softness.
    #[serde(default = "default_full_percent")]
    pub planet_cloud_softness: u16,
    /// Frost on the cockpit canopy while the ship is in cloud or fog.
    #[serde(default = "default_true")]
    pub planet_canopy_frost: bool,
    /// How strong the canopy frost is, as a percent of the shader's usual strength.
    #[serde(default = "default_full_percent")]
    pub planet_canopy_frost_strength: u16,
    /// Light everything in the colour of the system's star (Odyssey's stars all cast white
    /// light), through 3DMigoto's copies of the game's sun and sky passes and EDCAS's own
    /// clouds, fog and dust (see [`crate::star_light`]).
    #[serde(default = "default_true")]
    pub star_light: bool,
    /// How far towards the star's colour, in percent, when the player chose a value of their own:
    /// 100 is the blackbody colour of its temperature, above pushes the hue further. `None` is
    /// EDCAS's default ([`STAR_LIGHT_STRENGTH_DEFAULT`]): a player on it follows it when a
    /// release changes it. Read it with [`Self::star_light_strength`], set it with
    /// [`Self::set_star_light_strength`].
    #[serde(default)]
    pub star_light_percent: Option<u16>,
    /// What a settings file from before 0.43.121 holds under the old key, a plain number that
    /// could not tell "the default" from "my choice"; [`Self::adopt_defaults`] moves it over
    /// after loading, and it is never written again.
    #[serde(default, rename = "star_light_strength", skip_serializing)]
    pub star_light_strength_before_default: Option<u16>,
    /// In witch space (the hyperspace jump) the light cycles through the rainbow instead of
    /// keeping the last star's colour.
    #[serde(default = "default_true")]
    pub star_light_rainbow: bool,
    /// The fuel bar's switch of the releases up to 0.43.244, when the spectrum was its one look:
    /// read from an older settings file and never written again. [`Self::adopt_defaults`] moves
    /// it over to the fuel bar's look in [`Self::hud_gauges`].
    #[serde(default, rename = "fuel_bar_spectrum", skip_serializing)]
    pub fuel_bar_spectrum_before_gauge: bool,
    /// The colour of a ship's HUD turned and its brightness changed while the game runs (see
    /// [`crate::shader_mods::hud`]): the panels, the head-up lines and the radar (its disc, rings and
    /// contacts, a hostile's red kept); the holograms keep Elite's colours. Off unless the player
    /// asks: it replaces the game's HUD shaders, the ones of the parts it is on for.
    #[serde(default)]
    pub hud_colour: bool,
    /// How each part of the HUD is coloured under it: left Elite's, turned, or one colour.
    #[serde(default)]
    pub hud_parts: HudParts,
    /// How far the HUD's hue is turned, in degrees: orange to blue is 180.
    #[serde(default = "default_hud_hue_shift")]
    pub hud_hue_shift: u16,
    /// The HUD's brightness, in percent of Elite's.
    #[serde(default = "default_full_percent")]
    pub hud_brightness: u16,
    /// The gauges of the lower right panel in colours of their own (see
    /// [`crate::shader_mods::hud`]). All Elite's unless the player asks: a look of their own
    /// replaces the game's HUD panel shader.
    #[serde(default)]
    pub hud_gauges: HudGauges,
    /// The ship's hologram, right of the radar, in a colour of its own (see
    /// [`crate::shader_mods::hologram`]). Off unless the player asks: it replaces the game's
    /// shader of that hologram. The target's has a switch of its own.
    #[serde(default)]
    pub hud_hologram: bool,
    /// That colour: a name, `#rrggbb` or `r,g,b`.
    #[serde(default = "default_hud_hologram_colour")]
    pub hud_hologram_colour: String,
    /// The target's hologram, left of the radar, in a colour of its own, as the ship's.
    #[serde(default)]
    pub hud_target_hologram: bool,
    /// That colour: a name, `#rrggbb` or `r,g,b`.
    #[serde(default = "default_hud_hologram_colour")]
    pub hud_target_hologram_colour: String,
    /// How strongly the game's lamps (ship's lights, suit torch, settlement lamps) light the fog,
    /// as a percent of the physical glow. Elite's night exposure is far brighter than physical, so
    /// the physical 100 % turned the fog in front of the ship's lights white; 10 % looked right.
    #[serde(default = "default_lamp_glow")]
    pub planet_lamp_glow: u16,
    /// Picture effects (fake HDR, film looks, blur, chromatic aberration, grain),
    /// on Elite's tonemapped picture (see [`crate::post_effects`]). All off unless the player asks.
    #[serde(default)]
    pub post: PostEffects,
    /// The game folder holding 3DMigoto; empty to find it from the journal folder.
    #[serde(default)]
    pub game_folder: String,
}

/// Where the ground tint's colour comes from.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TintMode {
    /// From the makeup of the planet the commander is at; EDCAS has to run.
    #[default]
    PerPlanet,
    /// One colour the player picked, on every planet; stays when EDCAS closes.
    Fixed,
}

/// How a part of the ship's HUD is coloured while HUD Colour is on.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PartLook {
    /// As Elite draws it.
    Elite,
    /// Its colours turned by the hue shift, so what Elite draws in two colours stays two.
    #[default]
    Turned,
    /// One colour the player picked, each pixel as bright as Elite drew it.
    Colour,
}

/// Which way a part's colour fades into its second one across the picture.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FadeWay {
    /// From the picture's left edge to its right one.
    #[default]
    Across,
    /// From the picture's bottom to its top.
    Up,
}

/// One part's look and the colour of [`PartLook::Colour`]: a name, `#rrggbb` or `r,g,b`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct PartSetting {
    pub look: PartLook,
    pub colour: String,
    /// The colour that one fades into across the picture, written as it is; empty for one
    /// colour alone.
    pub fade_to: String,
    pub fade_way: FadeWay,
}

impl Default for PartSetting {
    fn default() -> Self {
        PartSetting { look: PartLook::Turned, colour: "cyan".to_string(), fade_to: String::new(), fade_way: FadeWay::Across }
    }
}

impl PartSetting {
    /// Whether the part's colour fades into a second one: the look is one colour and a second
    /// colour is set.
    pub fn fades(&self) -> bool {
        self.look == PartLook::Colour && crate::theme::parse_rgb(&self.fade_to).is_some()
    }

    /// The colour as the settings show it: the one colour, or `first > second` for a fade.
    pub fn colour_text(&self) -> String {
        if self.fade_to.trim().is_empty() { self.colour.clone() } else { format!("{} > {}", self.colour, self.fade_to) }
    }
}

/// The parts of a ship's HUD that take the HUD's colour, each with a look of its own: the
/// panels (the gauges and the side panels), the head-up lines, the radar (its disc and rings).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct HudParts {
    pub panels: PartSetting,
    pub lines: PartSetting,
    pub radar: PartSetting,
}

impl HudParts {
    /// The parts in the order of their rows, each with its name here.
    pub fn list(&self) -> [(&'static str, &PartSetting); 3] {
        [("panels", &self.panels), ("lines", &self.lines), ("radar", &self.radar)]
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut PartSetting> {
        match name {
            "panels" => Some(&mut self.panels),
            "lines" => Some(&mut self.lines),
            "radar" => Some(&mut self.radar),
            _ => None,
        }
    }
}

/// How a gauge of the ship's HUD is coloured.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GaugeLook {
    /// As Elite draws it.
    #[default]
    Elite,
    /// The colours of the spectrum along it, red at one end to violet at the other.
    Spectrum,
    /// One colour the player picked.
    Colour,
}

/// One gauge's look and the colour of [`GaugeLook::Colour`]: a name, `#rrggbb` or `r,g,b`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct GaugeSetting {
    pub look: GaugeLook,
    pub colour: String,
}

impl Default for GaugeSetting {
    fn default() -> Self {
        GaugeSetting { look: GaugeLook::Elite, colour: "white".to_string() }
    }
}

/// The gauges of a ship's lower right panel that take colours of their own, in the order of
/// [`crate::shader_mods::GAUGES`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct HudGauges {
    pub fuel: GaugeSetting,
    pub heat: GaugeSetting,
    pub speed: GaugeSetting,
    pub shields: GaugeSetting,
    pub hull: GaugeSetting,
    pub distributor: GaugeSetting,
}

impl Default for HudGauges {
    fn default() -> Self {
        let of = |colour: &str| GaugeSetting { look: GaugeLook::Elite, colour: colour.to_string() };
        HudGauges { fuel: of("cyan"), heat: of("red"), speed: of("green"), shields: of("blue"), hull: of("white"), distributor: of("yellow") }
    }
}

impl HudGauges {
    /// Each gauge by its name in [`crate::shader_mods::GAUGES`].
    pub fn list(&self) -> [(&'static str, &GaugeSetting); 6] {
        [("fuel", &self.fuel), ("heat", &self.heat), ("speed", &self.speed), ("shields", &self.shields), ("hull", &self.hull), ("distributor", &self.distributor)]
    }

    /// The gauge of that name, to change it.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut GaugeSetting> {
        match name {
            "fuel" => Some(&mut self.fuel),
            "heat" => Some(&mut self.heat),
            "speed" => Some(&mut self.speed),
            "shields" => Some(&mut self.shields),
            "hull" => Some(&mut self.hull),
            "distributor" => Some(&mut self.distributor),
            _ => None,
        }
    }

    /// Whether a gauge is not as Elite draws it.
    pub fn any(&self) -> bool {
        self.list().iter().any(|(_, gauge)| gauge.look != GaugeLook::Elite)
    }
}

/// One post effect: whether it is on and how strong, in percent of its usual strength.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct PostEffect {
    pub on: bool,
    pub strength: u16,
}

impl Default for PostEffect {
    fn default() -> Self {
        PostEffect { on: false, strength: 100 }
    }
}

/// Which colour blindness Daltonize corrects for.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ColourBlindness {
    /// Red-blind.
    #[default]
    Protanopia,
    /// Green-blind.
    Deuteranopia,
    /// Blue-blind.
    Tritanopia,
}

impl ColourBlindness {
    pub const ALL: [ColourBlindness; 3] = [ColourBlindness::Protanopia, ColourBlindness::Deuteranopia, ColourBlindness::Tritanopia];

    pub fn name(self) -> &'static str {
        match self {
            ColourBlindness::Protanopia => "Protanopia (red)",
            ColourBlindness::Deuteranopia => "Deuteranopia (green)",
            ColourBlindness::Tritanopia => "Tritanopia (blue)",
        }
    }

    /// The shader's number for it (`y607`).
    pub fn index(self) -> u8 {
        match self {
            ColourBlindness::Protanopia => 0,
            ColourBlindness::Deuteranopia => 1,
            ColourBlindness::Tritanopia => 2,
        }
    }
}

/// The post effects, in the order the shaders apply them (see [`crate::post_effects`]).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct PostEffects {
    pub deband: PostEffect,
    pub smaa: PostEffect,
    pub cas: PostEffect,
    pub luma_sharpen: PostEffect,
    pub clarity: PostEffect,
    pub fake_hdr: PostEffect,
    pub levels: PostEffect,
    pub lift_gamma_gain: PostEffect,
    pub dpx: PostEffect,
    pub technicolor: PostEffect,
    pub curves: PostEffect,
    pub vibrance: PostEffect,
    pub gaussian_blur: PostEffect,
    pub chromatic_aberration: PostEffect,
    pub vignette: PostEffect,
    pub film_grain: PostEffect,
    pub daltonize: PostEffect,
    pub white_balance: PostEffect,
    pub bleach_bypass: PostEffect,
    pub monochrome: PostEffect,
    pub sepia: PostEffect,
    pub lens_flare: PostEffect,
    pub anamorphic_streaks: PostEffect,
    pub diffraction_spikes: PostEffect,
    pub lens_dirt: PostEffect,
    /// White Balance's colour temperature, in kelvin: the picture as if lit by a light of this colour (6500 neutral).
    pub colour_temperature: u16,
    /// Lift, gamma and gain per channel (red, green, blue), 1 neutral, 0 to 2.
    pub lift: [f32; 3],
    pub gamma: [f32; 3],
    pub gain: [f32; 3],
    pub colour_blindness: ColourBlindness,
    /// The supersampled picture brought down to the screen by a filter over each pixel's whole
    /// footprint instead of Elite's one bilinear read (see `edcas_downscale.hlsl`).
    pub supersampling_downscale: bool,
}

impl Default for PostEffects {
    fn default() -> Self {
        let off = PostEffect::default();
        PostEffects {
            deband: off,
            smaa: off,
            cas: off,
            luma_sharpen: off,
            clarity: off,
            fake_hdr: off,
            levels: off,
            lift_gamma_gain: off,
            dpx: off,
            technicolor: off,
            curves: off,
            vibrance: off,
            gaussian_blur: off,
            chromatic_aberration: off,
            vignette: off,
            film_grain: off,
            daltonize: off,
            white_balance: off,
            bleach_bypass: off,
            monochrome: off,
            sepia: off,
            lens_flare: off,
            anamorphic_streaks: off,
            diffraction_spikes: off,
            lens_dirt: off,
            colour_temperature: 5000,
            lift: [1.0; 3],
            gamma: [1.0; 3],
            gain: [1.0; 3],
            colour_blindness: ColourBlindness::default(),
            supersampling_downscale: false,
        }
    }
}

impl PostEffects {
    /// Each effect by its name in [`crate::post_effects::EFFECTS`], in that order.
    pub fn list(&self) -> [(&'static str, PostEffect); 25] {
        [
            ("deband", self.deband),
            ("smaa", self.smaa),
            ("cas", self.cas),
            ("luma_sharpen", self.luma_sharpen),
            ("clarity", self.clarity),
            ("fake_hdr", self.fake_hdr),
            ("levels", self.levels),
            ("lift_gamma_gain", self.lift_gamma_gain),
            ("dpx", self.dpx),
            ("technicolor", self.technicolor),
            ("curves", self.curves),
            ("vibrance", self.vibrance),
            ("gaussian_blur", self.gaussian_blur),
            ("chromatic_aberration", self.chromatic_aberration),
            ("vignette", self.vignette),
            ("film_grain", self.film_grain),
            ("daltonize", self.daltonize),
            ("white_balance", self.white_balance),
            ("bleach_bypass", self.bleach_bypass),
            ("monochrome", self.monochrome),
            ("sepia", self.sepia),
            ("lens_flare", self.lens_flare),
            ("anamorphic_streaks", self.anamorphic_streaks),
            ("diffraction_spikes", self.diffraction_spikes),
            ("lens_dirt", self.lens_dirt),
        ]
    }

    /// The effect of that name, to change it.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut PostEffect> {
        match name {
            "deband" => Some(&mut self.deband),
            "smaa" => Some(&mut self.smaa),
            "cas" => Some(&mut self.cas),
            "luma_sharpen" => Some(&mut self.luma_sharpen),
            "clarity" => Some(&mut self.clarity),
            "fake_hdr" => Some(&mut self.fake_hdr),
            "levels" => Some(&mut self.levels),
            "lift_gamma_gain" => Some(&mut self.lift_gamma_gain),
            "dpx" => Some(&mut self.dpx),
            "technicolor" => Some(&mut self.technicolor),
            "curves" => Some(&mut self.curves),
            "vibrance" => Some(&mut self.vibrance),
            "gaussian_blur" => Some(&mut self.gaussian_blur),
            "chromatic_aberration" => Some(&mut self.chromatic_aberration),
            "vignette" => Some(&mut self.vignette),
            "film_grain" => Some(&mut self.film_grain),
            "daltonize" => Some(&mut self.daltonize),
            "white_balance" => Some(&mut self.white_balance),
            "bleach_bypass" => Some(&mut self.bleach_bypass),
            "monochrome" => Some(&mut self.monochrome),
            "sepia" => Some(&mut self.sepia),
            "lens_flare" => Some(&mut self.lens_flare),
            "anamorphic_streaks" => Some(&mut self.anamorphic_streaks),
            "diffraction_spikes" => Some(&mut self.diffraction_spikes),
            "lens_dirt" => Some(&mut self.lens_dirt),
            _ => None,
        }
    }
}

fn default_tint_strength() -> u8 {
    70
}

/// The HUD's hue turn a player starts from: orange to blue.
fn default_hud_hue_shift() -> u16 {
    180
}

/// The colour a hologram starts from when its switch goes on.
fn default_hud_hologram_colour() -> String {
    "cyan".to_string()
}

/// The range of the HUD's brightness setting, in percent of Elite's.
pub const HUD_BRIGHTNESS_RANGE: std::ops::RangeInclusive<u16> = 25..=300;

/// The most the fog density, the cloud thickness, the cloud softness, the canopy frost, the lamp glow and the ground detail settings take, in percent.
pub const MAX_LOOK_PERCENT: u16 = 300;
/// The range of the fog depth setting, in metres.
pub const FOG_DEPTH_RANGE: std::ops::RangeInclusive<u16> = 100..=3000;
/// The range of the dust reach setting, in kilometres.
pub const DUST_REACH_RANGE: std::ops::RangeInclusive<u16> = 1..=30;

fn default_full_percent() -> u16 {
    100
}

/// EDCAS's default for the star light's strength, in percent. 100 is the star's blackbody colour
/// as a camera balanced for daylight records it; an eye there adapts to the light and sees less
/// of it, so the default is below (2026-11-11: 100 looked a bit strong in most systems).
pub const STAR_LIGHT_STRENGTH_DEFAULT: u16 = 75;
/// The default up to 0.43.120, when the file held a plain number: a file with it was on the default.
const STAR_LIGHT_STRENGTH_DEFAULT_BEFORE: u16 = 100;

impl GameGraphicsSettings {
    /// How far the light goes towards the star's colour, in percent: the player's value, else
    /// EDCAS's default.
    pub fn star_light_strength(&self) -> u16 {
        self.star_light_percent.unwrap_or(STAR_LIGHT_STRENGTH_DEFAULT)
    }

    /// Sets the star light's strength. The default's own value is "on the default": whoever is
    /// at it moves with it when a release changes it.
    pub fn set_star_light_strength(&mut self, percent: u16) {
        self.star_light_percent = (percent != STAR_LIGHT_STRENGTH_DEFAULT).then_some(percent);
    }

    /// After loading a settings file: a value from before the default was its own state moves
    /// over, the old default (100) as "on the default", so it becomes today's; any other as the
    /// player's choice. A file that already holds the new key keeps it. And the fuel bar's old
    /// switch becomes the fuel bar's look: on is the spectrum, unless the file gives it a look.
    pub fn adopt_defaults(&mut self) {
        match self.star_light_strength_before_default.take() {
            Some(old) if self.star_light_percent.is_none() && old != STAR_LIGHT_STRENGTH_DEFAULT_BEFORE => self.set_star_light_strength(old),
            _ => {}
        }
        if std::mem::take(&mut self.fuel_bar_spectrum_before_gauge) && self.hud_gauges.fuel.look == GaugeLook::Elite {
            self.hud_gauges.fuel.look = GaugeLook::Spectrum;
        }
    }
}

fn default_lamp_glow() -> u16 {
    10
}

fn default_fog_depth() -> u16 {
    800
}

fn default_dust_reach() -> u16 {
    15
}

fn default_true() -> bool {
    true
}

fn default_tint_colour() -> String {
    "#a8c8ff".to_string()
}

impl Default for GameGraphicsSettings {
    fn default() -> Self {
        Self {
            manage: false,
            folder: String::new(),
            options: BTreeMap::new(),
            ground_tint: false,
            ground_tint_strength: default_tint_strength(),
            ground_tint_mode: TintMode::default(),
            ground_tint_colour: default_tint_colour(),
            ground_detail: true,
            ground_detail_strength: default_full_percent(),
            planet_clouds: false,
            planet_clouds_strength: default_tint_strength(),
            planet_clouds_volumetric: true,
            planet_cloud_shadows: true,
            planet_fog: true,
            planet_fog_density: default_full_percent(),
            planet_fog_depth: default_fog_depth(),
            planet_dust: true,
            planet_dust_strength: default_full_percent(),
            planet_dust_reach_km: default_dust_reach(),
            planet_cloud_thickness: default_full_percent(),
            planet_cloud_softness: default_full_percent(),
            planet_canopy_frost: true,
            planet_canopy_frost_strength: default_full_percent(),
            star_light: true,
            star_light_percent: None,
            star_light_strength_before_default: None,
            star_light_rainbow: true,
            fuel_bar_spectrum_before_gauge: false,
            hud_colour: false,
            hud_parts: HudParts::default(),
            hud_hue_shift: default_hud_hue_shift(),
            hud_brightness: default_full_percent(),
            hud_gauges: HudGauges::default(),
            hud_hologram: false,
            hud_hologram_colour: default_hud_hologram_colour(),
            hud_target_hologram: false,
            hud_target_hologram_colour: default_hud_hologram_colour(),
            planet_lamp_glow: default_lamp_glow(),
            post: PostEffects::default(),
            game_folder: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct OverrideChoice {
    pub enabled: bool,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_settings_file_from_before_the_landing_dust_went_still_loads() {
        // `planet_landing_dust` (removed 2026-11-03) may still be in the player's settings.json.
        let mut json = serde_json::to_value(GameGraphicsSettings::default()).unwrap();
        json["planet_landing_dust"] = serde_json::Value::Bool(false);
        let loaded: GameGraphicsSettings = serde_json::from_value(json).unwrap();
        assert!(loaded == GameGraphicsSettings::default());
    }

    /// A settings file as a release up to 0.43.120 wrote it, with the star light at `percent`, loaded.
    fn loaded_from_before(percent: u16) -> GameGraphicsSettings {
        let mut json = serde_json::to_value(GameGraphicsSettings::default()).unwrap();
        json.as_object_mut().unwrap().remove("star_light_percent");
        json["star_light_strength"] = percent.into();
        let mut g: GameGraphicsSettings = serde_json::from_value(json).unwrap();
        g.adopt_defaults();
        g
    }

    #[test]
    fn whoever_was_on_the_old_star_light_default_moves_to_the_new_one() {
        // The old default, 100: now on the default, whatever it is.
        let g = loaded_from_before(100);
        assert_eq!((g.star_light_percent, g.star_light_strength()), (None, STAR_LIGHT_STRENGTH_DEFAULT));
        assert!(g == GameGraphicsSettings::default());
        // A value of the player's own stays; one that happens to be today's default is the default.
        assert_eq!(loaded_from_before(60).star_light_percent, Some(60));
        assert_eq!(loaded_from_before(0).star_light_strength(), 0);
        assert_eq!(loaded_from_before(STAR_LIGHT_STRENGTH_DEFAULT).star_light_percent, None);
        // The old key is not written again, so a 100 chosen from now on is the player's and stays.
        let mut g = loaded_from_before(100);
        g.set_star_light_strength(100);
        let json = serde_json::to_value(&g).unwrap();
        assert!(json.get("star_light_strength").is_none(), "{json}");
        assert_eq!(json["star_light_percent"], 100);
        let mut again: GameGraphicsSettings = serde_json::from_value(json).unwrap();
        again.adopt_defaults();
        assert_eq!(again.star_light_strength(), 100);
        // Back at the default's value: on the default again (null in the file).
        g.set_star_light_strength(STAR_LIGHT_STRENGTH_DEFAULT);
        assert!(serde_json::to_value(&g).unwrap()["star_light_percent"].is_null());
        // A file with neither key (older than the star light): the default.
        let mut json = serde_json::to_value(GameGraphicsSettings::default()).unwrap();
        json.as_object_mut().unwrap().remove("star_light_percent");
        let mut g: GameGraphicsSettings = serde_json::from_value(json).unwrap();
        g.adopt_defaults();
        assert_eq!(g.star_light_strength(), STAR_LIGHT_STRENGTH_DEFAULT);
    }

    #[test]
    fn the_old_spectrum_fuel_bar_switch_becomes_the_fuel_bars_look() {
        // A settings file of a release up to 0.43.244: the switch alone, no fuel bar among the gauges.
        let before = |on: bool| {
            let mut json = serde_json::to_value(GameGraphicsSettings::default()).unwrap();
            json["hud_gauges"].as_object_mut().unwrap().remove("fuel");
            json["fuel_bar_spectrum"] = on.into();
            let mut g: GameGraphicsSettings = serde_json::from_value(json).unwrap();
            g.adopt_defaults();
            g
        };
        assert_eq!(before(true).hud_gauges.fuel.look, GaugeLook::Spectrum);
        assert!(before(true).hud_gauges.any(), "the panel shader's copy is still asked for");
        assert!(before(false) == GameGraphicsSettings::default());
        // The old key is not written again, so a fuel bar set back to Elite's stays Elite's.
        let mut g = before(true);
        g.hud_gauges.fuel.look = GaugeLook::Elite;
        let json = serde_json::to_value(&g).unwrap();
        assert!(json.get("fuel_bar_spectrum").is_none(), "{json}");
        let mut again: GameGraphicsSettings = serde_json::from_value(json).unwrap();
        again.adopt_defaults();
        assert_eq!(again.hud_gauges.fuel.look, GaugeLook::Elite);
        // A colour picked for it is kept like any gauge's.
        g.hud_gauges.fuel = GaugeSetting { look: GaugeLook::Colour, colour: "#30d8e8".to_string() };
        let again: GameGraphicsSettings = serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
        assert!(again.hud_gauges.fuel == g.hud_gauges.fuel);
    }
}
