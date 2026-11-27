//! A black hole's accretion disc, for the shader mods (`scripts/ed-shader/edcas_black_hole.hlsl`, README.md
//! "Black holes", 2026-11-25): whether the hole the commander is at has one, its plane, its size and how hot
//! it is, from the journal's scans, written to IniParams rows 551 and 552, with the blackbody table the shader
//! colours it by in rows 560 to 591.
//!
//! A disc only where it would be: a stellar black hole has one when a companion star of its pair fills its Roche
//! lobe (Eggleton 1983, from the masses and the pair's separation at periapsis); a lone hole stays bare. The disc
//! lies in the pair's orbital plane and reaches out to 0.9 of the hole's own Roche lobe (tidal truncation,
//! Paczynski 1977). It is a thin Shakura-Sunyaev disc (1973) from the innermost stable orbit, 3 r_s, with
//! sigma T^4 = 3 G M Mdot / (8 pi R^3) (1 - sqrt(3 r_s / R)); the shader adds the gravitational redshift and the
//! Doppler shift of the orbiting gas (Luminet 1979). How much it accretes the journal does not say: 0.1 of the
//! Eddington luminosity (bright persistent X-ray binaries shine at 0.01 to 1 of it). Sagittarius A*
//! (`SupermassiveBlackHole`) accretes at about 3e-9 of Eddington (Genzel et al. 2010): the same formula makes its
//! flow a dim red-orange glow some thousand kelvin warm, out to 1e5 r_s (about its Bondi radius), optically thin: the
//! shader adds its light over the sky behind it (kind 3). The disc's
//! spin and the hole's (the journal has none) are left out: no jets, the Schwarzschild metric.
//!
//! The orientation is an assumption: Elite's sky (the skybox's cubemap, the frame of the hole's direction the
//! shader gets) is taken to be the galaxy's frame of `StarPos` (x, y to galactic north, z towards the core), and a
//! scan's `OrbitalInclination` and `AscendingNode` to be measured from the galactic plane and its x axis. Neither
//! is checked against the game.

use crate::journal_reader::BodyScan;

/// What the values file carries for the disc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Disc {
    /// The body Elite bends the light round (the black hole or neutron star nearest the commander) is a black hole:
    /// `x553`, the switch of EDCAS's lens. Round a neutron star Elite's constant is not r_s / r (about 4 r_s / r at
    /// Plooe Thoi EP-R d4-9 A, README-ELITE.md "Black holes"), so Elite's own sky and stars stay there.
    pub hole: bool,
    pub kind: Kind,
    /// The temperature scale T_s of `T(R) = T_s (R / r_s)^(-3/4) (1 - sqrt(3 r_s / R))^(1/4)`, kelvin.
    pub t_scale: f32,
    /// The outer edge in Schwarzschild radii.
    pub outer: f32,
    /// The disc's axis in Elite's sky frame (unit), the gas orbiting anticlockwise round it.
    pub normal: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    None,
    /// Fed by a companion that fills its Roche lobe.
    Companion,
    /// Sagittarius A*'s dim flow.
    Supermassive,
}

impl Disc {
    pub const NONE: Disc = Disc { hole: false, kind: Kind::None, t_scale: 0.0, outer: 0.0, normal: [0.0, 1.0, 0.0] };

    /// The values file's lines: `x551` on, `y551` T_s, `z551` the outer edge, `w551` the kind; `x552`..`z552` the
    /// axis; rows 560 to 591 the blackbody table (only while on, it never changes).
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        let hole = ("x553", if self.hole { "1" } else { "0" }.to_string());
        if self.kind == Kind::None {
            return vec![("x551", "0".into()), hole];
        }
        let kind = match self.kind {
            Kind::None => 0,
            Kind::Companion => 1,
            Kind::Supermassive => 3,
        };
        let [nx, ny, nz] = self.normal;
        let mut out = vec![
            ("x551", "1".to_string()),
            ("y551", format!("{:.1}", self.t_scale)),
            ("z551", format!("{:.1}", self.outer)),
            ("w551", kind.to_string()),
            ("x552", format!("{nx:.5}")),
            ("y552", format!("{ny:.5}")),
            ("z552", format!("{nz:.5}")),
        ];
        out.push(hole);
        for (name, v) in TABLE_NAMES.iter().zip(blackbody_table().iter().flatten()) {
            out.push((name.as_str(), format!("{v:.6e}")));
        }
        out
    }
}

