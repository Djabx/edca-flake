//! The wind-blown dust a planet's air can lift (`scripts/ed-shader/edcas_dust.hlsl`): sand
//! streaming in sheets along the ground, and billows in gusts.
//! Mars's dust devils are left out: the shader can't light or shade towers that tall.
//!
//! Whether the wind moves sand at all is a threshold ([`start_threshold`]): the friction speed at
//! which grains start to hop grows as `1 / sqrt(ρ_air)`, and only with the fourth root of gravity
//! and of the grain's density, because the grain size that moves most easily shifts with them
//! (Shao and Lu's form at its minimum; Kok et al. 2012, arXiv 1201.4353). Against it stands a
//! gust: Mars lifted dust at Jezero only in winds of 15 m/s and more at 1.5 m (Newman et al. 2022),
//! a friction speed of 1.2 m/s ([`GUST_FRICTION`]). Their ratio S decides: Mars has 0.72 and gets
//! the strength tuned for it; under 0.6 the dust fades and by 0.3 there is none (CO2 at Mars's
//! gravity: from about 420 Pa down to 105 Pa), from about 1.5 on it is at full. The threshold is
//! good to a factor 2 (recent wind tunnels put Mars's at 0.6 to 1.0 m/s, the classical value is 1.4
//! to 2), and the fade's range is a judgement on six Solar System bodies: the thing to look at in
//! game. Derived in `scripts/ed-shader/reports/Planetary dust and wind rendering.md`, proposal A1;
//! until client 0.43.147 every body from 100 Pa on got at least 0.15 of its grain supply.
//!
//! Ice grains (0.93 g/cm³) start at 0.77 of rock's (2.65) threshold while they are colder than 170 K;
//! from 200 K on water ice is ten times as sticky and they start at 1.37 of it ([`cohesion`]): cold
//! icy worlds blow a little more easily than rocky ones, warm ones less. Worlds of 1 to 2 Pa (Pluto's
//! dunes, Triton's streaks) are not this: their grains are lifted by subliming ice and geysers, and
//! the wind only carries them; the gate at 100 Pa leaves them out. How much loose grain there is
//! goes by the ground ([`GroundType`]): sand and frost are covered in it, rocky ice less, fresh
//! lava crust has little, its dark ash only in patches.
//!
//! The sheet of hopping grains is as deep as the gravity is low ([`sheet_depth`]). Where the gust only
//! just reaches the threshold the dust comes in passing fronts with clear ground between ([`activity`])
//! and dies at night, when the wind near the ground falls to a third ([`night_share`]). The fine dust
//! that was lifted stays up for days to months, mixed through kilometres of air: a haze through the
//! whole column ([`DUST_HAZE`]), which follows neither the hour nor the fronts.
//!
//! Written as `x515` (strength), `y515` (wind, m/s), `w515` (radius, m), `x516` to `z516` (colour), `x519` (the far drift's reach, m), `y519` (the sheet's depth, m), `z519` (the fronts' share), `w519` (the night's dust over the day's), `x531` and `y531` (the dust haze and the air's scale height) of
//! `ShaderFixes/edcas_params.txt`; `z515` is the shader's clock, set by `edcas.ini`.

use crate::ground_tint::{ground_type, makeup, GroundType, Makeup};
use crate::journal_reader::BodyScan;
use crate::planet_clouds::mean_molar_mass;

