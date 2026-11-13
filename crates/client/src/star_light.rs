//! The colour of the light a star casts, for the shader mods: Odyssey lights everything with a
//! nearly white sun whatever the star (Frontier tickets 45769 and 85552; measured 2026-11-09 in
//! `scripts/ed-shader/README-ELITE.md`: an M5 at 2703 K gives (1.00, 0.96, 0.93) at point-blank
//! range and the same chroma far out). EDCAS works the colour out from the star's temperature
//! and writes it to IniParams row 526 (`x y z` the colour at luminance 1, `w` the strength);
//! 3DMigoto's copies of Elite's sun pass and sky pass multiply their sun colour by it
//! ([`crate::shader_mods::star_light`]), and EDCAS's own clouds, fog and dust do the same.
//!
//! The colour is Planck's law over the CIE 1931 observer to linear sRGB (`scripts/ed-shader/star_colour.py`
//! is the same sum in Python, for checking by hand; no test keeps the two in step). The shader side,
//! the patched copies of Elite's shaders that read row 526, is [`crate::shader_mods::star_light`]. It is normalised to luminance 1, so the
//! light's brightness stays Elite's: a red dwarf's hull is as bright as before, only red.
//! In a system of several stars the light is their mix, each weighted by how much it shines at
//! the body ([`at`]), together with the sky's own white starlight ([`SKY_LUX`]): stars that give
//! less than the sky does (brown dwarfs beside a black hole) leave Elite's light as it is.

use crate::journal_reader::BodyScan;

/// What the values file carries for the star's light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarLight {
    /// Linear sRGB at luminance 1.
    pub colour: [f32; 3],
    /// 0 = Elite's own light; 1 = the star's colour in full; above 1 the chroma is pushed further.
    pub strength: f32,
    /// The witch-space rainbow (`x527` 1): the shaders blend the tint into a hue turning on their
    /// clock. Written during a hyperspace jump together with the colour of the star jumped to, so
    /// 3DMigoto's fade slides into the rainbow and, at arrival, straight into the new star's light.
    pub rainbow: bool,
}

impl StarLight {
    pub const NONE: StarLight = StarLight { colour: [1.0, 1.0, 1.0], strength: 0.0, rainbow: false };

    /// The values file's lines for it: with the strength 0 only `w526` and `x527`, so the shaders
    /// leave the light alone and the last colour stays in the file for the fade.
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        let rainbow = ("x527", if self.rainbow { "1" } else { "0" }.to_string());
        if self.strength == 0.0 {
            return vec![("w526", "0".to_string()), rainbow];
        }
        let [r, g, b] = self.colour;
        vec![("x526", format!("{r:.3}")), ("y526", format!("{g:.3}")), ("z526", format!("{b:.3}")), ("w526", format!("{:.3}", self.strength)), rainbow]
    }
}

/// The light at `body` (or, with none, at the arrival point) at `strength`: the system's stars
/// mixed by how much each one shines there, its luminosity (radius² × temperature⁴, or a typical
/// one for its class) over the square of its distance, which the journal only gives as each
/// body's distance from the arrival point: the difference of the two, floored at 1 Ls, is the
/// estimate. In a close binary the brighter star wins by its luminosity, a K over an L by
/// thousands (2026-11-09: the ship at Pro Thaa MG-O c6-15 BC was lit in the targeted L's colour
/// with the K beside it); a far-off companion hardly counts. The sky's starlight is in the mix
/// as white ([`with_sky`]). White when no star is known.
pub fn at(body: Option<&BodyScan>, lists: &[&[BodyScan]], strength: f32) -> StarLight {
    if strength <= 0.0 {
        return StarLight::NONE;
    }
    let here = body.map_or(0.0, |b| b.distance_from_arrival_ls);
    let (mut sum, mut weight) = ([0.0f32; 3], 0.0f32);
    for star in stars(lists) {
        let Some(k) = temperature(star) else { continue };
        let d = (star.distance_from_arrival_ls - here).abs().max(1.0);
        let w = luminosity(star, k) / (d * d);
        let c = blackbody(k);
        for i in 0..3 {
            sum[i] += w * c[i];
        }
        weight += w;
    }
    if weight <= 0.0 {
        return StarLight::NONE;
    }
    StarLight { colour: with_sky(sum, weight), strength, rainbow: false }
}