/// The first row of the blackbody table and its size: linear sRGB radiance at `TABLE_LOG_T0 + i * TABLE_STEP`
/// (log10 kelvin), in units of the Sun's surface luminance (5778 K: luminance 1).
pub const TABLE_ROW: u32 = 560;
pub const TABLE_ROWS: usize = 32;
pub const TABLE_LOG_T0: f64 = 3.0;
pub const TABLE_STEP: f64 = 5.0 / 31.0;
/// The table's IniParams names, `x560`, `y560`, `z560`, `x561` ...
static TABLE_NAMES: std::sync::LazyLock<Vec<String>> =
    std::sync::LazyLock::new(|| (0..TABLE_ROWS as u32).flat_map(|i| ['x', 'y', 'z'].map(move |c| format!("{c}{}", TABLE_ROW + i))).collect());

const G: f64 = 6.674_30e-11;
const C: f64 = 299_792_458.0;
const SIGMA: f64 = 5.670_374e-8;
const SUN_KG: f64 = 1.988_47e30;
/// The Eddington luminosity per solar mass (ionised hydrogen, 4 pi G M m_p c / sigma_T), watts.
const EDDINGTON_PER_SUN: f64 = 1.257e31;
/// The accretion efficiency down to the innermost stable orbit of a Schwarzschild hole, 1 - sqrt(8/9).
const EFFICIENCY: f64 = 0.057_191;
/// How much of the Eddington luminosity a fed stellar hole's disc gives (an assumption, the module's doc).
const FED: f64 = 0.1;
/// Sagittarius A*'s share of it.
const SGR_A: f64 = 3e-9;
/// The outer edge of Sagittarius A*'s flow, r_s.
const SGR_A_OUTER: f64 = 1e5;

/// A black hole's class in the journal.
pub fn is_black_hole(b: &BodyScan) -> bool {
    matches!(b.star_type.as_str(), "H" | "SupermassiveBlackHole")
}

/// A body Elite bends the light round with its lensing skybox: a black hole or a neutron star.
fn bends_light(b: &BodyScan) -> bool {
    is_black_hole(b) || b.star_type == "N"
}

/// Eggleton's (1983) Roche lobe radius over the separation, for the body of mass ratio `q` (its mass over the
/// other's): 0.49 q^(2/3) / (0.6 q^(2/3) + ln(1 + q^(1/3))), within 1 % for every q.
pub fn roche_lobe(q: f64) -> f64 {
    let q3 = q.cbrt();
    0.49 * q3 * q3 / (0.6 * q3 * q3 + (1.0 + q3).ln())
}

/// The temperature scale T_s of a thin disc round `solar_masses` accreting at `eddington` of the Eddington
/// luminosity, kelvin: (3 G M Mdot / (8 pi sigma r_s^3))^(1/4).
pub fn t_scale(solar_masses: f64, eddington: f64) -> f64 {
    let m = solar_masses * SUN_KG;
    let r_s = 2.0 * G * m / (C * C);
    let mdot = eddington * EDDINGTON_PER_SUN * solar_masses / (EFFICIENCY * C * C);
    (3.0 * G * m * mdot / (8.0 * std::f64::consts::PI * SIGMA * r_s.powi(3))).powf(0.25)
}

/// The disc's temperature at `r` Schwarzschild radii (0 inside the innermost stable orbit): the shader's formula, for
/// the tests.
#[cfg(test)]
pub fn temperature(t_scale: f64, r: f64) -> f64 {
    if r <= 3.0 {
        return 0.0;
    }
    t_scale * r.powf(-0.75) * (1.0 - (3.0 / r).sqrt()).powf(0.25)
}