/// Below this the scan's "atmosphere" is too thin for the wind to move anything (the journals
/// list no atmosphere under it, 2026-10-29).
const MIN_PRESSURE_PA: f32 = 100.0;
/// Grain densities (kg/m³): Mars's sand (quartz-like, the reference) and water ice.
const ROCK_GRAIN: f64 = 2650.0;
const ICE_GRAIN: f64 = 930.0;
/// The strength on a Mars-like planet: dust moves, but is seldom thick.
const MARS_STRENGTH: f64 = 0.35;
/// The gust's friction speed over the start threshold on the Mars of the tests (610 Pa of CO2 at
/// 210 K, 3.71 m/s², rock grains): there the strength is [`MARS_STRENGTH`].
const MARS_GUST_RATIO: f64 = 0.72;
/// The cohesion in the start threshold, N/m (a fitted parameter, 1.65e-4 to 5e-4 in Kok et al. 2012),
/// of rock and of cold ice.
const COHESION: f64 = 3e-4;
/// Water ice warmer than about 200 K sticks at least ten times as hard as silicates (Gundlach and
/// Blum 2015); under 170 to 175 K it is no stickier than rock (Musiolik and Wurm 2019). Only the ratio
/// is used: the cohesion above is fitted, not a surface energy. The scan does not say which ice a
/// body has; this takes it for water ice. No source was found for SO2, CO2 or NH3 frost.
const WARM_ICE_COHESION: f64 = 10.0;
const ICE_WARMS_K: (f64, f64) = (170.0, 200.0);
/// The friction speed over the wind at 1.5 m: the log law over a roughness of 1 cm,
/// `0.4 / ln(1.5 / 0.01)`.
const FRICTION_PER_WIND: f64 = 0.080;
/// The friction speed of a gust that lifts dust, m/s: 15 m/s at 1.5 m (Jezero). One value for every
/// body: the winds that are known vary far less than the threshold does.
const GUST_FRICTION: f64 = FRICTION_PER_WIND * 15.0;
/// The gust ratio under which there is no dust, and from which it is not thinned.
const GUST_FADE: (f64, f64) = (0.3, 0.6);
/// The range of the wind (m/s) the shader is given: the wind at 1.5 m at which the sand starts to
/// move, 20.7 on Mars (the classical threshold; the dust moves in gusts of about that). The floor is
/// for the eye: at 3 m/s the streaks looked still, at 10 they moved (7 c in Pro Thaa RR-W d1-67,
/// 2026-10-29).
const WIND_RANGE: (f64, f64) = (10.0, 25.0);
/// How far the dust drifting over the land reaches by default, metres (the shader's FAR_REACH_M).
const DEFAULT_REACH_M: f32 = 15_000.0;
/// Dust colours (linear) of rock, metal and ice: Mars's ochre dust, a darker grey-brown, frost.
const ROCK_DUST: [f32; 3] = [0.62, 0.45, 0.32];
const METAL_DUST: [f32; 3] = [0.42, 0.38, 0.34];
const ICE_DUST: [f32; 3] = [0.85, 0.88, 0.92];
/// Basalt ash, on lava worlds.
const LAVA_DUST: [f32; 3] = [0.22, 0.21, 0.20];

/// How much loose grain the wind finds on `ground`, 0–1 (a judgement, not a measurement): sand
/// is made of it (the most, tuned in game on a sand world, 2026-10-29); regolith and frost have
/// a loose layer over firmer ground, rocky ice a dust lag over ice, bare ice and fresh lava
/// crust little.
fn grain_supply(ground: Option<GroundType>) -> f32 {
    match ground {
        Some(GroundType::Sand) => 1.0,
        Some(GroundType::Regolith | GroundType::Frost) => 0.6,
        Some(GroundType::RockyIce) => 0.45,
        Some(GroundType::Ice | GroundType::Lava) => 0.3,
        None => 0.5,
    }
}

/// What 3DMigoto gets for the dust.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dust {
    /// `x515`: 0 = none, 1 = thick.
    pub strength: f32,
    /// `y515`: m/s.
    pub wind: f32,
    /// `w515`: the planet's radius in metres, for the shader's metres.
    pub radius_m: f32,
    /// `x516`, `y516`, `z516`.
    pub colour: [f32; 3],
    /// `x519`: how far the dust drifting over the land reaches, metres (the player's setting).
    pub reach_m: f32,
    /// `y519`: the saltation sheet's scale height, metres ([`sheet_depth`]).
    pub sheet_m: f32,
    /// `z519`: the share of the ground a dust front lies over, 0.05 to 1 ([`activity`]).
    pub activity: f32,
    /// `w519`: the night's dust over the day's, 0.001 to 1 ([`night_share`]).
    pub night: f32,
    /// `x531`: the dust haze's vertical optical depth ([`DUST_HAZE`]); 0 = none.
    pub haze: f32,
    /// `y531`: the air's scale height, metres: how the haze thins with the height.
    pub scale_m: f32,
    /// How much loose grain the ground has, 0–1 ([`grain_supply`]; 0.5 for an unknown ground),
    /// in the strength. Not written itself.
    pub loose: f32,
}

