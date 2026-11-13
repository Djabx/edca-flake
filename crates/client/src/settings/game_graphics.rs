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
    /// How strongly the game's lamps (ship's lights, suit torch, settlement lamps) light the fog,
    /// as a percent of the physical glow. Elite's night exposure is far brighter than physical, so
    /// the physical 100 % turned the fog in front of the ship's lights white; 10 % looked right.
    #[serde(default = "default_lamp_glow")]
    pub planet_lamp_glow: u16,
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

fn default_tint_strength() -> u8 {
    70
}

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
    /// player's choice. A file that already holds the new key keeps it.
    pub fn adopt_defaults(&mut self) {
        match self.star_light_strength_before_default.take() {
            Some(old) if self.star_light_percent.is_none() && old != STAR_LIGHT_STRENGTH_DEFAULT_BEFORE => self.set_star_light_strength(old),
            _ => {}
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
            planet_lamp_glow: default_lamp_glow(),
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
}
