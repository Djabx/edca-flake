//! Planet clouds: whether the sky of the planet the commander is at has clouds, and what
//! they look like, worked out from its atmosphere.
//!
//! A gas forms ice clouds where the air is colder than the gas's frost point at the
//! pressure it has in the mix. The frost point comes from Clausius–Clapeyron, anchored at
//! the gas's triple point. A thin atmosphere cools with height to about 0.6 of its surface
//! temperature, so the gas that freezes closest to the ground makes the clouds, and how
//! high up the air must be for that tells how high the clouds sit. A frost point above
//! the gas's triple point means the gas condenses to droplets, not ice. A gas that is only
//! a small share of the air makes thinner clouds.
//!
//! The drawing is done by 3DMigoto (`scripts/ed-shader/edcas_clouds.hlsl`); EDCAS writes
//! the values it reads, `x503` to `w504`, `z505`, `x507`, `y507`, `w507`, `x508` and `x509` of `ShaderFixes/edcas_params.txt` (see
//! [`crate::ground_tint`], which owns that file). The shader draws either a thin shell
//! (`z505 = 0`) or a ray-marched slab of the thickness `z505`; the slab costs frames.
//!
//! The same frost line tells whether the planet fogs up at night (`x507`, with the planet's radius
//! in `w507`): fog forms where the air near the ground cools to the gas's frost point overnight, so
//! it needs the frost point close to the surface temperature, and a trace gas makes less.

use crate::journal_reader::BodyScan;

/// Gas constant, J/(mol K).
const R: f64 = 8.314;
/// How cold the top of a thin atmosphere gets, as a share of the surface temperature.
const TOP_OF_AIR: f64 = 0.6;
/// Gases with less than this share (percent) of the air don't make clouds of their own.
const MIN_PERCENT: f32 = 0.01;

// 2026-10-26: tuned in game on a pure CO2 world (Hegoea XL-K c22-8 6 e), where the model
// gave x503 0.630, x504 0.508, w503 1.6 and the player found x503 0.35, x504 0.22, w503 0.8 right (Mars's
// cloud optical depth of 0.1 to 0.5 backs the thinner opacity). Later that day, with the clouds
// visible on the approach as well, x503 0.25 and x504 0.09 looked right (coverage x 0.72, opacity
// x 0.39 on top). The physics above stays as it was; these three map it to how the clouds of a
// thin atmosphere look, on every planet.
/// Share of the modelled coverage that is drawn (x503).
const COVERAGE_SCALE: f64 = 0.40;
/// Share of the modelled opacity that is drawn (x504), after the strength setting. The model's
/// opacity is at least 0.15, so at full strength the drawn one is at least 0.15 * 0.18 = 0.03.
const OPACITY_SCALE: f32 = 0.18;
/// The clouds' brightness (w503).
const BRIGHTNESS: f32 = 0.8;

// 2026-10-26: the same world (Hegoea XL-K c22-8 6 e) with the volumetric slab on: z505 0.1 and
// x504 0.6 looked right, where the thin shell wanted x504 0.09. In the slab x504 is the alpha of a
// vertical path through all of it, so it is the thin opacity times 0.6 / 0.09. Mars and Pluto
// have thin flat cloud layers, and their fog hugs the ground, hence the other thicknesses.
// 2026-10-27, the next night there: x504 0.6 drew nearly opaque white masses, 0.3 veils with the
// stars showing through, as Mars's clouds (optical depth 0.15 to 0.5). So 0.3 / 0.09; the
// thickest air (thin 0.144) comes to 0.48.
/// How many times the thin shell's opacity the slab's x504 is (0.3 / 0.09).
const SLAB_OPACITY_SCALE: f32 = 0.3 / 0.09;
/// The slab's thickness (z505) for clouds in the middle of the air, in fog on the ground and
/// for high clouds.
const THICKNESS: f32 = 0.1;
const FOG_THICKNESS: f32 = 0.2;
const HIGH_THICKNESS: f32 = 0.06;

/// The depth of the fog (`y507`, metres) the shader draws when nothing sets it: what the fog
/// setting starts at.
pub const FOG_DEPTH_M: f32 = 800.0;
/// The softness of the clouds' edge (`x508`) the shader draws when nothing sets it; the softness
/// setting scales it (larger = softer).
pub const EDGE_SOFTNESS: f32 = 0.5;
/// The strength of the canopy frost (`x509`) the shader draws when nothing sets it; the frost
/// setting scales it.
pub const FROST_STRENGTH: f32 = 1.0;

/// The lamps' glow in fog (`w509`) as a share of the physical one, as `edcas.ini` has it.
pub const LAMP_GLOW: f32 = 0.1;