impl Dust {
    pub const NONE: Dust = Dust { strength: 0.0, wind: 0.0, radius_m: 0.0, colour: [0.0; 3], reach_m: DEFAULT_REACH_M, sheet_m: 0.8, activity: 1.0, night: 1.0, haze: 0.0, scale_m: 0.0, loose: 0.5 };

    /// The values file's lines. Without dust that is `x515 = 0` and the haze's `x531 = 0` alone: the
    /// rest stays as it was, so 3DMigoto's fade only takes the strengths down.
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        if self.strength <= 0.0 {
            return vec![("x515", "0.000".to_string()), ("x531", "0.0000".to_string())];
        }
        vec![
            ("x515", format!("{:.3}", self.strength)),
            ("y515", format!("{:.1}", self.wind)),
            ("w515", format!("{:.0}", self.radius_m)),
            ("x516", format!("{:.3}", self.colour[0])),
            ("y516", format!("{:.3}", self.colour[1])),
            ("z516", format!("{:.3}", self.colour[2])),
            ("x519", format!("{:.0}", self.reach_m)),
            ("y519", format!("{:.2}", self.sheet_m)),
            ("z519", format!("{:.3}", self.activity)),
            ("w519", format!("{:.3}", self.night)),
            ("x531", format!("{:.4}", self.haze)),
            ("y531", format!("{:.0}", self.scale_m)),
        ]
    }
}

/// The air's density at the ground, kg/m³ (ideal gas).
fn air_density(body: &BodyScan, pressure_pa: f32, gases: &[(&str, f32)]) -> f64 {
    f64::from(pressure_pa) * mean_molar_mass(gases) / (8.314 * f64::from(body.surface_temperature.max(1.0)))
}

/// The body's makeup, rock when unknown.
fn makeup_or_rock(body: &BodyScan) -> Makeup {
    makeup(body).unwrap_or(Makeup { ice: 0.0, rock: 1.0, metal: 0.0 })
}

/// The dust's colour for a body, by what its ground is made of: ash on lava worlds.
fn colour_of(body: &BodyScan) -> [f32; 3] {
    if ground_type(body) == Some(GroundType::Lava) {
        return LAVA_DUST;
    }
    let m = makeup_or_rock(body);
    let parts = (m.ice + m.rock + m.metal).max(1e-6);
    [0, 1, 2].map(|i| (m.rock * ROCK_DUST[i] + m.metal * METAL_DUST[i] + m.ice * ICE_DUST[i]) / parts)
}

/// The share of ice in the grains at the surface, 0 to 1: the scan's metal is the whole planet's,
/// mostly in its core, so the grains are rock or ice.
fn ice_share(body: &BodyScan) -> f64 {
    let m = makeup_or_rock(body);
    f64::from(m.ice / (m.ice + m.rock + m.metal).max(1e-6))
}

/// The grains' density (kg/m³) at an ice share.
fn grain_density(ice: f64) -> f64 {
    ice * ICE_GRAIN + (1.0 - ice) * ROCK_GRAIN
}

/// The grains' cohesion (N/m) at an ice share and a surface temperature (K).
fn cohesion(ice: f64, kelvin: f64) -> f64 {
    COHESION * (1.0 + (WARM_ICE_COHESION - 1.0) * ice * smoothstep(ICE_WARMS_K.0, ICE_WARMS_K.1, kelvin))
}

/// The saltation sheet's scale height, metres: the flux of hopping sand falls by e every 5 to 10 cm on
/// Earth (Martin and Kok 2017) and the hops are as high as the gravity is low. 0.1 m at 1 g, 0.26 on
/// Mars, 0.63 on 7 c, 1 at a tenth of a g, at most 2. The Earth value is measured; every other is the
/// 1 / g extrapolation (DUST-PLAN.md phase 3). The shader had 0.8 m for every body.
fn sheet_depth(gravity: f64) -> f64 {
    (0.1 * 9.81 / gravity).clamp(0.05, 2.0)
}