/// The light of a moonless night sky on Earth without the airglow, the stars alone: about
/// 0.0002 lux. In a system whose stars give less than that, what Elite lights the scene with is
/// not theirs, so their colour must not tint it.
const SKY_LUX: f32 = 2e-4;

/// The Sun's light at 1 Ls in lux: 128 000 lux at 1 AU (499.005 Ls), by the inverse square.
/// A weight in [`at`] (solar visible luminosities per Ls²) times this is lux.
const SUN_LUX_AT_1_LS: f32 = 128_000.0 * 499.005 * 499.005;

/// The colour of lights whose weighted colours add to `sum` and weights to `weight` (solar
/// visible luminosities per Ls²), mixed with the sky's white starlight ([`SKY_LUX`]). With a real
/// star about the sky counts for nothing (an M dwarf 5000 Ls away still gives a hundred times
/// its light); brown dwarfs that only glow in the infrared leave the mix white, so Elite's own
/// light stays (2026-11-11, Noijou ZE-A g116: a black hole with two Y dwarfs of 605 K and 454 K
/// lit everything pure red, (4.70, 0, 0), because the dwarfs were the only stars counted and
/// the mix was normalised whatever little they gave). White is at luminance 1 like the stars'
/// colours, so the mix keeps it.
fn with_sky(sum: [f32; 3], weight: f32) -> [f32; 3] {
    let sky = SKY_LUX / SUN_LUX_AT_1_LS;
    sum.map(|x| (x + sky) / (weight + sky))
}

/// The light of a star known by its class alone (the `StartJump`'s `StarClass` of the star being
/// jumped to) at `strength`, as [`at`] gives it at the arrival point (the star 1 Ls away, a
/// typical radius and temperature for the class, the sky in the mix: a Y dwarf's is white);
/// white when the class is unknown.
pub fn of_class(star_type: &str, strength: f32) -> StarLight {
    match class_temperature(star_type) {
        Some(k) if strength > 0.0 => {
            let star = BodyScan { star_type: star_type.into(), ..BodyScan::default() };
            let w = luminosity(&star, k);
            StarLight { colour: with_sky(blackbody(k).map(|c| w * c), w), strength, rainbow: false }
        }
        _ => StarLight::NONE,
    }
}

/// Every star in `lists`, each body once (the journal's and the server's lists overlap).
fn stars<'a>(lists: &[&'a [BodyScan]]) -> Vec<&'a BodyScan> {
    let mut out: Vec<&BodyScan> = Vec::new();
    for b in lists.iter().flat_map(|l| l.iter()).filter(|b| !b.star_type.is_empty()) {
        if !out.iter().any(|s| s.body_id == b.body_id) {
            out.push(b);
        }
    }
    out
}

/// The star's luminosity in the visible, in solar units: radius² × the luminance of a blackbody
/// at its temperature (the CIE Y integral, not the bolometric T⁴: a 650 K brown dwarf radiates
/// in the infrared and casts almost no light to see by), with a typical radius for its class
/// when the scan has none.
fn luminosity(star: &BodyScan, kelvin: f32) -> f32 {
    const SUN_RADIUS_M: f32 = 695_700_000.0;
    let radius = if star.radius > 0.0 { star.radius / SUN_RADIUS_M } else { class_radius(&star.star_type) };
    radius * radius * (visible_luminance(kelvin) / visible_luminance(5778.0)) as f32
}

/// The luminance (CIE Y) a blackbody at `kelvin` radiates per unit area, in arbitrary units.
fn visible_luminance(kelvin: f32) -> f64 {
    blackbody_xyz(kelvin)[1]
}