// 2026-10-27: tuned in game on the same world (Hegoea XL-K c22-8 6 e: pure CO2, 804.6 Pa, 156.03 K,
// m 0.127) at clouds strength 72 %, where a ground fog density x507 of 0.1 looked right. The fog
// share is 1 where the gas freezes at the ground (m <= 0) and falls to 0 at FOG_MAX_M, where the
// air has to cool a third of the way up the atmosphere's cold before it frosts: no overnight fog.
/// The frost-line height from which a planet gets no ground fog.
const FOG_MAX_M: f64 = 0.3;
/// The fog density (x507) of a planet whose gas freezes at the ground, with the clouds at full
/// strength and the gas the whole air. 6 e then comes to (1 - 0.127 / 0.3) * 0.72 * 0.24 = 0.10.
const FOG_SCALE: f64 = 0.24;

/// A gas that can freeze into clouds: its triple point and heat of sublimation.
struct Ice {
    /// The game's name for the gas.
    name: &'static str,
    /// What the clouds are called when they are ice, and when they are droplets.
    label: &'static str,
    droplets: &'static str,
    /// Triple point temperature, K.
    t_triple: f64,
    /// Triple point pressure, Pa.
    p_triple: f64,
    /// Heat of sublimation, J/mol.
    latent: f64,
}

const ICES: [Ice; 9] = [
    Ice { name: "CarbonDioxide", label: "carbon dioxide ice", droplets: "carbon dioxide clouds", t_triple: 216.58, p_triple: 518_000.0, latent: 25_200.0 },
    Ice { name: "Water", label: "water ice", droplets: "water clouds", t_triple: 273.16, p_triple: 611.66, latent: 51_000.0 },
    Ice { name: "Ammonia", label: "ammonia ice", droplets: "ammonia clouds", t_triple: 195.4, p_triple: 6_060.0, latent: 28_900.0 },
    Ice { name: "Methane", label: "methane ice", droplets: "methane clouds", t_triple: 90.69, p_triple: 11_696.0, latent: 9_130.0 },
    Ice { name: "Nitrogen", label: "nitrogen ice", droplets: "nitrogen clouds", t_triple: 63.15, p_triple: 12_520.0, latent: 6_290.0 },
    Ice { name: "Oxygen", label: "oxygen ice", droplets: "oxygen clouds", t_triple: 54.36, p_triple: 146.3, latent: 7_260.0 },
    Ice { name: "SulphurDioxide", label: "sulphur dioxide ice", droplets: "sulphur dioxide clouds", t_triple: 197.69, p_triple: 1_670.0, latent: 32_300.0 },
    Ice { name: "Argon", label: "argon ice", droplets: "argon clouds", t_triple: 83.81, p_triple: 68_900.0, latent: 7_610.0 },
    Ice { name: "Neon", label: "neon ice", droplets: "neon clouds", t_triple: 24.56, p_triple: 43_370.0, latent: 2_050.0 },
];

impl Ice {
    /// The temperature below which the gas, at this partial pressure (Pa), freezes. `None`
    /// where the extrapolation breaks down (absurd pressures).
    fn frost_point(&self, partial_pa: f64) -> Option<f64> {
        let inverse = 1.0 / self.t_triple - (R / self.latent) * (partial_pa / self.p_triple).ln();
        (inverse > 0.0).then(|| 1.0 / inverse)
    }
}

/// What 3DMigoto gets. The names are the shader's `IniParams`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clouds {
    /// `x503`: share of the sky covered, 0 = no clouds.
    pub coverage: f32,
    /// `y503`: clouds across the visible sky (larger = smaller clouds).
    pub size: f32,
    /// `z503`: height, 0 = at the ground, 1 = highest.
    pub height: f32,
    /// `w503`
    pub brightness: f32,
    /// `x504` of the thin shell (`z505 = 0`): the largest alpha.
    pub opacity: f32,
    /// `x504` of the slab (`z505 > 0`): the alpha of a vertical path through all of it.
    pub slab_opacity: f32,
    /// `z505`: how thick the slab is (the fraction of the atmosphere), used while `volumetric`.
    pub thickness: f32,
    /// Draw the slab (`z505` = thickness, `x504` = slab opacity) instead of the thin shell
    /// (`z505` = 0, `x504` = opacity). The player's setting; the forecast starts out with it on
    /// and [`crate::ground_tint`] sets it from the settings.
    pub volumetric: bool,
    /// `y504`: 0 = puffy, 1 = cirrus streaks.
    pub streaks: f32,
    /// Twilight glow. No longer in the values file: the shader doesn't read `z504` any more.
    pub glow: f32,
    /// `w504`
    pub iridescence: f32,
    /// `x507`: how dense the ground fog is (1 = about 400 m of sight), 0 = none. The player's
    /// fog setting zeroes it in [`crate::ground_tint`].
    pub fog: f32,
    /// `w507`: the planet's radius in metres, for the fog's metres; 0 where the body is not known
    /// ([`for_body`] fills it in).
    pub radius_m: f32,
    /// `y507`: how deep the fog is, in metres. The shader reads 0 as its own 800, so the player's
    /// setting is always written (the forecast starts at [`FOG_DEPTH_M`]).
    pub fog_depth_m: f32,
    /// `x508`: how soft the clouds' edge is (larger = softer). The shader reads 0 as its own 0.5,
    /// so the setting never writes 0 (the forecast starts at [`EDGE_SOFTNESS`]).
    pub softness: f32,
    /// `x509`: how strong the frost on the cockpit canopy is (`scripts/ed-shader/edcas_canopy_ice.hlsl`
    /// draws it while the ship is in cloud or fog). The shader reads 0 as its own 1, so the
    /// setting never writes 0 (the forecast starts at [`FROST_STRENGTH`]).
    pub frost: f32,
    /// `w509`: how strongly the game's lamps light the fog, a factor of the physical glow
    /// (`scripts/ed-shader/edcas_lights.hlsl`); 0 is none. Set from the settings in
    /// [`crate::ground_tint`] (the forecast starts at [`LAMP_GLOW`]).
    pub lamp_glow: f32,
}