/// How thick the dust is for a gust ratio, before the grain supply and the player's setting: Mars's
/// strength at Mars's ratio, in proportion to it, to full; faded out under [`GUST_FADE`].
fn thickness(gust: f64) -> f64 {
    (MARS_STRENGTH * gust / MARS_GUST_RATIO).clamp(0.0, 1.0) * smoothstep(GUST_FADE.0, GUST_FADE.1, gust)
}

/// The dust haze's vertical optical depth where the dust is at full strength on a ground of loose
/// sand (`x531`, the shader's `dustHaze`); in proportion to the dust's thickness elsewhere, so 0.035
/// on Mars and none where no gust reaches the threshold.
///
/// It is far thinner than the one measured haze there is. Mars's column is 0.4 to 1.25 at Gale, and
/// the dust along the ground was measured there from the crater's rim through the air: an extinction
/// of 0.05 to 0.11 per km in ordinary seasons (Moores et al. 2015, Moore et al. 2016, as Smith et al.
/// 2019 sum them up, arXiv 1910.00986: "average peak values of 0.11 km-1"), the rim 30 km off seen
/// at most times of the year; over 1.1 per km in the storm of 2018, when less than 3 km could be
/// seen. This value gives Mars 0.035 / 11.1 km = 0.003 per km: a fifteenth to a thirty-fifth of the
/// measured. Mars's own would take 1.1 to 3.6 here.
///
/// Picked first on replays at 0.02, 0.05 and 0.1 (`ed-captures/dust-haze`, 2026-11-12), while the
/// shader drew too little of it. With its columns right (client 0.43.199, 0.43.200) it lies over the
/// whole view from 8 km up in a basin on Plooe Thoi EP-R d4-9 C 4 and hides the basin's far wall on
/// foot (`ed-captures/crater`). For a day (client 0.43.201) it was a third of this, for the looks;
/// the owner's rule is the real thing before the looks (2026-11-17), and of the strengths tried the
/// thickest is the nearest to the measured, so it is 0.1 again.
const DUST_HAZE: f64 = 0.1;

/// The gust ratio from which the sand moves all the time: no fronts. Under 3 by DUST-PLAN.md's first
/// figure; at 2.5 so that 7 c (2.68), where the dust was tuned in game as a steady one, stays so (the
/// owner's choice, 2026-11-12).
const STEADY_GUST_RATIO: f64 = 2.5;

/// The share of the ground a dust front lies over (`z519`, the shader's `frontAt`), and so about the
/// share of the time there is dust at a place: 0.05 where the gust only just reaches the threshold,
/// 0.19 on Mars, 1 from [`STEADY_GUST_RATIO`] on. A starting value to tune in game: measured shares are
/// lower still (Earth: 26 %, 6 % and 0.33 % of three hours as the wind fell; InSight's winds are over
/// 15 m/s 0.4 to 1.4 % of the time).
fn activity(gust: f64) -> f64 {
    ((gust - GUST_FADE.0) / (STEADY_GUST_RATIO - GUST_FADE.0)).clamp(0.05, 1.0)
}

/// The wind near the ground at night over the day's: 0.19 to 0.37 on Mars (InSight, Jezero).
const NIGHT_WIND: f64 = 0.3;

/// The night's dust over the day's (`w519`): the thickness at the night's wind over the day's. 0.001
/// on Mars (none: the slot's 0 means "as by day"), 0.39 on 7 c.
fn night_share(gust: f64) -> f64 {
    (thickness(NIGHT_WIND * gust) / thickness(gust).max(1e-9)).clamp(0.001, 1.0)
}