/// The CIE XYZ of a blackbody at `kelvin` (clamped to 300 K to 200 000 K), in arbitrary units:
/// Planck's law summed over 380 to 780 nm in 5 nm steps against the CIE 1931 2° observer.
fn blackbody_xyz(kelvin: f32) -> [f64; 3] {
    let t = f64::from(kelvin.clamp(300.0, 200_000.0));
    let mut xyz = [0.0f64; 3];
    let mut nm = 380.0f64;
    while nm <= 780.0 {
        let p = planck(nm, t);
        let (xb, yb, zb) = cie_1931(nm);
        xyz[0] += p * xb;
        xyz[1] += p * yb;
        xyz[2] += p * zb;
        nm += 5.0;
    }
    xyz
}

/// A typical radius, in solar radii, for a journal `StarType` without a measured one.
fn class_radius(star_type: &str) -> f32 {
    match star_type.trim().to_ascii_uppercase().as_str() {
        "O" => 10.0,
        "B" | "AEBE" => 5.0,
        "A" => 1.8,
        "F" => 1.3,
        "G" => 1.0,
        "K" => 0.8,
        "M" => 0.4,
        "L" | "T" => 0.1,
        "Y" => 0.08,
        "TTS" => 1.5,
        "W" | "WN" | "WNC" | "WC" | "WO" => 10.0,
        "C" | "CS" | "CN" | "CJ" | "CH" | "CHD" | "MS" | "S" => 100.0,
        "N" => 1e-5,
        t if t.starts_with('D') => 0.013,
        _ => 1.0,
    }
}

/// The star's surface temperature in kelvin: the scan's, else a typical one for its class
/// (the server's bodies and old scans may carry none). `None` for a class that casts no light
/// of its own (a black hole) or an unknown one.
fn temperature(star: &BodyScan) -> Option<f32> {
    if star.surface_temperature > 0.0 {
        return Some(star.surface_temperature);
    }
    class_temperature(&star.star_type)
}

/// A typical surface temperature for a journal `StarType`.
fn class_temperature(star_type: &str) -> Option<f32> {
    let k = match star_type.trim().to_ascii_uppercase().as_str() {
        "O" => 40000.0,
        "B" | "AEBE" => 20000.0,
        "A" => 8500.0,
        "F" => 6500.0,
        "G" => 5700.0,
        "K" => 4500.0,
        "M" => 3200.0,
        "L" => 1800.0,
        "T" => 1100.0,
        "Y" => 500.0,
        "TTS" => 4000.0,
        "W" | "WN" | "WNC" | "WC" | "WO" => 50000.0,
        "C" | "CS" | "CN" | "CJ" | "CH" | "CHD" => 3000.0,
        "MS" | "S" => 3300.0,
        "D" | "DA" | "DAB" | "DAO" | "DAZ" | "DAV" | "DB" | "DBZ" | "DBV" | "DO" | "DOV" | "DQ" | "DC" | "DCV" | "DX" => 15000.0,
        "N" => 40000.0,
        // Black holes ("H", "SupermassiveBlackHole") and exotic ("X") cast no light of their own.
        _ => return None,
    };
    Some(k)
}

/// The colour of a blackbody at `kelvin`, linear sRGB at luminance 1 ([`blackbody_xyz`] through
/// the XYZ to linear sRGB matrix). Negative primaries (deep red and blue ends) are clipped at 0.
fn blackbody(kelvin: f32) -> [f32; 3] {
    let [x, y, z] = blackbody_xyz(kelvin);
    let r = (3.2406 * x - 1.5372 * y - 0.4986 * z).max(0.0);
    let g = (-0.9689 * x + 1.8758 * y + 0.0415 * z).max(0.0);
    let b = (0.0557 * x - 0.2040 * y + 1.0570 * z).max(0.0);
    let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    if lum <= 0.0 {
        return [1.0, 1.0, 1.0];
    }
    [(r / lum) as f32, (g / lum) as f32, (b / lum) as f32]
}