impl Clouds {
    /// No clouds. Only `x503` matters then; the writer leaves the others as they are.
    pub const NONE: Clouds = Clouds { coverage: 0.0, size: 0.0, height: 0.0, brightness: 0.0, opacity: 0.0, slab_opacity: 0.0, thickness: 0.0, volumetric: false, streaks: 0.0, glow: 0.0, iridescence: 0.0, fog: 0.0, radius_m: 0.0, fog_depth_m: 0.0, softness: 0.0, frost: 0.0, lamp_glow: 0.0 };

    /// The values file's lines. Without clouds that is `x503 = 0` and `x507 = 0` alone: the other
    /// values are left as they were, so the file keeps the last clouds' look (or the defaults of
    /// `edcas.ini` before any), but the fog of the last planet doesn't stay on one without air.
    /// `z505` is the thickness for the slab and 0 for the thin shell, and `x504` goes with it.
    /// `y507`, `x508`, `x509` and `w509` (the fog's depth, the clouds' edge, the canopy frost, the lamps' glow) come last, with clouds
    /// only: the frost needs cloud or fog anyway, and its own measure fades out without them.
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        if self.coverage <= 0.0 {
            return vec![("x503", "0.000".to_string()), ("x507", "0.000".to_string())];
        }
        // A slab of no thickness (Cloud Thickness 0 %) is the thin shell, with the shell's opacity:
        // the slab's x504 drew it about three times too opaque.
        let (opacity, thickness) =
            if self.volumetric && self.thickness > 0.0 { (self.slab_opacity, self.thickness) } else { (self.opacity, 0.0) };
        [
            ("x503", self.coverage),
            ("y503", self.size),
            ("z503", self.height),
            ("w503", self.brightness),
            ("x504", opacity),
            ("y504", self.streaks),
            ("w504", self.iridescence),
            ("z505", thickness),
        ]
        .map(|(name, v)| (name, format!("{v:.3}")))
        .into_iter()
        .chain([
            ("x507", format!("{:.3}", self.fog)),
            ("w507", format!("{:.0}", self.radius_m)),
            ("y507", format!("{:.0}", self.fog_depth_m)),
            ("x508", format!("{:.3}", self.softness)),
            ("x509", format!("{:.3}", self.frost)),
            ("w509", format!("{:.3}", self.lamp_glow)),
        ])
        .collect()
    }
}

/// Why a planet has no clouds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoClouds {
    /// No atmosphere, or the scan has none of its data.
    Airless,
    /// There is air, but none of its gases freeze into clouds (helium, hydrogen, …).
    NothingFreezes,
    /// Every gas that could is too warm to freeze at any height.
    TooWarm,
}

/// The clouds for an atmosphere and where they come from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Forecast {
    pub clouds: Clouds,
    /// What the clouds are made of ("water ice", "water clouds" for droplets) and how high
    /// the gas's frost line is: 0 at the ground,
    /// 1 at the top of the air. Negative: the gas freezes right at the surface.
    pub source: Result<(&'static str, f64), NoClouds>,
}

impl Forecast {
    pub const AIRLESS: Forecast = Forecast { clouds: Clouds::NONE, source: Err(NoClouds::Airless) };

    /// One line for the settings, e.g. "Clouds: water ice, high, sparse".
    pub fn summary(&self) -> String {
        match self.source {
            Err(NoClouds::Airless) => "No clouds: no atmosphere here".to_string(),
            Err(NoClouds::NothingFreezes) => "No clouds: none of its gases freeze".to_string(),
            Err(NoClouds::TooWarm) => "No clouds: too warm for any gas to freeze".to_string(),
            Ok((label, m)) => {
                let height = match m {
                    m if m < 0.0 => "fog on the ground",
                    m if m < 0.33 => "low",
                    m if m < 0.66 => "mid height",
                    _ => "high",
                };
                // The words go by the modelled coverage, before it is scaled for drawing.
                let c = f64::from(self.clouds.coverage) / COVERAGE_SCALE;
                let spread = if c < 0.3 {
                    "sparse"
                } else if c < 0.5 {
                    "scattered"
                } else {
                    "widespread"
                };
                format!("Clouds: {label}, {height}, {spread}")
            }
        }
    }
}