/// 0 under `from`, 1 over `to`, smooth between.
fn smoothstep(from: f64, to: f64, x: f64) -> f64 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The friction speed (m/s) at which the wind starts to move the grains that move most easily:
/// `0.157 (γ ρ_grain g)^(1/4) / sqrt(ρ_air)`. The reduction of Shao and Lu's threshold at its minimum
/// over the grain size (the report's own, not a published formula); it gives Earth 0.2 and Mars
/// 1.5 m/s as Kok et al. 2012 (Table 2.3) do, Titan 0.05 and Venus 0.035 against their 0.04 and 0.02.
fn start_threshold(air_density: f64, gravity: f64, grain_density: f64, cohesion: f64) -> f64 {
    0.157 * (cohesion * grain_density * gravity).powf(0.25) / air_density.sqrt()
}

/// The wind-blown dust of a scanned body at `strength` (the player's setting, 0–1 or more).
/// None without the radius, or without air or gravity.
pub fn for_body(body: &BodyScan, strength: f32) -> Dust {
    if body.radius <= 0.0 {
        return Dust::NONE;
    }
    let ground = ground_type(body);
    let still = Dust { radius_m: body.radius, colour: colour_of(body), loose: grain_supply(ground), ..Dust::NONE };
    let Some(exo) = body.exo.as_ref().filter(|e| e.surface_pressure >= MIN_PRESSURE_PA) else { return still };
    if body.surface_gravity <= 0.0 || strength <= 0.0 {
        return still;
    }
    let base = exo.atmosphere_type.strip_suffix("Rich").unwrap_or(&exo.atmosphere_type);
    let gases: Vec<(&str, f32)> = if exo.composition.is_empty() {
        vec![(base, 100.0)]
    } else {
        exo.composition.iter().map(|g| (g.name.as_str(), g.percent)).collect()
    };
    let density = air_density(body, exo.surface_pressure, &gases);
    let gravity = f64::from(body.surface_gravity);
    let ice = ice_share(body);
    let threshold = start_threshold(density, gravity, grain_density(ice), cohesion(ice, f64::from(body.surface_temperature)));
    let gust = GUST_FRICTION / threshold;
    let wind = (threshold / FRICTION_PER_WIND).clamp(WIND_RANGE.0, WIND_RANGE.1);
    let thick = thickness(gust) * f64::from(still.loose);
    let strength = (thick * f64::from(strength)) as f32;
    // What would be written as 0.000 is none: `x515 = 0` alone, the rest stays as it was.
    if strength < 0.0005 {
        return still;
    }
    // The air's scale height, R T / (M g): the fine dust is mixed through the air as the air itself thins.
    let scale_m = 8.314 * f64::from(body.surface_temperature.max(1.0)) / (mean_molar_mass(&gases) * gravity);
    Dust {
        strength,
        wind: wind as f32,
        sheet_m: sheet_depth(gravity) as f32,
        activity: activity(gust) as f32,
        night: night_share(gust) as f32,
        haze: (DUST_HAZE * f64::from(strength)) as f32,
        scale_m: scale_m as f32,
        ..still
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal_reader::{AtmosphereGas, ExoScan};

    fn body(class: &str, gas: &str, pressure: f32, kelvin: f32, gravity_ms2: f32) -> BodyScan {
        BodyScan {
            body_name: "Test 1".to_string(),
            planet_class: class.to_string(),
            radius: 1_325_241.0,
            surface_temperature: kelvin,
            surface_gravity: gravity_ms2,
            exo: Some(ExoScan {
                atmosphere_type: gas.to_string(),
                so2_percent: 0.0,
                surface_pressure: pressure,
                composition: vec![AtmosphereGas { name: gas.to_string(), percent: 100.0 }],
            }),
            ..Default::default()
        }
    }

    #[test]
    fn airless_bodies_have_no_wind_blown_dust() {
        let mut b = body("Rocky body", "CarbonDioxide", 0.0, 180.0, 1.5);
        let d = for_body(&b, 1.0);
        assert_eq!((d.strength, d.radius_m), (0.0, 1_325_241.0));
        b.exo = None;
        assert_eq!(for_body(&b, 1.0).strength, 0.0);
        let none = vec![("x515", "0.000".to_string()), ("x531", "0.0000".to_string())];
        assert_eq!(d.assignments(), none);
        assert_eq!(Dust::NONE.assignments(), none);
    }

    #[test]
    fn loose_grain_is_most_on_sand_and_less_on_firmer_ground() {
        let loose = |class: &str, pressure: f32| for_body(&body(class, "CarbonDioxide", pressure, 180.0, 1.5), 1.0).loose;
        let (sand, regolith, frost, ice, rocky_ice) =
            (loose("Rocky body", 610.0), loose("Rocky body", 0.0), loose("Icy body", 610.0), loose("Icy body", 0.0), loose("Rocky ice body", 0.0));
        assert_eq!(sand, 1.0);
        assert!(sand > regolith && regolith == frost && frost > rocky_ice && rocky_ice > ice && ice > 0.0, "{sand} {regolith} {frost} {rocky_ice} {ice}");
    }

    #[test]
    fn mars_gets_the_mars_strength_and_wind() {
        // Mars: 610 Pa of CO2 at 210 K, 3.71 m/s².
        let d = for_body(&body("Rocky body", "CarbonDioxide", 610.0, 210.0, 3.71), 1.0);
        assert!((d.strength - 0.35).abs() < 0.05, "{d:?}");
        assert!((d.wind - 20.0).abs() < 3.0, "{d:?}");
    }

    #[test]
    fn denser_air_and_lower_gravity_lift_more_and_blow_slower() {
        // 7 c in Pro Thaa RR-W d1-67: 4.7 kPa CO2, 182 K, 0.16 g, where the ripples were tuned.
        let d = for_body(&body("Rocky body", "CarbonDioxide", 4687.5, 182.1, 1.5457), 1.0);
        assert_eq!(d.strength, 1.0, "{d:?}");
        assert_eq!(d.wind, 10.0, "{d:?}");
        assert_eq!(d.radius_m, 1_325_241.0);
    }

    #[test]
    fn ice_gets_white_dust() {
        let d = for_body(&body("Icy body", "Nitrogen", 2000.0, 60.0, 0.6), 1.0);
        assert!(d.strength > 0.0);
        assert!(d.colour[2] > d.colour[0], "{d:?}");
    }

    #[test]
    fn ice_grains_lift_more_easily_than_rock() {
        // The same air and gravity: an icy body's grains (0.85 ice) weigh about 45 % of rock's, and
        // the threshold goes with the fourth root of that; apart from how much loose grain each
        // ground has (frost against sand). At 1500 Pa: over the fade, under full strength.
        let rock = for_body(&body("Rocky body", "Nitrogen", 1500.0, 150.0, 2.0), 1.0);
        let ice = for_body(&body("Icy body", "Nitrogen", 1500.0, 150.0, 2.0), 1.0);
        let ratio = (ice.strength / ice.loose) / (rock.strength / rock.loose);
        assert!((1.15..1.30).contains(&ratio), "{ratio}: {rock:?} {ice:?}");
    }

    /// The air's density of CO2 at `pressure` Pa and 210 K, kg/m³.
    fn co2_210(pressure: f64) -> f64 {
        pressure * 0.04401 / (8.314 * 210.0)
    }

    #[test]
    fn the_start_threshold_of_the_reports_table() {
        // DUST-PLAN.md phase 1: Mars 1.66 m/s (S 0.72), 7 c 0.45 (2.7), CO2 at Mars's gravity and
        // 210 K: 420 Pa 2.0 (0.60), 250 Pa 2.6 (0.46), 100 Pa 4.1 (0.29).
        let near = |a: f64, b: f64| (a - b).abs() < 0.01 * b.abs().max(1.0);
        let mars = start_threshold(co2_210(610.0), 3.71, ROCK_GRAIN, COHESION);
        assert!(near(mars, 1.66) && near(GUST_FRICTION / mars, MARS_GUST_RATIO), "{mars}");
        let seven_c = start_threshold(4687.5 * 0.04401 / (8.314 * 182.1), 1.5457, ROCK_GRAIN, COHESION);
        assert!(near(seven_c, 0.448) && near(GUST_FRICTION / seven_c, 2.68), "{seven_c}");
        for (pressure, threshold, gust) in [(420.0, 2.00, 0.600), (250.0, 2.59, 0.463), (100.0, 4.10, 0.293)] {
            let t = start_threshold(co2_210(pressure), 3.71, ROCK_GRAIN, COHESION);
            assert!(near(t, threshold) && near(GUST_FRICTION / t, gust), "{pressure} Pa: {t}");
        }
    }

    #[test]
    fn warm_ice_is_stickier_than_rock_and_cold_ice_is_not() {
        // DUST-PLAN.md phase 2: pure ice against rock in the same air and gravity.
        let rock = start_threshold(0.02, 2.0, grain_density(0.0), cohesion(0.0, 220.0));
        let cold = start_threshold(0.02, 2.0, grain_density(1.0), cohesion(1.0, 150.0));
        let warm = start_threshold(0.02, 2.0, grain_density(1.0), cohesion(1.0, 220.0));
        assert!((cold / rock - 0.77).abs() < 0.005, "{}", cold / rock);
        assert!((warm / rock - 1.37).abs() < 0.005, "{}", warm / rock);
        // Rock is the same at any temperature, and ice changes only between 170 and 200 K.
        assert_eq!(cohesion(0.0, 150.0), cohesion(0.0, 300.0));
        assert_eq!(cohesion(1.0, 100.0), cohesion(1.0, 170.0));
        assert_eq!(cohesion(1.0, 200.0), cohesion(1.0, 300.0));
        // So an icy body's dust thins as it warms, in the same air's density.
        let at = |kelvin: f32, pressure: f32| for_body(&body("Icy body", "Nitrogen", pressure, kelvin, 2.0), 1.0).strength;
        assert!(at(150.0, 1500.0) > at(220.0, 2200.0), "{} {}", at(150.0, 1500.0), at(220.0, 2200.0));
    }

    #[test]
    fn the_sheet_is_as_deep_as_the_gravity_is_low() {
        // DUST-PLAN.md phase 3: 1 g 0.10 m, Mars 0.26, 7 c 0.63, a tenth of a g 1.0, 0.03 g the cap.
        for (gravity, metres) in [(9.81, 0.10), (3.71, 0.26), (1.5457, 0.63), (0.981, 1.0), (0.2943, 2.0), (40.0, 0.05)] {
            assert!((sheet_depth(gravity) - metres).abs() < 0.006, "{gravity}: {}", sheet_depth(gravity));
        }
        let d = for_body(&body("Rocky body", "CarbonDioxide", 610.0, 210.0, 3.71), 1.0);
        assert!(d.assignments().contains(&("y519", "0.26".to_string())), "{:?}", d.assignments());
    }

    #[test]
    fn thin_worlds_get_fronts_and_calm_nights_dense_ones_neither() {
        // DUST-PLAN.md phases 4 and 5. Mars: a front over a fifth of the ground, no dust at night.
        let mars = for_body(&body("Rocky body", "CarbonDioxide", 610.0, 210.0, 3.71), 1.0);
        assert!((mars.activity - 0.19).abs() < 0.005 && mars.night == 0.001, "{mars:?}");
        // 7 c, tuned in game as a steady dust: no fronts, and 0.39 of it left at night.
        let seven_c = for_body(&body("Rocky body", "CarbonDioxide", 4687.5, 182.1, 1.5457), 1.0);
        assert!(seven_c.activity == 1.0 && (seven_c.night - 0.39).abs() < 0.005, "{seven_c:?}");
        // Neither slot is ever written as 0, which the shader reads as "not set".
        for pressure in [150.0, 250.0, 610.0, 2000.0, 50_000.0] {
            let d = for_body(&body("Rocky body", "CarbonDioxide", pressure, 210.0, 3.71), 1.0);
            assert!((0.05..=1.0).contains(&d.activity) && (0.001..=1.0).contains(&d.night), "{pressure}: {d:?}");
            let lines = d.assignments();
            let get = |name: &str| lines.iter().find(|(n, _)| *n == name).map(|(_, v)| v.parse::<f32>().unwrap());
            assert!(get("z519").unwrap() > 0.0 && get("w519").unwrap() > 0.0, "{lines:?}");
        }
        // Both only grow with the gust.
        assert!(activity(0.3) == 0.05 && activity(1.4) < activity(2.0) && activity(2.5) == 1.0 && activity(9.0) == 1.0);
        assert!(night_share(0.72) == 0.001 && night_share(2.0) < night_share(3.0) && night_share(40.0) == 1.0);
    }

    #[test]
    fn the_haze_goes_with_the_dust_and_the_airs_scale_height() {
        // Mars: a tenth of its dust's strength, the air thinning by e every 10.7 km.
        let mars = for_body(&body("Rocky body", "CarbonDioxide", 610.0, 210.0, 3.71), 1.0);
        assert!((f64::from(mars.haze) - DUST_HAZE * f64::from(mars.strength)).abs() < 1e-6 && (mars.haze - 0.035).abs() < 0.001, "{mars:?}");
        assert!((mars.scale_m - 10_693.0).abs() < 20.0, "{mars:?}");
        // 7 c: at full strength, 22 km.
        let seven_c = for_body(&body("Rocky body", "CarbonDioxide", 4687.5, 182.1, 1.5457), 1.0);
        assert!((f64::from(seven_c.haze) - DUST_HAZE).abs() < 1e-6 && (seven_c.scale_m - 22_255.0).abs() < 40.0, "{seven_c:?}");
        // With the player's setting, and gone with the dust.
        assert!((for_body(&body("Rocky body", "CarbonDioxide", 610.0, 210.0, 3.71), 0.5).haze - 0.5 * mars.haze).abs() < 1e-6);
        assert_eq!(for_body(&body("Rocky body", "CarbonDioxide", 100.0, 210.0, 3.71), 1.0).haze, 0.0);
    }

    #[test]
    fn thin_air_under_the_gusts_reach_has_no_dust() {
        // 100 Pa of CO2 at Mars's gravity: the gust is 0.29 of the threshold, under the fade's end.
        let none = for_body(&body("Rocky body", "CarbonDioxide", 100.0, 210.0, 3.71), 1.0);
        assert_eq!(none.strength, 0.0, "{none:?}");
        assert_eq!(none.assignments(), vec![("x515", "0.000".to_string()), ("x531", "0.0000".to_string())]);
        // 250 Pa: inside the fade.
        let thin = for_body(&body("Rocky body", "CarbonDioxide", 250.0, 210.0, 3.71), 1.0);
        assert!((0.05..0.2).contains(&thin.strength), "{thin:?}");
        // And the strength only grows with the pressure, to full.
        let mut last = 0.0;
        for pressure in [100.0, 150.0, 250.0, 420.0, 610.0, 2000.0, 10_000.0] {
            let d = for_body(&body("Rocky body", "CarbonDioxide", pressure, 210.0, 3.71), 1.0);
            assert!(d.strength >= last, "{pressure}: {d:?}");
            last = d.strength;
        }
        assert_eq!(last, 1.0);
    }

    #[test]
    fn lava_worlds_blow_little_dark_ash() {
        let mut b = body("Rocky body", "SulphurDioxide", 610.0, 210.0, 3.71);
        let plain = for_body(&b, 1.0);
        b.volcanism = "major rocky magma volcanism".to_string();
        let lava = for_body(&b, 1.0);
        assert!((lava.strength / plain.strength - 0.3).abs() < 1e-3, "{plain:?} {lava:?}");
        assert_eq!(lava.colour, LAVA_DUST);
    }

    #[test]
    fn the_players_strength_scales_it_and_zero_is_none() {
        let b = body("Rocky body", "CarbonDioxide", 610.0, 210.0, 3.71);
        assert_eq!(for_body(&b, 0.0).strength, 0.0);
        let half = for_body(&b, 0.5);
        assert!((half.strength - 0.5 * for_body(&b, 1.0).strength).abs() < 1e-6);
    }

    #[test]
    fn the_lines_are_x15_to_x19_without_the_clock() {
        let d = for_body(&body("Rocky body", "CarbonDioxide", 610.0, 210.0, 3.71), 1.0);
        let names: Vec<_> = d.assignments().into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["x515", "y515", "w515", "x516", "y516", "z516", "x519", "y519", "z519", "w519", "x531", "y531"]);
    }
}