/// The disc at the black hole the commander is at: `near` when it is one, else the system's black hole or neutron
/// star closest to it by the distance from the arrival point (the journal's only measure of where bodies are), when
/// that is a black hole. [`Disc::NONE`] (and `hole` false)
/// without one.
pub fn at(near: Option<&BodyScan>, lists: &[&[BodyScan]]) -> Disc {
    let bodies: Vec<&BodyScan> = lists.iter().flat_map(|l| l.iter()).collect();
    let here = near.map_or(0.0, |b| b.distance_from_arrival_ls);
    let lens = match near.filter(|b| bends_light(b)) {
        Some(b) => b,
        None => match bodies.iter().copied().filter(|b| bends_light(b)).min_by(|a, b| {
            (a.distance_from_arrival_ls - here).abs().total_cmp(&(b.distance_from_arrival_ls - here).abs())
        }) {
            Some(b) => b,
            None => return Disc::NONE,
        },
    };
    if !is_black_hole(lens) {
        return Disc::NONE;
    }
    Disc { hole: true, ..of_hole(lens, &bodies) }
}

/// The disc of `hole`, among the system's `bodies`.
pub fn of_hole(hole: &BodyScan, bodies: &[&BodyScan]) -> Disc {
    let mass = hole.orbit.map_or(0.0, |o| f64::from(o.stellar_mass));
    if hole.star_type == "SupermassiveBlackHole" {
        let mass = if mass > 0.0 { mass } else { 4.3e6 };
        return Disc { hole: true, kind: Kind::Supermassive, t_scale: t_scale(mass, SGR_A) as f32, outer: SGR_A_OUTER as f32, normal: [0.0, 1.0, 0.0] };
    }
    if mass <= 0.0 {
        return Disc::NONE;
    }
    fed(hole, mass, bodies).unwrap_or(Disc::NONE)
}

fn r_s_metres(solar_masses: f64) -> f64 {
    2.0 * G * solar_masses * SUN_KG / (C * C)
}

/// The disc fed by a companion of `hole`'s pair that fills its Roche lobe at periapsis, if there is one.
fn fed(hole: &BodyScan, mass: f64, bodies: &[&BodyScan]) -> Option<Disc> {
    let pair = hole.parents.first()?.body_id;
    let h = hole.orbit?;
    for star in bodies.iter().filter(|b| b.body_id != hole.body_id && !is_black_hole(b) && !b.star_type.is_empty()) {
        if star.parents.first().map(|p| p.body_id) != Some(pair) || star.radius <= 0.0 {
            continue;
        }
        let Some(o) = star.orbit.filter(|o| o.stellar_mass > 0.0) else { continue };
        let m2 = f64::from(o.stellar_mass);
        // Each orbits the pair's barycentre: the separation is the sum of the two semi-major axes (or one of them
        // scaled by the masses when the other is missing).
        let (a1, a2) = (f64::from(h.semi_major_axis), f64::from(o.semi_major_axis));
        let a = match (a1 > 0.0, a2 > 0.0) {
            (true, true) => a1 + a2,
            (true, false) => a1 * (1.0 + mass / m2),
            (false, true) => a2 * (1.0 + m2 / mass),
            (false, false) => continue,
        };
        let periapsis = a * (1.0 - f64::from(o.eccentricity.max(h.eccentricity)).clamp(0.0, 0.99));
        if f64::from(star.radius) < periapsis * roche_lobe(m2 / mass) {
            continue;
        }
        let outer = 0.9 * periapsis * roche_lobe(mass / m2) / r_s_metres(mass);
        return Some(Disc { hole: true, kind: Kind::Companion, t_scale: t_scale(mass, FED) as f32, outer: outer as f32, normal: orbit_normal(o.inclination, o.ascending_node) });
    }
    None
}