/// The clouds of an atmosphere. `gases` are the game's names with their share in percent,
/// `pressure_pa` the surface pressure, `surface_k` the surface temperature and `strength`
/// (0–1) how visible the clouds are made.
pub fn forecast(gases: &[(&str, f32)], pressure_pa: f32, surface_k: f32, strength: f32) -> Forecast {
    let (pressure, surface) = (f64::from(pressure_pa), f64::from(surface_k));
    if gases.is_empty() || pressure <= 0.0 || surface <= 0.0 {
        return Forecast::AIRLESS;
    }
    let top = TOP_OF_AIR * surface;
    // The gas that freezes lowest wins: smallest m, the frost line's height in the air.
    let mut best: Option<(&Ice, f64, f64, f64, bool)> = None;
    let mut freezers = 0;
    for &(name, percent) in gases {
        let Some(ice) = ICES.iter().find(|i| i.name == name) else { continue };
        let partial = pressure * f64::from(percent) / 100.0;
        if percent < MIN_PERCENT || partial <= 0.0 {
            continue;
        }
        let Some(t_frost) = ice.frost_point(partial) else { continue };
        freezers += 1;
        let m = (surface - t_frost) / (surface - top);
        if best.is_none_or(|(_, lowest, ..)| m < lowest) {
            best = Some((ice, m, partial, f64::from(percent), t_frost > ice.t_triple));
        }
    }
    let Some((ice, m, partial, percent, droplets)) = best else {
        let reason = if freezers == 0 { NoClouds::NothingFreezes } else { NoClouds::TooWarm };
        return Forecast { clouds: Clouds::NONE, source: Err(reason) };
    };
    if m >= 1.0 {
        return Forecast { clouds: Clouds::NONE, source: Err(NoClouds::TooWarm) };
    }

    // A trace gas covers less of the sky: full cover from a quarter of the air up.
    let abundance = ((percent / 100.0).sqrt() + 0.3).clamp(0.0, 1.0);
    let high = if m < 0.0 { 0.1 } else { (0.15 + 0.7 * m).clamp(0.1, 0.85) };
    let opacity = ((partial.log10() / 4.0).clamp(0.15, 0.8) as f32) * strength.clamp(0.0, 1.0) * OPACITY_SCALE;
    let clouds = Clouds {
        coverage: (COVERAGE_SCALE * abundance * if m < 0.0 { 0.5 } else { (0.15 + 0.55 * (1.0 - m)).clamp(0.0, 0.7) }) as f32,
        size: 8.0,
        height: high as f32,
        brightness: BRIGHTNESS,
        opacity,
        slab_opacity: opacity * SLAB_OPACITY_SCALE,
        // Fog hugs the ground, high clouds are thin flat layers, the rest is in between.
        thickness: if m < 0.0 {
            FOG_THICKNESS
        } else if m > 0.66 {
            HIGH_THICKNESS
        } else {
            THICKNESS
        },
        volumetric: true,
        streaks: if m < 0.0 { 0.3 } else { (0.6 + 0.4 * m).clamp(0.0, 1.0) } as f32,
        glow: if high > 0.5 { 1.0 } else { 0.3 },
        iridescence: if ice.name == "CarbonDioxide" && m > 0.5 { 0.5 } else { 0.0 },
        fog: (FOG_SCALE * abundance * (1.0 - m / FOG_MAX_M).clamp(0.0, 1.0) * f64::from(strength.clamp(0.0, 1.0))) as f32,
        radius_m: 0.0,
        fog_depth_m: FOG_DEPTH_M,
        softness: EDGE_SOFTNESS,
        frost: FROST_STRENGTH,
        lamp_glow: LAMP_GLOW,
    };
    Forecast { clouds, source: Ok((if droplets { ice.droplets } else { ice.label }, m)) }
}

/// The base gas of an `AtmosphereType` ("CarbonDioxideRich" → "CarbonDioxide"), for scans
/// that carry the type but no composition.
fn base_gas(atmosphere_type: &str) -> Option<&str> {
    let gas = atmosphere_type.strip_suffix("Rich").unwrap_or(atmosphere_type);
    ICES.iter().any(|i| i.name == gas).then_some(gas)
}

