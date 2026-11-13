//! The wind-blown dust a planet's air can lift (`scripts/ed-shader/edcas_dust.hlsl`): sand
//! streaming in sheets along the ground, and billows in gusts.
//! Mars's dust devils are left out: the shader can't light or shade towers that tall.
//!
//! How easily the wind lifts sand goes with the threshold friction speed, which grows as
//! `sqrt(g / ρ_air)` for a given grain (Bagnold; Kok 2010 for Mars): denser air and lower
//! gravity lift more. So the strength is the planet's `sqrt(ρ / g)` against Mars's, where the
//! wind does lift dust but rarely. The wind itself is slower in denser air for the same push:
//! `1 / sqrt(ρ)`, from Mars's typical 6 m/s.
//!
//! The threshold also grows with the grain's own density, `sqrt(ρ_grain g / ρ_air)`: ice grains
//! (0.93 g/cm³) lift about 1.7 times as easily as rock (2.65), so icy worlds with air blow white
//! (Pluto's dunes of methane-ice grains, Triton's wind-blown plume streaks). How much loose grain
//! there is goes by the ground ([`GroundType`]): sand and frost are covered in it, rocky ice less,
//! fresh lava crust has little, its dark ash only in patches.
//!
//! Written as `x515` (strength), `y515` (wind, m/s), `w515` (radius, m), `x516` to `z516` (colour), `x519` (the far drift's reach, m) of
//! `ShaderFixes/edcas_params.txt`; `z515` is the shader's clock, set by `edcas.ini`.

use crate::ground_tint::{ground_type, makeup, GroundType, Makeup};
use crate::journal_reader::BodyScan;

/// Below this the scan's "atmosphere" is too thin for the wind to move anything (the journals
/// list no atmosphere under it, 2026-10-29).
const MIN_PRESSURE_PA: f32 = 100.0;
/// Mars's air near the ground (kg/m³) and gravity (m/s²), the reference.
const MARS_DENSITY: f64 = 0.020;
const MARS_GRAVITY: f64 = 3.71;
/// Grain densities (kg/m³): Mars's sand (quartz-like, the reference) and water ice.
const ROCK_GRAIN: f64 = 2650.0;
const ICE_GRAIN: f64 = 930.0;
/// The strength on a Mars-like planet: dust moves, but is seldom thick.
const MARS_STRENGTH: f64 = 0.35;
/// The wind (m/s) when the sand moves on Mars (the threshold is about 13 m/s at 2 m; Kok 2010),
/// and the range the shader is given. Where the grains lift more easily (`lift`) they move at
/// less. The floor is for the eye: at 3 m/s the streaks looked still, at 10 they moved
/// (7 c in Pro Thaa RR-W d1-67, 2026-10-29).
const MARS_WIND: f64 = 20.0;
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
    /// How much loose grain the ground has, 0–1 ([`grain_supply`]; 0.5 for an unknown ground),
    /// in the strength. Not written itself.
    pub loose: f32,
}

impl Dust {
    pub const NONE: Dust = Dust { strength: 0.0, wind: 0.0, radius_m: 0.0, colour: [0.0; 3], reach_m: DEFAULT_REACH_M, loose: 0.5 };

    /// The values file's lines. Without dust that is `x515 = 0` alone: the rest stays as it was,
    /// so 3DMigoto's fade only takes the strength down.
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        if self.strength <= 0.0 {
            return vec![("x515", "0.000".to_string())];
        }
        vec![
            ("x515", format!("{:.3}", self.strength)),
            ("y515", format!("{:.1}", self.wind)),
            ("w515", format!("{:.0}", self.radius_m)),
            ("x516", format!("{:.3}", self.colour[0])),
            ("y516", format!("{:.3}", self.colour[1])),
            ("z516", format!("{:.3}", self.colour[2])),
            ("x519", format!("{:.0}", self.reach_m)),
        ]
    }
}

/// Molar mass (kg/mol) of the game's gases; others count as 30 g/mol.
fn molar_mass(gas: &str) -> f64 {
    match gas {
        "CarbonDioxide" => 0.044,
        "SulphurDioxide" => 0.064,
        "Nitrogen" => 0.028,
        "Oxygen" => 0.032,
        "Argon" => 0.040,
        "Neon" => 0.020,
        "Helium" => 0.004,
        "Hydrogen" => 0.002,
        "Methane" => 0.016,
        "Ammonia" => 0.017,
        "Water" => 0.018,
        _ => 0.030,
    }
}

/// The air's density at the ground, kg/m³ (ideal gas).
fn air_density(body: &BodyScan, pressure_pa: f32, gases: &[(&str, f32)]) -> f64 {
    let total: f64 = gases.iter().map(|(_, p)| f64::from(*p)).sum();
    let mass = if total > 0.0 {
        gases.iter().map(|(g, p)| molar_mass(g) * f64::from(*p)).sum::<f64>() / total
    } else {
        0.030
    };
    f64::from(pressure_pa) * mass / (8.314 * f64::from(body.surface_temperature.max(1.0)))
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

/// The grains' density (kg/m³), by the body's ice share: the scan's metal is the whole
/// planet's, mostly in its core, so the grains at the surface are rock or ice.
fn grain_density(body: &BodyScan) -> f64 {
    let m = makeup_or_rock(body);
    let ice = f64::from(m.ice / (m.ice + m.rock + m.metal).max(1e-6));
    ice * ICE_GRAIN + (1.0 - ice) * ROCK_GRAIN
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
    let lift = (density / (gravity * grain_density(body))).sqrt() / (MARS_DENSITY / (MARS_GRAVITY * ROCK_GRAIN)).sqrt();
    let wind = (MARS_WIND / lift).clamp(WIND_RANGE.0, WIND_RANGE.1);
    let thick = (MARS_STRENGTH * lift).clamp(0.15, 1.0) * f64::from(still.loose);
    Dust { strength: (thick * f64::from(strength)) as f32, wind: wind as f32, ..still }
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
        let none = vec![("x515", "0.000".to_string())];
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
        // The same air and gravity: an icy body's grains (0.85 ice) weigh about 40 % of rock's;
        // apart from how much loose grain each ground has (frost against sand).
        let rock = for_body(&body("Rocky body", "Nitrogen", 300.0, 150.0, 2.0), 1.0);
        let ice = for_body(&body("Icy body", "Nitrogen", 300.0, 150.0, 2.0), 1.0);
        let ratio = (ice.strength / ice.loose) / (rock.strength / rock.loose);
        assert!((1.4..1.6).contains(&ratio), "{ratio}: {rock:?} {ice:?}");
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
        assert_eq!(names, ["x515", "y515", "w515", "x516", "y516", "z516", "x519"]);
    }
}