/// Planck's spectral radiance at `nm` nanometres and `t` kelvin (any constant factor cancels
/// in the normalisation).
fn planck(nm: f64, t: f64) -> f64 {
    let lam = nm * 1e-9;
    let c1 = 3.741_771e-16;
    let c2 = 1.438_776e-2;
    c1 / (lam.powi(5) * ((c2 / (lam * t)).exp() - 1.0))
}

/// The CIE 1931 2° colour matching functions at `nm`: the multi-lobe Gaussian fit by Wyman,
/// Sloan and Shirley (2013).
fn cie_1931(nm: f64) -> (f64, f64, f64) {
    let g = |mu: f64, s1: f64, s2: f64| {
        let t = (nm - mu) / if nm < mu { s1 } else { s2 };
        (-0.5 * t * t).exp()
    };
    let x = 1.056 * g(599.8, 37.9, 31.0) + 0.362 * g(442.0, 16.0, 26.7) - 0.065 * g(501.1, 20.4, 26.2);
    let y = 0.821 * g(568.8, 46.9, 40.5) + 0.286 * g(530.9, 16.3, 31.1);
    let z = 1.217 * g(437.0, 11.8, 36.0) + 0.681 * g(459.0, 26.0, 13.8);
    (x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal_reader::BodyParent;

    fn body(id: i32, name: &str, star_type: &str, k: f32, parents: &[i32], arrival: f32) -> BodyScan {
        BodyScan {
            body_id: id,
            body_name: name.into(),
            star_type: star_type.into(),
            surface_temperature: k,
            parents: parents.iter().map(|&body_id| BodyParent { body_id }).collect(),
            distance_from_arrival_ls: arrival,
            ..BodyScan::default()
        }
    }

    #[test]
    fn the_sun_is_nearly_white_and_a_red_dwarf_red() {
        let sol = blackbody(5778.0);
        assert!((sol[0] - 1.11).abs() < 0.02 && (sol[1] - 0.98).abs() < 0.02 && (sol[2] - 0.91).abs() < 0.02, "{sol:?}");
        let m5 = blackbody(2703.0);
        assert!((m5[0] - 1.91).abs() < 0.03 && (m5[1] - 0.81).abs() < 0.02 && (m5[2] - 0.19).abs() < 0.02, "{m5:?}");
        let b = blackbody(20000.0);
        assert!(b[2] > 1.7 && b[0] < 0.8, "{b:?}");
        // Luminance 1 throughout.
        for k in [1000.0, 2703.0, 5778.0, 20000.0, 40000.0] {
            let [r, g, b] = blackbody(k);
            assert!((0.2126 * r + 0.7152 * g + 0.0722 * b - 1.0).abs() < 1e-3, "{k}: {r} {g} {b}");
        }
    }

    #[test]
    fn the_scan_temperature_wins_over_the_class() {
        assert_eq!(temperature(&body(1, "A", "M", 2703.0, &[], 0.0)), Some(2703.0));
        assert_eq!(temperature(&body(1, "A", "M", 0.0, &[], 0.0)), Some(3200.0));
        assert_eq!(temperature(&body(1, "A", "H", 0.0, &[], 0.0)), None);
        assert_eq!(class_temperature("DA"), Some(15000.0));
        assert_eq!(class_temperature(""), None);
    }

    fn star(id: i32, name: &str, star_type: &str, k: f32, radius_sun: f32, arrival: f32) -> BodyScan {
        BodyScan { radius: radius_sun * 695_700_000.0, ..body(id, name, star_type, k, &[], arrival) }
    }

    #[test]
    fn the_brighter_star_of_a_close_pair_lights_the_ship_even_when_the_other_is_targeted() {
        // Pro Thaa MG-O c6-15, 2026-11-09: a G at the arrival point, a K and an L together 285 Ls out, an M 5500 Ls out.
        let a = star(2, "X A", "G", 5214.0, 0.9, 0.0);
        let b = star(4, "X B", "K", 5074.0, 0.8, 285.0);
        let c = star(5, "X C", "L", 1966.0, 0.1, 286.3);
        let d = star(6, "X D", "M", 2979.0, 0.3, 5525.7);
        let bodies = vec![a, b.clone(), c.clone(), d];
        let k = blackbody(5074.0);
        // The L targeted: the K beside it wins.
        let light = at(Some(&c), &[&bodies], 1.0);
        assert!((light.colour[0] - k[0]).abs() < 0.03 && (light.colour[2] - k[2]).abs() < 0.03, "{light:?} vs K {k:?}");
        // At the arrival point the G.
        let g = blackbody(5214.0);
        let light = at(None, &[&bodies], 1.0);
        assert!((light.colour[0] - g[0]).abs() < 0.03, "{light:?} vs G {g:?}");
        // A body round the far M: the M, whatever shines 5000 Ls away.
        let moon = body(9, "X D 1", "", 0.0, &[6, 0], 5530.0);
        let m = blackbody(2979.0);
        let light = at(Some(&moon), &[&bodies], 1.0);
        assert!((light.colour[0] - m[0]).abs() < 0.05, "{light:?} vs M {m:?}");
        // Luminance 1 is kept through the mix.
        let [r, gg, bb] = light.colour;
        assert!((0.2126 * r + 0.7152 * gg + 0.0722 * bb - 1.0).abs() < 1e-3);
        // No star known: white; strength 0: nothing.
        assert_eq!(at(Some(&moon), &[], 1.0), StarLight::NONE);
        assert_eq!(at(Some(&c), &[&bodies], 0.0), StarLight::NONE);
        // The journal's and the server's lists hold the same star: counted once (the mix is the same).
        assert_eq!(at(Some(&c), &[&bodies, &bodies], 1.0), at(Some(&c), &[&bodies], 1.0));
    }

    #[test]
    fn stars_that_give_less_light_than_the_sky_do_not_tint() {
        let white = |c: [f32; 3]| c.iter().all(|x| (x - 1.0).abs() < 0.01);
        let lum = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        // Noijou ZE-A g116, 2026-11-11: a black hole with two Y dwarfs; everything was lit pure red.
        let hole = star(0, "N", "H", 0.0, 3.6e-5, 0.0);
        let y1 = star(7, "N 1", "Y", 605.0, 0.118, 900.0);
        let y2 = star(19, "N 3", "Y", 454.0, 0.0805, 2600.0);
        let bodies = vec![hole, y1.clone(), y2];
        let arrival = at(None, &[&bodies], 1.0);
        assert!(white(arrival.colour), "{arrival:?}");
        assert_eq!(arrival.strength, 1.0);
        // Right at the dwarf too, and a moon of it.
        assert!(white(at(Some(&y1), &[&bodies], 1.0).colour));
        let moon = body(8, "N 1 a", "", 0.0, &[7, 0], 903.0);
        assert!(white(at(Some(&moon), &[&bodies], 1.0).colour));
        // A Y dwarf jumped to: no red tunnel into a white system.
        assert!(white(of_class("Y", 1.0).colour), "{:?}", of_class("Y", 1.0));
        // A real star keeps its colour far out: a dim M (2000 K, 0.15 solar radii) at 5000 Ls,
        // an L dwarf at 300 Ls.
        let m = [star(0, "M", "M", 2000.0, 0.15, 0.0)];
        let far = body(5, "M 5", "", 0.0, &[0], 5000.0);
        let (light, full) = (at(Some(&far), &[&m], 1.0), blackbody(2000.0));
        assert!((light.colour[0] - full[0]).abs() < 0.08 && light.colour[2] < 0.05, "{light:?} vs {full:?}");
        let l = [star(0, "L", "L", 1800.0, 0.1, 0.0)];
        let out = body(3, "L 3", "", 0.0, &[0], 300.0);
        let (light, full) = (at(Some(&out), &[&l], 1.0), blackbody(1800.0));
        assert!((light.colour[0] - full[0]).abs() < 0.03, "{light:?} vs {full:?}");
        let t = [star(0, "T", "T", 1100.0, 0.1, 0.0)];
        let (close, away) = (body(1, "T 1", "", 0.0, &[0], 5.0), body(9, "T 9", "", 0.0, &[0], 1000.0));
        // A T dwarf (1100 K) fades out with distance (red 4.02 at 1 Ls, 3.01 at 100, 1.55 at 300,
        // 1.06 at 1000), at luminance 1 at every step.
        for d in [1.0, 30.0, 100.0, 300.0, 1000.0] {
            let c = at(Some(&body(1, "T 1", "", 0.0, &[0], d)), &[&t], 1.0).colour;
            assert!((lum(c) - 1.0).abs() < 1e-3, "{d} Ls: {c:?}");
        }
        // It lights its close moons red; a thousand Ls out its light is all but gone.
        assert!(at(Some(&close), &[&t], 1.0).colour[0] > 2.5);
        let faint = at(Some(&away), &[&t], 1.0).colour;
        assert!((faint[0] - 1.0).abs() < 0.1 && (faint[2] - 1.0).abs() < 0.1, "{faint:?}");
    }

    #[test]
    fn luminosity_follows_radius_and_temperature() {
        let sun = star(1, "Sol", "G", 5778.0, 1.0, 0.0);
        assert!((luminosity(&sun, 5778.0) - 1.0).abs() < 1e-4);
        let no_radius = body(1, "X", "K", 4500.0, &[], 0.0);
        let k = luminosity(&no_radius, 4500.0);
        assert!(k > 0.15 && k < 0.35, "a K without a radius: {k}");
        assert!(luminosity(&body(1, "X", "DA", 15000.0, &[], 0.0), 15000.0) < 0.02, "a white dwarf is dim");
        // A Y dwarf 3 Ls away against an F star 2074 Ls away (Pro Thaa SX-U d2-21, 2026-11-09): in the visible the F
        // still gives far more light, where the bolometric T⁴ had them level.
        let y = star(20, "X 4", "Y", 649.0, 0.108, 2074.0);
        let f = star(0, "X", "F", 6589.0, 1.1, 0.0);
        assert!(luminosity(&y, 649.0) / 9.0 < 0.01 * luminosity(&f, 6589.0) / (2074.0f32 * 2074.0), "{} vs {}", luminosity(&y, 649.0), luminosity(&f, 6589.0));
    }

    #[test]
    fn assignments_carry_the_colour_only_while_on() {
        let m = [body(1, "A", "M", 2703.0, &[], 0.0)];
        let on = at(None, &[&m], 1.0);
        let names: Vec<&str> = on.assignments().iter().map(|(n, _)| *n).collect();
        assert_eq!(names, ["x526", "y526", "z526", "w526", "x527"]);
        assert_eq!(on.assignments()[3].1, "1.000");
        assert_eq!(on.assignments()[4].1, "0");
        assert_eq!(at(None, &[&m], 0.0).assignments(), vec![("w526", "0".to_string()), ("x527", "0".to_string())]);
        let jumping = StarLight { rainbow: true, ..of_class("K", 1.0) };
        assert_eq!(jumping.assignments()[4], ("x527", "1".to_string()));
        assert!(jumping.colour[0] > 1.0 && jumping.colour[2] < 1.0, "{jumping:?}");
        assert_eq!(of_class("?", 1.0), StarLight::NONE);
        assert_eq!(at(None, &[], 1.0), StarLight::NONE);
        assert_eq!(at(None, &[&[body(1, "A", "H", 0.0, &[], 0.0)]], 1.0), StarLight::NONE);
    }
}