/// The clouds of a scanned body. Bodies the commander hasn't scanned (only known from the
/// server) have no atmosphere data, so no clouds.
pub fn for_body(body: &BodyScan, strength: f32) -> Forecast {
    let Some(exo) = &body.exo else { return Forecast::AIRLESS };
    let gases: Vec<(&str, f32)> = if exo.composition.is_empty() {
        match base_gas(&exo.atmosphere_type) {
            Some(gas) => vec![(gas, 100.0)],
            None => return Forecast::AIRLESS,
        }
    } else {
        exo.composition.iter().map(|g| (g.name.as_str(), g.percent)).collect()
    };
    let mut forecast = forecast(&gases, exo.surface_pressure, body.surface_temperature, strength);
    if forecast.source.is_ok() {
        forecast.clouds.radius_m = body.radius;
    }
    forecast
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal_reader::{AtmosphereGas, ExoScan};

    fn near(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn ice(name: &str) -> &'static Ice {
        ICES.iter().find(|i| i.name == name).unwrap()
    }

    #[test]
    fn frost_points_follow_clausius_clapeyron() {
        // CO2 at 570 Pa: 1/Tf = 1/216.58 - (8.314/25200) ln(570/518000)
        //   = 4.6172e-3 + 3.2992e-4 * 6.8118 = 6.8647e-3 -> 145.7 K.
        assert!(near(ice("CarbonDioxide").frost_point(570.0).unwrap(), 145.67, 0.05));
        // At its triple point pressure a gas freezes at its triple point.
        assert!(near(ice("Water").frost_point(611.66).unwrap(), 273.16, 1e-9));
        // Water at 0.18 Pa: 1/Tf = 1/273.16 + (8.314/51000) * 8.1307 = 4.9862e-3 -> 200.6 K.
        assert!(near(ice("Water").frost_point(0.18).unwrap(), 200.55, 0.05));
    }

    #[test]
    fn a_mars_like_sky_has_high_sparse_carbon_dioxide_clouds() {
        // CO2 95 % of 600 Pa = 570 Pa -> Tf 145.67 K; surface 210 K, top 126 K:
        // m = (210 - 145.67) / 84 = 0.766.
        let f = forecast(&[("CarbonDioxide", 95.0), ("Nitrogen", 5.0)], 600.0, 210.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!(label, "carbon dioxide ice");
        assert!(near(m, 0.766, 0.002), "{m}");
        let c = f.clouds;
        // coverage (0.15 + 0.55 * 0.234 = 0.279) * 0.40 = 0.112; height 0.15 + 0.7 * 0.766 = 0.686
        assert!(near(c.coverage.into(), 0.279 * 0.40, 0.002), "{c:?}");
        assert!(near(c.height.into(), 0.686, 0.002), "{c:?}");
        assert!(near(c.streaks.into(), 0.906, 0.002), "{c:?}");
        // opacity: log10(570) / 4 = 0.6890, times 0.18 = 0.124
        assert!(near(c.opacity.into(), 0.689 * 0.18, 0.002), "{c:?}");
        assert_eq!((c.size, c.brightness, c.glow, c.iridescence), (8.0, 0.8, 1.0, 0.5));
        assert_eq!(f.summary(), "Clouds: carbon dioxide ice, high, sparse");
    }

    #[test]
    fn a_trace_of_water_beats_the_carbon_dioxide() {
        // Water 0.03 % of 600 Pa = 0.18 Pa -> Tf 200.55 K: m = (210 - 200.55) / 84 = 0.1125.
        let f = forecast(&[("CarbonDioxide", 95.0), ("Water", 0.03), ("Nitrogen", 4.97)], 600.0, 210.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!(label, "water ice");
        assert!(near(m, 0.1125, 0.002), "{m}");
        let c = f.clouds;
        // coverage 0.15 + 0.55 * 0.8875 = 0.638, times the trace factor
        // sqrt(0.0003) + 0.3 = 0.317 -> 0.202, times 0.40 = 0.081; height 0.15 + 0.7 * 0.1125 = 0.229
        assert!(near(c.coverage.into(), 0.202 * 0.40, 0.002), "{c:?}");
        assert!(near(c.height.into(), 0.229, 0.002), "{c:?}");
        assert_eq!((c.glow, c.iridescence), (0.3, 0.0), "low and not carbon dioxide");
        // opacity: log10(0.18) / 4 is negative, so it floors at 0.15, times 0.18 = 0.027.
        assert!(near(c.opacity.into(), 0.15 * 0.18, 1e-6), "{c:?}");
        assert_eq!(f.summary(), "Clouds: water ice, low, sparse");
    }

    #[test]
    fn neon_this_cold_and_thin_is_too_warm_to_freeze() {
        // Neon 2000 Pa: 1/Tf = 1/24.56 - (8.314/2050) ln(2000/43370)
        //   = 0.040717 + 4.0556e-3 * 3.0768 = 0.053195 -> 18.8 K. Surface 40 K, top 24 K:
        // m = (40 - 18.8) / 16 = 1.33 >= 1: the air never gets cold enough.
        assert!(near(ice("Neon").frost_point(2000.0).unwrap(), 18.80, 0.05));
        let f = forecast(&[("Neon", 100.0)], 2000.0, 40.0, 1.0);
        assert_eq!(f.source, Err(NoClouds::TooWarm));
        assert_eq!(f.clouds.coverage, 0.0);
        // Colder (25 K, top 15 K): m = (25 - 18.8) / 10 = 0.62 -> clouds.
        let f = forecast(&[("Neon", 100.0)], 2000.0, 25.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!(label, "neon ice");
        assert!(near(m, 0.62, 0.005), "{m}");
    }

    #[test]
    fn gases_that_never_freeze_and_hot_air_make_no_clouds() {
        let f = forecast(&[("Helium", 100.0)], 5000.0, 100.0, 1.0);
        assert_eq!(f.source, Err(NoClouds::NothingFreezes));
        assert_eq!(forecast(&[("Hydrogen", 50.0), ("Iron", 50.0)], 5000.0, 100.0, 1.0).source, Err(NoClouds::NothingFreezes));
        // SO2 1000 Pa -> Tf 192.7 K; surface 400 K, top 240 K: m = 1.3.
        let f = forecast(&[("SulphurDioxide", 100.0)], 1000.0, 400.0, 1.0);
        assert_eq!(f.source, Err(NoClouds::TooWarm));
        assert_eq!(f.summary(), "No clouds: too warm for any gas to freeze");
        // A share under 0.01 % isn't a cloud maker, and zero pressure is airless.
        assert_eq!(forecast(&[("Water", 0.005)], 600.0, 210.0, 1.0).source, Err(NoClouds::NothingFreezes));
        assert_eq!(forecast(&[("Water", 100.0)], 0.0, 210.0, 1.0).source, Err(NoClouds::Airless));
    }

    #[test]
    fn a_cold_nitrogen_sky_sits_halfway_up() {
        // N2 3000 Pa: Tf 56.43 K; surface 70 K, top 42 K: m = 13.57 / 28 = 0.485.
        let f = forecast(&[("Nitrogen", 100.0)], 3000.0, 70.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!(label, "nitrogen ice");
        assert!(near(ice("Nitrogen").frost_point(3000.0).unwrap(), 56.43, 0.05));
        assert!(near(m, 0.485, 0.002), "{m}");
        assert_eq!(f.clouds.iridescence, 0.0);
        assert_eq!(f.clouds.glow, 0.3, "height 0.49 is not above 0.5");
        // opacity: log10(3000) / 4 = 0.8693 -> capped at 0.8, times the 50 % strength and 0.18 = 0.072.
        let half = forecast(&[("Nitrogen", 100.0)], 3000.0, 70.0, 0.5);
        assert!(near(half.clouds.opacity.into(), 0.4 * 0.18, 1e-6), "{:?}", half.clouds);
    }

    #[test]
    fn gases_freezing_at_the_surface_give_a_low_fog() {
        // Water 100 % of 600 Pa -> Tf 273.2 K; surface 200 K: m < 0.
        let f = forecast(&[("Water", 100.0)], 600.0, 200.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!(label, "water ice");
        assert!(m < 0.0);
        let c = f.clouds;
        assert_eq!((c.coverage, c.height, c.streaks, c.glow, c.iridescence), (0.5 * 0.40, 0.1, 0.3, 0.3, 0.0));
        assert_eq!(f.summary(), "Clouds: water ice, fog on the ground, widespread");
    }

    #[test]
    fn a_hot_thin_water_world_has_droplets_not_ice() {
        // Water 2000 Pa: 1/Tf = 1/273.16 - (8.314/51000) ln(2000/611.66) = 3.4677e-3 -> 288.4 K,
        // above the triple point 273.16 K. Surface 400 K, top 240 K: m = 111.6 / 160 = 0.70.
        let f = forecast(&[("Water", 100.0)], 2000.0, 400.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!(label, "water clouds");
        assert!(near(m, 0.70, 0.005), "{m}");
        // coverage 0.15 + 0.55 * 0.30 = 0.315
        assert_eq!(f.summary(), "Clouds: water clouds, high, scattered");
        // Below the triple point (100 Pa -> 252.8 K) it is ice.
        assert_eq!(forecast(&[("Water", 100.0)], 100.0, 400.0, 1.0).source.unwrap().0, "water ice");
    }

    #[test]
    fn a_trace_gas_covers_less_of_the_sky_down_to_the_fog() {
        // Water 1 % of 60000 Pa = 600 Pa -> Tf 273.2 K; surface 200 K: fog, 0.5 * (0.1 + 0.3) = 0.2, times 0.40 = 0.08.
        let f = forecast(&[("Water", 1.0), ("Helium", 99.0)], 60_000.0, 200.0, 1.0);
        assert!(f.source.unwrap().1 < 0.0);
        assert!(near(f.clouds.coverage.into(), 0.2 * 0.40, 0.001), "{:?}", f.clouds);
        // From about half the air (sqrt(0.7) + 0.3 > 1) the cover is the full 0.5 * 0.40 = 0.2.
        let f = forecast(&[("Water", 70.0), ("Helium", 30.0)], 1500.0, 200.0, 1.0);
        assert!(near(f.clouds.coverage.into(), 0.5 * 0.40, 0.001), "{:?}", f.clouds);
    }

    #[test]
    fn no_clouds_write_x3_and_x7_alone() {
        // x507 too, so the fog of the last planet doesn't stay on an airless one.
        let none = vec![("x503", "0.000".to_string()), ("x507", "0.000".to_string())];
        assert_eq!(Clouds::NONE.assignments(), none);
        let names: Vec<_> = forecast(&[("Nitrogen", 100.0)], 3000.0, 70.0, 1.0).clouds.assignments().iter().map(|(n, _)| *n).collect();
        assert_eq!(names, ["x503", "y503", "z503", "w503", "x504", "y504", "w504", "z505", "x507", "w507", "y507", "x508", "x509", "w509"]);
    }

    #[test]
    fn the_fog_is_full_where_the_gas_freezes_at_the_ground_and_gone_from_a_third_up() {
        // Water 100 % of 600 Pa on a 200 K ground: m < 0, share 1: 0.24 * strength.
        let f = forecast(&[("Water", 100.0)], 600.0, 200.0, 1.0);
        assert!(f.source.unwrap().1 < 0.0);
        assert!(near(f.clouds.fog.into(), 0.24, 1e-6), "{:?}", f.clouds);
        let half = forecast(&[("Water", 100.0)], 600.0, 200.0, 0.5).clouds;
        assert!(near(half.fog.into(), 0.12, 1e-6), "{half:?}");
        // The Mars-like sky (m 0.766) and the cold nitrogen one (m 0.485) are past 0.3: no fog.
        assert_eq!(forecast(&[("CarbonDioxide", 95.0), ("Nitrogen", 5.0)], 600.0, 210.0, 1.0).clouds.fog, 0.0);
        assert_eq!(forecast(&[("Nitrogen", 100.0)], 3000.0, 70.0, 1.0).clouds.fog, 0.0);
    }

    #[test]
    fn the_fog_of_6_e_is_the_x7_that_looked_right() {
        // Hegoea XL-K c22-8 6 e: pure CO2, 804.6 Pa, 156.03 K. Tf 148.1 K, top 93.6 K:
        // m = 7.9 / 62.4 = 0.127, share 1 - 0.127 / 0.3 = 0.577, times 0.72 times 0.24 = 0.0998.
        let f = forecast(&[("CarbonDioxide", 100.0)], 804.6, 156.03, 0.72);
        assert!(near(f.source.unwrap().1, 0.127, 0.001));
        assert!(near(f.clouds.fog.into(), 0.10, 0.001), "{:?}", f.clouds);
        // On a 175 K ground the same air has m 0.38, past 0.3: no fog.
        let warm = forecast(&[("CarbonDioxide", 100.0)], 804.6, 175.0, 0.72);
        assert!(warm.source.unwrap().1 > 0.3 && warm.clouds.fog == 0.0, "{:?}", warm.clouds);
    }

    #[test]
    fn a_trace_gas_makes_less_fog() {
        // The trace of water over carbon dioxide: m 0.1125, share 0.625, abundance 0.317, times 0.24.
        let f = forecast(&[("CarbonDioxide", 95.0), ("Water", 0.03), ("Nitrogen", 4.97)], 600.0, 210.0, 1.0);
        assert!(near(f.clouds.fog.into(), 0.625 * 0.317 * 0.24, 0.001), "{:?}", f.clouds);
        // The same gas as the whole air would make 0.625 * 0.24 = 0.15.
        let full = forecast(&[("Water", 100.0)], 600.0, 210.0, 1.0).clouds.fog;
        assert!(full > 3.0 * f.clouds.fog, "{full}");
    }

    #[test]
    fn the_fog_and_the_radius_are_written_after_the_clouds() {
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        let mut c = forecast(&[("CarbonDioxide", 100.0)], 804.6, 156.03, 0.72).clouds;
        c.radius_m = 564_697.0;
        assert_eq!((get(&c, "x507").as_str(), get(&c, "w507").as_str()), ("0.100", "564697"), "{c:?}");
        // The fog setting (see ground_tint) zeroes the density, the radius stays.
        c.fog = 0.0;
        assert_eq!((get(&c, "x507").as_str(), get(&c, "w507").as_str()), ("0.000", "564697"), "{c:?}");
    }

    #[test]
    fn the_fog_depth_the_edge_softness_and_the_frost_start_at_the_shaders_defaults() {
        let mut c = forecast(&[("CarbonDioxide", 100.0)], 804.6, 156.03, 0.72).clouds;
        c.radius_m = 564_697.0;
        let get = |name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!((get("y507").as_str(), get("x508").as_str(), get("x509").as_str()), ("800", "0.500", "1.000"), "{c:?}");
        assert_eq!((get("x507").as_str(), get("z505").as_str()), ("0.100", "0.100"), "{c:?}");
    }

    #[test]
    fn the_slab_is_thinner_up_high_and_thicker_in_fog() {
        // Mid height: N2 at m 0.485. High: the Mars-like sky (m 0.766). Fog: water on a 200 K ground.
        assert_eq!(forecast(&[("Nitrogen", 100.0)], 3000.0, 70.0, 1.0).clouds.thickness, 0.1);
        let high = forecast(&[("CarbonDioxide", 95.0), ("Nitrogen", 5.0)], 600.0, 210.0, 1.0);
        assert_eq!(high.clouds.thickness, 0.06);
        assert_eq!(forecast(&[("Water", 100.0)], 600.0, 200.0, 1.0).clouds.thickness, 0.2);
    }

    #[test]
    fn the_slab_opacity_is_the_thin_one_scaled() {
        // The 6 e-like case: pure CO2 at 600 Pa, 210 K. log10(600) / 4 = 0.6945, times 0.18
        // = 0.125 thin, times 0.3 / 0.09 = 0.417.
        let f = forecast(&[("CarbonDioxide", 100.0)], 600.0, 210.0, 1.0).clouds;
        assert!(near(f.opacity.into(), 0.6945 * 0.18, 0.001), "{f:?}");
        assert!(near(f.slab_opacity.into(), 0.6945 * 0.18 * 0.3 / 0.09, 0.003), "{f:?}");
        // Where the thin opacity is the 0.09 of 6 e, the slab one is 0.3.
        let half = forecast(&[("CarbonDioxide", 100.0)], 600.0, 210.0, 0.72).clouds;
        assert!(near(half.opacity.into(), 0.09, 0.002) && near(half.slab_opacity.into(), 0.3, 0.006), "{half:?}");
        // The densest air: 0.8 * 0.18 = 0.144 thin, 0.48 in the slab.
        let dense = forecast(&[("Nitrogen", 100.0)], 100_000.0, 70.0, 1.0).clouds;
        assert!(near(dense.opacity.into(), 0.144, 1e-6) && near(dense.slab_opacity.into(), 0.48, 1e-5), "{dense:?}");
    }

    #[test]
    fn the_thin_shell_writes_z5_zero_and_its_own_opacity() {
        let mut c = forecast(&[("CarbonDioxide", 100.0)], 600.0, 210.0, 0.72).clouds;
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!((get(&c, "z505").as_str(), get(&c, "x504").as_str()), ("0.060", "0.300"), "{c:?}");
        c.volumetric = false;
        assert_eq!((get(&c, "z505").as_str(), get(&c, "x504").as_str()), ("0.000", "0.090"), "{c:?}");
        // A slab of no thickness is the thin shell too, with the shell's opacity.
        c.volumetric = true;
        c.thickness = 0.0;
        assert_eq!((get(&c, "z505").as_str(), get(&c, "x504").as_str()), ("0.000", "0.090"), "{c:?}");
        // Without clouds x503 and x507 stay alone in either mode.
        assert_eq!(Clouds { volumetric: true, ..Clouds::NONE }.assignments(), vec![("x503", "0.000".to_string()), ("x507", "0.000".to_string())]);
    }

    #[test]
    fn a_scan_without_composition_assumes_the_base_gas_of_its_type() {
        let body = |atmosphere_type: &str, composition: Vec<AtmosphereGas>| BodyScan {
            surface_temperature: 210.0,
            exo: Some(ExoScan { atmosphere_type: atmosphere_type.to_string(), surface_pressure: 600.0, composition, so2_percent: 0.0 }),
            ..Default::default()
        };
        // "CarbonDioxideRich" -> 100 % carbon dioxide at 600 Pa: Tf 146.0 K -> high clouds.
        let f = for_body(&body("CarbonDioxideRich", vec![]), 1.0);
        assert_eq!(f.source.map(|(l, _)| l), Ok("carbon dioxide ice"));
        assert_eq!(f.clouds.radius_m, 0.0, "the scan has no radius here");
        // The radius of the body goes to w507.
        let f = for_body(&BodyScan { radius: 564_697.0, ..body("CarbonDioxideRich", vec![]) }, 1.0);
        assert_eq!(f.clouds.radius_m, 564_697.0);
        // The composition wins when there is one.
        let gas = |name: &str, percent| AtmosphereGas { name: name.to_string(), percent };
        let f = for_body(&body("CarbonDioxideRich", vec![gas("Helium", 100.0)]), 1.0);
        assert_eq!(f.source, Err(NoClouds::NothingFreezes));
        // Airless, unknown types and unscanned bodies.
        assert_eq!(for_body(&body("None", vec![]), 1.0), Forecast::AIRLESS);
        assert_eq!(for_body(&body("", vec![]), 1.0), Forecast::AIRLESS);
        assert_eq!(for_body(&BodyScan::default(), 1.0), Forecast::AIRLESS);
    }
}