/// The axis of an orbit of `inclination` and `node` (degrees) in the galaxy's frame (the module's doc: an
/// assumption): the galactic north tilted by the inclination about the line of nodes at `node` from x.
pub fn orbit_normal(inclination: f32, node: f32) -> [f32; 3] {
    let (i, n) = (inclination.to_radians(), node.to_radians());
    [i.sin() * n.sin(), i.cos(), -i.sin() * n.cos()]
}

/// The blackbody table: linear sRGB radiance per [`TABLE_ROWS`] temperatures, the Sun's surface luminance 1.
pub fn blackbody_table() -> [[f64; 3]; TABLE_ROWS] {
    let sun = crate::star_light::blackbody_xyz_unclamped(5778.0)[1];
    let mut table = [[0.0; 3]; TABLE_ROWS];
    for (i, row) in table.iter_mut().enumerate() {
        let t = 10f64.powf(TABLE_LOG_T0 + i as f64 * TABLE_STEP);
        let [x, y, z] = crate::star_light::blackbody_xyz_unclamped(t);
        let rgb = [3.2406 * x - 1.5372 * y - 0.4986 * z, -0.9689 * x + 1.8758 * y + 0.0415 * z, 0.0557 * x - 0.2040 * y + 1.0570 * z];
        *row = rgb.map(|c| c.max(0.0) / sun);
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal_reader::{BodyParent, OrbitScan};

    fn body(id: i32, star_type: &str, mass: f32, radius_m: f32, parent: i32, a_m: f32) -> BodyScan {
        BodyScan {
            body_id: id,
            star_type: star_type.into(),
            radius: radius_m,
            parents: vec![BodyParent { body_id: parent }],
            orbit: Some(OrbitScan { stellar_mass: mass, semi_major_axis: a_m, eccentricity: 0.0, inclination: 30.0, ascending_node: 0.0 }),
            ..BodyScan::default()
        }
    }

    #[test]
    fn eggleton_matches_the_known_values() {
        // q = 1: 0.3789 (Eggleton 1983); small q: Paczynski's 0.462 (q / (1 + q))^(1/3) within 2 %.
        assert!((roche_lobe(1.0) - 0.3789).abs() < 1e-4);
        for q in [0.05f64, 0.1, 0.3] {
            let p = 0.462 * (q / (1.0 + q)).cbrt();
            assert!((roche_lobe(q) - p).abs() / p < 0.02, "{q}: {} against {p}", roche_lobe(q));
        }
        // the two lobes never overlap
        for q in [0.01f64, 0.1, 1.0, 10.0, 100.0] {
            assert!(roche_lobe(q) + roche_lobe(1.0 / q) < 1.0);
        }
    }

    #[test]
    fn a_stellar_disc_is_millions_of_kelvin_inside_and_ten_thousand_far_out() {
        let ts = t_scale(4.777, FED);
        let peak = (40..200).map(|i| temperature(ts, f64::from(i) / 10.0)).fold(0.0, f64::max);
        assert!(peak > 3e6 && peak < 3e7, "{peak}");
        // the outer disc near 10^4 K at some 1e5 r_s
        let far = temperature(ts, 1e5);
        assert!(far > 3e3 && far < 3e4, "{far}");
        assert_eq!(temperature(ts, 2.9), 0.0);
    }

    #[test]
    fn a_lone_hole_is_bare() {
        // Outords LM-W f1-3: 4.777 solar masses, alone.
        let hole = body(0, "H", 4.777, 14093.0, -1, 0.0);
        let hole = BodyScan { parents: vec![], ..hole };
        let bare = at(Some(&hole), &[std::slice::from_ref(&hole)]);
        assert_eq!(bare, Disc { hole: true, ..Disc::NONE });
        assert_eq!(bare.assignments(), vec![("x551", "0".to_string()), ("x553", "1".to_string())]);
        // a neutron star nearer the commander: Elite's own lens there
        let ns = BodyScan { body_id: 5, star_type: "N".into(), distance_from_arrival_ls: 100.0, ..BodyScan::default() };
        let far = BodyScan { distance_from_arrival_ls: 5000.0, ..hole.clone() };
        assert_eq!(at(None, &[&[far.clone(), ns.clone()]]), Disc::NONE);
        assert!(at(Some(&far), &[&[far.clone(), ns]]).hole);
    }

    #[test]
    fn a_companion_that_fills_its_roche_lobe_feeds_a_disc_in_its_orbital_plane() {
        // A 7 solar mass hole and a 1 solar mass star 4e9 m apart: the star's lobe is 0.25 a = 1e9 m.
        let hole = body(1, "H", 7.0, 20_000.0, 0, 0.5e9);
        let full = body(2, "K", 1.0, 1.1e9, 0, 3.5e9);
        let small = body(2, "K", 1.0, 0.7e9, 0, 3.5e9);
        let lists = [hole.clone(), full.clone()];
        let d = at(Some(&hole), &[&lists]);
        assert_eq!(d.kind, Kind::Companion);
        let n = orbit_normal(30.0, 0.0);
        assert_eq!(d.normal, n);
        assert!((n[1] - 30f32.to_radians().cos()).abs() < 1e-6);
        // outer edge 0.9 of the hole's lobe (q = 7: 0.53 of 4e9 m) in r_s of 7 suns (20.7 km)
        let want = 0.9 * 4e9 * roche_lobe(7.0) / r_s_metres(7.0);
        assert!((f64::from(d.outer) - want).abs() / want < 1e-3);
        assert_eq!(at(Some(&hole), &[&[hole.clone(), small]]), Disc { hole: true, ..Disc::NONE });
        // a star of another pair does not feed it
        let elsewhere = BodyScan { parents: vec![BodyParent { body_id: 9 }], ..full };
        assert_eq!(at(Some(&hole), &[&[hole.clone(), elsewhere]]), Disc { hole: true, ..Disc::NONE });
    }

    #[test]
    fn sagittarius_a_glows_dim_and_red() {
        let sgr = BodyScan { star_type: "SupermassiveBlackHole".into(), ..BodyScan::default() };
        let d = at(Some(&sgr), &[std::slice::from_ref(&sgr)]);
        assert_eq!(d.kind, Kind::Supermassive);
        let peak = (40..200).map(|i| temperature(f64::from(d.t_scale), f64::from(i) / 10.0)).fold(0.0, f64::max);
        assert!(peak > 1500.0 && peak < 6000.0, "{peak}");
    }

    #[test]
    fn the_table_is_the_sun_at_one_and_blue_white_when_hot() {
        let t = blackbody_table();
        let lum = |c: [f64; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        // brighter with every step, the Sun's 5778 K between its neighbours at luminance 1
        assert!(t.windows(2).all(|w| lum(w[1]) > lum(w[0])));
        let i = ((5778f64.log10() - TABLE_LOG_T0) / TABLE_STEP).floor() as usize;
        assert!(lum(t[i]) < 1.0 && lum(t[i + 1]) > 1.0, "{} {}", lum(t[i]), lum(t[i + 1]));
        // far into Rayleigh-Jeans the radiance grows as T: the last step is 10^(5/31) = 1.45 times
        let r = lum(t[TABLE_ROWS - 1]) / lum(t[TABLE_ROWS - 2]);
        assert!((r - 10f64.powf(TABLE_STEP)).abs() < 0.01, "{r}");
        let hot = t[TABLE_ROWS - 1];
        assert!(hot[2] > hot[0], "{hot:?}");
        assert!(t[0][0] > t[0][2], "{:?}", t[0]);
        assert_eq!(Disc::NONE.assignments(), vec![("x551", "0".to_string()), ("x553", "0".to_string())]);
        let on = Disc { hole: true, kind: Kind::Companion, t_scale: 1e8, outer: 9e4, normal: [0.0, 1.0, 0.0] }.assignments();
        assert_eq!(on.len(), 8 + 3 * TABLE_ROWS);
        assert_eq!(on.last().unwrap().0, format!("z{}", TABLE_ROW + TABLE_ROWS as u32 - 1).as_str());
    }
}
