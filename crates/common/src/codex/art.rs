//! Line pictures of the codex families: what a genus or kind of site looks like. Each
//! picture is a list of marks in a unit square ((0, 0) top left, (1, 1) bottom right), in
//! one colour. The client paints them with egui, the website renders them as SVG, so both
//! show the same drawing.

use std::f32::consts::{PI, TAU};

/// A point in the unit square.
pub type Pt = (f32, f32);

/// Where the ground line sits, as a fraction of the picture's height.
pub const GROUND: f32 = 0.88;

/// How a mark is outlined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pen {
    None,
    Main,
    Thin,
    /// A heavy line, for bodies drawn as strokes alone.
    Thick,
    /// A thin, half-transparent line: the ground, haze.
    Faint,
}

impl Pen {
    /// The line's width, as a multiple of the picture's line width.
    pub fn width(self) -> f32 {
        match self {
            Pen::None => 0.0,
            Pen::Main => 1.0,
            Pen::Thin | Pen::Faint => 0.6,
            Pen::Thick => 2.4,
        }
    }

    /// The line's opacity, 0 to 255.
    pub fn alpha(self) -> u8 {
        match self {
            Pen::None => 0,
            Pen::Faint => 90,
            _ => 255,
        }
    }
}

/// How a mark is filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    None,
    /// The body of a shape: the colour, mostly transparent.
    Tint,
    /// A soft light around something: fainter than a tint.
    Glow,
    Solid,
}

impl Fill {
    /// The fill's opacity, 0 to 255.
    pub fn alpha(self) -> u8 {
        match self {
            Fill::None => 0,
            Fill::Tint => 60,
            Fill::Glow => 40,
            Fill::Solid => 255,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A polyline; closed ones are polygons. Every filled polygon in a genus picture is
    /// convex, so the client can fill it as one.
    Path { pts: Vec<Pt>, closed: bool },
    Ellipse { c: Pt, r: Pt },
    /// A rectangle with corners rounded by `round`.
    Rect { min: Pt, max: Pt, round: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub shape: Shape,
    pub pen: Pen,
    pub fill: Fill,
}

/// The picture of the codex family `slug`, if it has one.
pub fn picture(slug: &str) -> Option<Vec<Mark>> {
    let mut p = Pic::default();
    match slug {
        "aleoida" => p.aleoida(),
        "bacterium" => p.bacterium(),
        "cactoida" => p.cactoida(),
        "clypeus" => p.clypeus(),
        "concha" => p.concha(),
        "electricae" => p.electricae(),
        "fonticulua" => p.fonticulua(),
        "frutexa" => p.frutexa(),
        "fumerola" => p.fumerola(),
        "fungoida" => p.fungoida(),
        "osseus" => p.osseus(),
        "recepta" => p.recepta(),
        "stratum" => p.stratum(),
        "tubus" => p.tubus(),
        "tussock" => p.tussock(),
        "radicoida" => p.radicoida(),
        "amphora-plant" => p.amphora(),
        "anemone" => p.anemone(),
        "bark-mound" => p.bark_mound(),
        "brain-tree" => p.brain_tree(),
        "crystalline-shards" => p.shards(),
        "sinuous-tubers" => p.tubers(),
        "crystals" => p.space_crystals(),
        "molluscs" => p.molluscs(),
        "pods-and-trees" => p.pods(),
        "mineral-spheres" => p.mineral_spheres(),
        "other-space-life" => p.plates(),
        "fumaroles" => p.fumaroles(),
        "ice-fumaroles" => p.ice_fumaroles(),
        "geysers" => p.geysers(),
        "gas-vents" => p.gas_vents(),
        "lava-spouts" => p.lava_spouts(),
        "lagrange-clouds" => p.clouds(),
        "anomalies" => p.anomaly(),
        "guardian-sites" => p.obelisks(),
        "guardian-beacons" => p.beacon(),
        "thargoid-vessels" => p.interceptor(),
        "thargoid-wrecks" => p.wreck(),
        "thargoid-barnacles" => p.barnacle(),
        "thargoid-sites" => p.thargoid_site(),
        "thargoid-spires" => p.spire(),
        _ => return None,
    }
    Some(p.marks)
}

/// A generic sprout, for life no picture covers.
pub fn sprout() -> Vec<Mark> {
    let mut p = Pic::default();
    p.sprout();
    p.marks
}

/// What the family `slug` looks like, in one sentence: the picture's caption.
pub fn looks(slug: &str) -> Option<&'static str> {
    Some(match slug {
        "aleoida" => "A low rosette of thick, pointed leaves, like an aloe.",
        "bacterium" => "A flat, patchy film on the ground. Easy to miss: look down, not up.",
        "cactoida" => "Upright ribbed columns with stubby arms, like a cactus.",
        "clypeus" => "Shield-shaped cups on short stalks.",
        "concha" => "A single cupped shell, opening upwards like a clam.",
        "electricae" => "Thin glowing filaments radiating from a small base.",
        "fonticulua" => "A rounded mound of stacked, frilly plates.",
        "frutexa" => "A woody, branching shrub.",
        "fumerola" => "Small chimneys venting plumes, near geological sites.",
        "fungoida" => "Mushroom caps on stalks, often in small clusters.",
        "osseus" => "Pale, bone-like plates fanning up from the ground.",
        "recepta" => "Round pods sitting on the ground, dimpled on top.",
        "stratum" => "Wide, flat leaves hugging the ground in layers.",
        "tubus" => "Clusters of upright tubes with open ends.",
        "tussock" => "A clump of grass-like blades curving outwards.",
        "radicoida" => "A swollen crown on great roots arching into the ground.",
        "amphora-plant" => "A single vase-shaped plant.",
        "anemone" => "A round bulb crowned with short tentacles.",
        "bark-mound" => "A cone-shaped mound with bark-like ridges.",
        "brain-tree" => "A trunk carrying a wrinkled, brain-like crown.",
        "crystalline-shards" => "Angular crystals jutting out of the ground.",
        "sinuous-tubers" => "Thick tubers snaking along the ground.",
        "crystals" => "A cluster of long crystals growing out from one core, adrift in space.",
        "molluscs" => "Soft domes trailing tendrils, floating in groups.",
        "pods-and-trees" => "A stalk carrying seed pods, with loose pods drifting around it.",
        "mineral-spheres" => "Round mineral bodies, solid or a lattice.",
        "other-space-life" => "Plates and spores drifting in space.",
        "fumaroles" => "A vent in a low mound, breathing plumes of hot gas.",
        "ice-fumaroles" => "An icy vent breathing vapour that freezes into snow.",
        "geysers" => "A cone throwing a jet high into the sky, its spray falling around it.",
        "gas-vents" => "An opening in the ground releasing a steady, spreading haze.",
        "lava-spouts" => "A cone throwing glowing gobbets of molten rock.",
        "lagrange-clouds" => "A coloured cloud among the stars; storm clouds crackle with lightning.",
        "anomalies" => "A glowing orb inside rings of energy.",
        "guardian-sites" => "Pointed obelisks carved with glyphs, standing in the ruins.",
        "guardian-beacons" => "A long crystal-like beacon with a glowing heart, hanging in space.",
        "thargoid-vessels" => "A Thargoid interceptor from above: petals around a glowing core.",
        "thargoid-wrecks" => "A broken Thargoid ship half sunk in the ground, petals strewn around.",
        "thargoid-barnacles" => "A barnacle: a stalk with curved barbs, coral growing beside it.",
        "thargoid-sites" => "A dome half buried in the ground with a glowing opening.",
        "thargoid-spires" => "A tall, thin spire rising far above the ground.",
        _ => return None,
    })
}

fn deg(d: f32) -> f32 {
    d * PI / 180.0
}

/// Builds a picture mark by mark.
#[derive(Default)]
struct Pic {
    marks: Vec<Mark>,
}

impl Pic {
    fn mark(&mut self, shape: Shape, pen: Pen, fill: Fill) {
        self.marks.push(Mark { shape, pen, fill });
    }
    fn path(&mut self, pts: &[Pt], pen: Pen) {
        self.mark(Shape::Path { pts: pts.to_vec(), closed: false }, pen, Fill::None);
    }
    fn line(&mut self, pts: &[Pt]) {
        self.path(pts, Pen::Main);
    }
    fn thin_line(&mut self, pts: &[Pt]) {
        self.path(pts, Pen::Thin);
    }
    /// A filled, outlined polygon.
    fn poly(&mut self, pts: &[Pt]) {
        self.poly_with(pts, Pen::Main, Fill::Tint);
    }
    fn poly_with(&mut self, pts: &[Pt], pen: Pen, fill: Fill) {
        self.mark(Shape::Path { pts: pts.to_vec(), closed: true }, pen, fill);
    }
    fn ellipse(&mut self, x: f32, y: f32, rx: f32, ry: f32) {
        self.ellipse_with(x, y, rx, ry, Pen::Main, Fill::Tint);
    }
    fn ellipse_with(&mut self, x: f32, y: f32, rx: f32, ry: f32, pen: Pen, fill: Fill) {
        self.mark(Shape::Ellipse { c: (x, y), r: (rx, ry) }, pen, fill);
    }
    fn dot(&mut self, x: f32, y: f32, rad: f32) {
        self.ellipse_with(x, y, rad, rad, Pen::None, Fill::Solid);
    }
    fn glow(&mut self, x: f32, y: f32, rad: f32) {
        self.ellipse_with(x, y, rad, rad, Pen::None, Fill::Glow);
    }
    /// A quadratic curve from `a` to `b`, pulled towards `ctrl`.
    fn quad(&mut self, a: Pt, ctrl: Pt, b: Pt) {
        let pts: Vec<Pt> = (0..=16)
            .map(|i| {
                let t = i as f32 / 16.0;
                let (u, v, w) = ((1.0 - t) * (1.0 - t), 2.0 * (1.0 - t) * t, t * t);
                (u * a.0 + v * ctrl.0 + w * b.0, u * a.1 + v * ctrl.1 + w * b.1)
            })
            .collect();
        self.line(&pts);
    }
    /// An ellipse around `c` turned by `angle` (radians), outlined thinly.
    fn ring(&mut self, c: Pt, rx: f32, ry: f32, angle: f32) {
        let (s, co) = angle.sin_cos();
        let pts: Vec<Pt> = (0..32)
            .map(|i| {
                let a = TAU * i as f32 / 32.0;
                let (x, y) = (rx * a.cos(), ry * a.sin());
                (c.0 + x * co - y * s, c.1 + x * s + y * co)
            })
            .collect();
        self.mark(Shape::Path { pts, closed: true }, Pen::Thin, Fill::None);
    }
    /// The top half of an ellipse standing on `y`: a dome.
    fn dome(&mut self, x: f32, y: f32, rx: f32, ry: f32) {
        let pts: Vec<Pt> = (0..=16)
            .map(|i| {
                let a = PI + PI * i as f32 / 16.0;
                (x + rx * a.cos(), y + ry * a.sin())
            })
            .collect();
        self.poly(&pts);
    }
    /// A filled leaf from `base` to `tip`, `width` across at its widest. Each side is an
    /// open line, so the pointed ends get no stroke join to spike out of.
    fn leaf(&mut self, base: Pt, tip: Pt, width: f32) {
        let (dx, dy) = (tip.0 - base.0, tip.1 - base.1);
        let l = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (nx, ny) = (-dy / l, dx / l);
        let side = |sign: f32| -> Vec<Pt> {
            (0..=12)
                .map(|i| {
                    let t = i as f32 / 12.0;
                    let off = sign * width * (t * PI).sin() * (1.0 - 0.3 * t);
                    (base.0 + dx * t + nx * off, base.1 + dy * t + ny * off)
                })
                .collect()
        };
        let (left, right) = (side(1.0), side(-1.0));
        let mut outline = left.clone();
        outline.extend(right.iter().rev().skip(1).take(11));
        self.poly_with(&outline, Pen::None, Fill::Tint);
        self.line(&left);
        self.line(&right);
    }
    /// A crystal from `c` pointing at `angle` (degrees, 0 = right, 90 = down).
    fn shard(&mut self, c: Pt, angle: f32, len: f32, half: f32) {
        let (dy, dx) = deg(angle).sin_cos();
        let (nx, ny) = (-dy, dx);
        let mid = (c.0 + dx * len * 0.35, c.1 + dy * len * 0.35);
        let tip = (c.0 + dx * len, c.1 + dy * len);
        self.poly(&[c, (mid.0 + nx * half, mid.1 + ny * half), tip, (mid.0 - nx * half, mid.1 - ny * half)]);
        self.thin_line(&[c, tip]);
    }
    /// Puffs of vapour or cloud: (x, y, rx, ry) each.
    fn puffs(&mut self, puffs: &[(f32, f32, f32, f32)], pen: Pen) {
        for &(x, y, rx, ry) in puffs {
            self.ellipse_with(x, y, rx, ry, pen, Fill::Glow);
        }
    }
    fn stars(&mut self, at: &[Pt]) {
        for &(x, y) in at {
            self.dot(x, y, 0.01);
        }
    }

    fn ground(&mut self) {
        self.path(&[(0.04, GROUND), (0.96, GROUND)], Pen::Faint);
    }

    // ── Surface life ──────────────────────────────────────────────────────────────

    fn aleoida(&mut self) {
        self.ground();
        for (tx, ty) in [(0.14, 0.42), (0.86, 0.42), (0.3, 0.2), (0.7, 0.2), (0.5, 0.12)] {
            self.leaf((0.5, GROUND), (tx, ty), 0.07);
        }
    }

    fn bacterium(&mut self) {
        self.ground();
        // A low film of overlapping patches with speckles on it.
        for (x, rx) in [(0.28, 0.2), (0.6, 0.24), (0.8, 0.12)] {
            self.ellipse(x, GROUND - 0.04, rx, 0.05);
        }
        for (x, y) in [(0.2, 0.83), (0.32, 0.85), (0.5, 0.83), (0.63, 0.86), (0.72, 0.82), (0.83, 0.84)] {
            self.dot(x, y, 0.018);
        }
    }

    fn cactoida(&mut self) {
        self.ground();
        for (x, top, bottom, half) in [(0.5, 0.14, GROUND, 0.1), (0.25, 0.36, 0.58, 0.06), (0.75, 0.28, 0.52, 0.06)] {
            self.mark(Shape::Rect { min: (x - half, top), max: (x + half, bottom), round: half }, Pen::Main, Fill::Tint);
        }
        self.line(&[(0.31, 0.54), (0.4, 0.54)]);
        self.line(&[(0.69, 0.48), (0.6, 0.48)]);
        for x in [0.46, 0.54] {
            self.thin_line(&[(x, 0.2), (x, GROUND - 0.04)]);
        }
    }

    fn clypeus(&mut self) {
        self.ground();
        for (x, y, s) in [(0.3, 0.42, 1.0), (0.7, 0.52, 0.8)] {
            self.line(&[(x, y + 0.08 * s), (x, GROUND)]);
            // A shield: a tilted, shallow cup.
            self.poly(&[
                (x - 0.2 * s, y - 0.06 * s),
                (x + 0.2 * s, y - 0.12 * s),
                (x + 0.16 * s, y + 0.04 * s),
                (x, y + 0.1 * s),
                (x - 0.15 * s, y + 0.06 * s),
            ]);
        }
    }

    fn concha(&mut self) {
        self.ground();
        // A fan-shaped shell with ribs, hinged at the ground.
        let hinge = (0.5, GROUND - 0.02);
        let rim: Vec<Pt> = (0..=8)
            .map(|i| {
                let a = PI + PI * i as f32 / 8.0;
                (0.5 + 0.38 * a.cos(), GROUND - 0.06 + 0.62 * a.sin())
            })
            .collect();
        let mut fan = vec![hinge];
        fan.extend(&rim);
        self.poly(&fan);
        for &(x, y) in rim.iter().skip(1).take(7) {
            self.thin_line(&[hinge, (x, y)]);
        }
    }

    fn electricae(&mut self) {
        self.ground();
        let base = (0.5, GROUND - 0.02);
        for i in 0..7 {
            let a = PI + PI * (i as f32 + 0.5) / 7.0;
            let tip = (0.5 + 0.4 * a.cos(), GROUND - 0.02 + 0.7 * a.sin());
            let mid = ((base.0 + tip.0) / 2.0 + 0.04 * (i as f32 - 3.0).signum(), (base.1 + tip.1) / 2.0);
            self.thin_line(&[base, mid, tip]);
            self.dot(tip.0, tip.1, 0.03);
            self.glow(tip.0, tip.1, 0.06);
        }
        self.ellipse(0.5, GROUND - 0.03, 0.1, 0.04);
    }

    fn fonticulua(&mut self) {
        self.ground();
        // Frilly plates stacked into a dome, widest at the bottom.
        for (i, (y, rx)) in [(0.74, 0.38), (0.6, 0.31), (0.47, 0.23), (0.36, 0.14)].into_iter().enumerate() {
            self.ellipse(0.5, y, rx, 0.07 - i as f32 * 0.008);
        }
        self.line(&[(0.5, 0.81), (0.5, GROUND)]);
    }

    fn fumerola(&mut self) {
        self.ground();
        for (x, h, s) in [(0.32, 0.36, 1.0), (0.7, 0.5, 0.75)] {
            self.poly(&[(x - 0.12 * s, GROUND), (x - 0.05 * s, GROUND - h), (x + 0.05 * s, GROUND - h), (x + 0.12 * s, GROUND)]);
            // A wavy plume rising from the vent.
            let top = GROUND - h;
            self.thin_line(&[(x, top - 0.02), (x - 0.05, top - 0.1), (x + 0.03, top - 0.18), (x - 0.02, top - 0.27)]);
        }
    }

    fn fungoida(&mut self) {
        self.ground();
        for (x, y, s) in [(0.32, 0.38, 1.0), (0.7, 0.56, 0.7), (0.52, 0.64, 0.5)] {
            self.line(&[(x, y), (x, GROUND)]);
            let cap: Vec<Pt> = (0..=10)
                .map(|i| {
                    let a = PI + PI * i as f32 / 10.0;
                    (x + 0.2 * s * a.cos(), y + 0.16 * s * a.sin())
                })
                .collect();
            self.poly(&cap);
        }
    }

    fn osseus(&mut self) {
        self.ground();
        // Bony plates fanning up, each a flattened ellipse leaning outwards.
        for (x, y, rx, ry) in [(0.5, 0.36, 0.13, 0.22), (0.3, 0.52, 0.11, 0.18), (0.7, 0.52, 0.11, 0.18), (0.5, 0.7, 0.3, 0.06)] {
            self.ellipse(x, y, rx, ry);
        }
        for (x, y) in [(0.5, 0.36), (0.3, 0.52), (0.7, 0.52)] {
            self.thin_line(&[(x, y - 0.1), (x, y + 0.1)]);
        }
    }

    fn recepta(&mut self) {
        self.ground();
        for (x, rad) in [(0.3, 0.17), (0.68, 0.22)] {
            let y = GROUND - rad;
            self.ellipse(x, y, rad, rad);
            self.ellipse_with(x, y - rad * 0.55, rad * 0.3, rad * 0.3, Pen::Thin, Fill::None);
        }
    }

    fn frutexa(&mut self) {
        self.ground();
        self.line(&[(0.5, GROUND), (0.5, 0.55), (0.32, 0.32), (0.22, 0.16)]);
        self.line(&[(0.5, 0.55), (0.66, 0.36), (0.76, 0.18)]);
        self.line(&[(0.5, 0.7), (0.24, 0.56)]);
        self.line(&[(0.5, 0.66), (0.8, 0.5)]);
        self.line(&[(0.32, 0.32), (0.42, 0.18)]);
        for (x, y) in [(0.22, 0.16), (0.42, 0.18), (0.76, 0.18), (0.24, 0.56), (0.8, 0.5), (0.6, 0.26)] {
            self.ellipse(x, y, 0.06, 0.045);
        }
    }

    fn stratum(&mut self) {
        self.ground();
        // Broad flat leaves spreading close to the ground.
        let base = (0.5, GROUND - 0.03);
        for tip in [(0.06, GROUND - 0.1), (0.94, GROUND - 0.1), (0.24, GROUND - 0.22), (0.76, GROUND - 0.22), (0.5, GROUND - 0.3)] {
            self.leaf(base, tip, 0.06);
        }
    }

    fn tubus(&mut self) {
        self.ground();
        for (x, top) in [(0.24, 0.42), (0.42, 0.2), (0.6, 0.3), (0.78, 0.5)] {
            let half = 0.07;
            self.poly(&[(x - half, GROUND), (x - half, top), (x + half, top), (x + half, GROUND)]);
            self.ellipse(x, top, half, 0.03);
        }
    }

    fn tussock(&mut self) {
        self.ground();
        for i in 0..9 {
            let t = i as f32 / 8.0 - 0.5;
            let tip = (0.5 + t * 0.9, 0.14 + t.abs() * 0.5);
            let ctrl = (0.5 + t * 0.2, 0.3);
            self.quad((0.5, GROUND), ctrl, tip);
        }
    }

    fn radicoida(&mut self) {
        self.ground();
        // A knobbly crown held up on roots that spread out into the ground.
        for (from, ctrl, to) in [
            ((0.4, 0.46), (0.2, 0.5), (0.06, GROUND)),
            ((0.44, 0.5), (0.3, 0.64), (0.24, GROUND)),
            ((0.48, 0.52), (0.44, 0.7), (0.4, GROUND)),
            ((0.52, 0.52), (0.56, 0.7), (0.6, GROUND)),
            ((0.56, 0.5), (0.7, 0.64), (0.76, GROUND)),
            ((0.6, 0.46), (0.8, 0.5), (0.94, GROUND)),
        ] {
            self.quad(from, ctrl, to);
        }
        self.dome(0.5, 0.5, 0.2, 0.3);
        for (x, y) in [(0.38, 0.36), (0.5, 0.26), (0.62, 0.36), (0.46, 0.42), (0.56, 0.44)] {
            self.ellipse_with(x, y, 0.035, 0.025, Pen::Thin, Fill::None);
        }
    }

    fn shards(&mut self) {
        self.ground();
        for (x, h, lean, half) in [(0.3, 0.5, -0.1, 0.07), (0.52, 0.72, 0.03, 0.09), (0.74, 0.42, 0.12, 0.07)] {
            let tip = (x + lean, GROUND - h);
            self.poly(&[(x - half, GROUND), (x - half + lean * 0.8, GROUND - h * 0.8), tip, (x + half + lean * 0.8, GROUND - h * 0.8), (x + half, GROUND)]);
            self.thin_line(&[(x, GROUND), tip]);
        }
    }

    fn amphora(&mut self) {
        self.ground();
        // A vase: narrow foot, round belly, narrow neck, flared lip.
        let half = |y: f32| -> f32 {
            let t = (GROUND - y) / 0.72;
            0.06 + 0.22 * (t * PI).sin().max(0.0) * (1.0 - t * 0.5)
        };
        let steps = 14;
        let side = |sign: f32| -> Vec<Pt> {
            (0..=steps)
                .map(|i| {
                    let y = GROUND - 0.66 * i as f32 / steps as f32;
                    (0.5 + sign * half(y), y)
                })
                .collect()
        };
        let mut outline = side(-1.0);
        outline.extend(side(1.0).into_iter().rev());
        self.poly_with(&outline, Pen::Main, Fill::None);
        self.ellipse(0.5, GROUND - 0.66, 0.13, 0.04);
    }

    fn anemone(&mut self) {
        self.ground();
        let (cx, cy, rad) = (0.5, 0.6, 0.22);
        for i in 0..9 {
            let a = PI + PI * (i as f32 + 0.5) / 9.0;
            let from = (cx + rad * 0.8 * a.cos(), cy + rad * 0.8 * a.sin());
            let tip = (cx + (rad + 0.16) * a.cos(), cy + (rad + 0.18) * a.sin());
            self.thin_line(&[from, tip]);
            self.dot(tip.0, tip.1, 0.022);
        }
        self.ellipse(cx, cy, rad, rad * 0.9);
        self.line(&[(cx, cy + rad * 0.9), (cx, GROUND)]);
    }

    fn brain_tree(&mut self) {
        self.ground();
        self.poly(&[(0.44, GROUND), (0.46, 0.52), (0.54, 0.52), (0.56, GROUND)]);
        self.ellipse(0.5, 0.34, 0.3, 0.2);
        // Folds across the crown.
        self.thin_line(&[(0.28, 0.32), (0.36, 0.26), (0.42, 0.34), (0.5, 0.24), (0.58, 0.34), (0.64, 0.26), (0.72, 0.34)]);
        self.thin_line(&[(0.3, 0.42), (0.4, 0.38), (0.5, 0.44), (0.6, 0.38), (0.7, 0.42)]);
    }

    fn bark_mound(&mut self) {
        self.ground();
        self.poly(&[(0.12, GROUND), (0.38, 0.22), (0.62, 0.22), (0.88, GROUND)]);
        for (y, w) in [(0.4, 0.17), (0.56, 0.24), (0.72, 0.31)] {
            self.thin_line(&[(0.5 - w, y), (0.5 - w * 0.3, y + 0.03), (0.5 + w * 0.3, y - 0.02), (0.5 + w, y)]);
        }
    }

    fn tubers(&mut self) {
        self.ground();
        for (y, phase) in [(0.6, 0.0), (0.74, 1.6)] {
            let wave: Vec<Pt> = (0..=24)
                .map(|i| {
                    let x = 0.08 + 0.84 * i as f32 / 24.0;
                    (x, y + 0.07 * (x * TAU * 1.5 + phase).sin())
                })
                .collect();
            self.path(&wave, Pen::Thick);
        }
        self.dot(0.92, 0.6 + 0.07 * (0.92 * TAU * 1.5).sin(), 0.04);
    }

    fn sprout(&mut self) {
        self.ground();
        self.quad((0.5, GROUND), (0.48, 0.55), (0.52, 0.3));
        self.leaf((0.51, 0.45), (0.24, 0.3), 0.07);
        self.leaf((0.51, 0.38), (0.78, 0.2), 0.07);
    }

    // ── Space life ────────────────────────────────────────────────────────────────

    fn space_crystals(&mut self) {
        self.stars(&[(0.12, 0.14), (0.88, 0.2), (0.1, 0.86), (0.9, 0.84)]);
        let core = (0.5, 0.55);
        for (angle, len, half) in [(-90.0, 0.42, 0.08), (-35.0, 0.36, 0.07), (-145.0, 0.34, 0.07), (20.0, 0.3, 0.06), (160.0, 0.28, 0.06), (95.0, 0.24, 0.05)] {
            self.shard(core, angle, len, half);
        }
        self.glow(core.0, core.1, 0.12);
    }

    fn molluscs(&mut self) {
        // An umbrella mollusc trailing tendrils, a small gourd mollusc beside it.
        for i in 0..5 {
            let x0 = 0.3 + 0.1 * i as f32;
            let wave: Vec<Pt> = (0..=10)
                .map(|j| {
                    let t = j as f32 / 10.0;
                    (x0 + 0.03 * (t * TAU * 1.5 + i as f32).sin(), 0.42 + 0.44 * t)
                })
                .collect();
            self.thin_line(&wave);
        }
        self.dome(0.5, 0.44, 0.3, 0.26);
        self.thin_line(&[(0.22, 0.44), (0.3, 0.4), (0.4, 0.44), (0.5, 0.4), (0.6, 0.44), (0.7, 0.4), (0.78, 0.44)]);
        self.ellipse(0.84, 0.24, 0.07, 0.09);
        self.ellipse(0.84, 0.13, 0.035, 0.03);
        self.stars(&[(0.12, 0.12), (0.14, 0.8), (0.9, 0.7)]);
    }

    fn pods(&mut self) {
        // A peduncle tree: a stalk with pods on its branches; loose pods drift around it.
        self.line(&[(0.5, 0.94), (0.47, 0.72), (0.52, 0.52), (0.5, 0.3)]);
        self.line(&[(0.49, 0.66), (0.31, 0.5)]);
        self.line(&[(0.51, 0.56), (0.71, 0.42)]);
        self.thin_line(&[(0.5, 0.94), (0.38, 0.98)]);
        self.thin_line(&[(0.5, 0.94), (0.62, 0.98)]);
        for (x, y, r) in [(0.27, 0.45, 0.075), (0.75, 0.37, 0.075), (0.5, 0.22, 0.09)] {
            self.ellipse(x, y, r, r * 0.85);
            self.thin_line(&[(x, y - r * 0.85), (x, y + r * 0.85)]);
        }
        for (x, y, r) in [(0.16, 0.18, 0.04), (0.85, 0.68, 0.05), (0.2, 0.76, 0.035), (0.84, 0.12, 0.03)] {
            self.ellipse(x, y, r, r);
        }
    }

    fn mineral_spheres(&mut self) {
        // A lattice sphere and a small solid one.
        self.ellipse(0.46, 0.54, 0.32, 0.32);
        for rx in [0.11, 0.22] {
            self.ellipse_with(0.46, 0.54, rx, 0.32, Pen::Thin, Fill::None);
        }
        self.thin_line(&[(0.46, 0.22), (0.46, 0.86)]);
        for (dy, half) in [(-0.16, 0.277), (0.0, 0.32), (0.16, 0.277)] {
            self.ellipse_with(0.46, 0.54 + dy, half, 0.04, Pen::Thin, Fill::None);
        }
        self.ellipse(0.84, 0.18, 0.08, 0.08);
        self.dot(0.81, 0.15, 0.018);
        self.stars(&[(0.12, 0.12), (0.9, 0.86)]);
    }

    fn plates(&mut self) {
        // Calcite-like plates and spores drifting together.
        for (x, y, rx, ry) in [(0.4, 0.38, 0.24, 0.1), (0.62, 0.58, 0.22, 0.09), (0.34, 0.76, 0.15, 0.06)] {
            self.ellipse(x, y, rx, ry);
            self.ellipse_with(x, y, rx * 0.55, ry * 0.55, Pen::Thin, Fill::None);
        }
        for (x, y) in [(0.8, 0.22), (0.86, 0.34), (0.16, 0.56), (0.74, 0.82), (0.58, 0.16)] {
            self.dot(x, y, 0.018);
        }
    }

    // ── Geology ───────────────────────────────────────────────────────────────────

    fn fumaroles(&mut self) {
        self.ground();
        self.poly(&[(0.2, GROUND), (0.36, 0.76), (0.64, 0.76), (0.8, GROUND)]);
        self.thin_line(&[(0.44, 0.76), (0.48, 0.8), (0.53, 0.77), (0.56, 0.76)]);
        self.puffs(&[(0.5, 0.66, 0.08, 0.05), (0.46, 0.54, 0.11, 0.07), (0.53, 0.4, 0.14, 0.08), (0.47, 0.24, 0.17, 0.09)], Pen::Thin);
        for (x, y) in [(0.12, 0.85), (0.88, 0.86), (0.9, 0.83)] {
            self.dot(x, y, 0.015);
        }
    }

    fn ice_fumaroles(&mut self) {
        self.ground();
        self.poly(&[(0.3, GROUND), (0.42, 0.74), (0.58, 0.74), (0.7, GROUND)]);
        self.poly(&[(0.2, GROUND), (0.25, 0.74), (0.31, GROUND)]);
        self.poly(&[(0.69, GROUND), (0.76, 0.7), (0.8, GROUND)]);
        self.puffs(&[(0.5, 0.64, 0.07, 0.04), (0.48, 0.53, 0.09, 0.055), (0.52, 0.41, 0.11, 0.06)], Pen::Thin);
        for (x, y) in [(0.26, 0.36), (0.76, 0.3), (0.7, 0.5), (0.3, 0.56), (0.62, 0.22), (0.38, 0.2), (0.84, 0.46)] {
            self.dot(x, y, 0.013);
        }
    }

    fn geysers(&mut self) {
        self.ground();
        // The jet, its spray arching back down, then the cone it rises from.
        self.poly_with(&[(0.47, 0.8), (0.45, 0.2), (0.55, 0.2), (0.53, 0.8)], Pen::Thin, Fill::Tint);
        self.glow(0.5, 0.18, 0.1);
        self.quad((0.47, 0.2), (0.3, 0.04), (0.18, 0.52));
        self.quad((0.53, 0.2), (0.7, 0.04), (0.82, 0.52));
        for (x, y) in [(0.22, 0.38), (0.26, 0.24), (0.36, 0.11), (0.78, 0.38), (0.74, 0.24), (0.64, 0.11), (0.16, 0.66), (0.84, 0.66)] {
            self.dot(x, y, 0.016);
        }
        self.poly(&[(0.3, GROUND), (0.43, 0.8), (0.57, 0.8), (0.7, GROUND)]);
    }

    fn gas_vents(&mut self) {
        self.ground();
        self.puffs(&[(0.5, 0.72, 0.14, 0.06), (0.4, 0.58, 0.2, 0.08), (0.6, 0.44, 0.24, 0.09), (0.46, 0.28, 0.28, 0.09)], Pen::Faint);
        self.ellipse(0.5, GROUND - 0.02, 0.13, 0.035);
        self.poly(&[(0.16, GROUND), (0.2, 0.8), (0.27, 0.82), (0.3, GROUND)]);
        self.poly(&[(0.72, GROUND), (0.76, 0.82), (0.82, 0.8), (0.86, GROUND)]);
    }

    fn lava_spouts(&mut self) {
        self.ground();
        self.glow(0.5, 0.36, 0.24);
        self.poly(&[(0.16, GROUND), (0.42, 0.56), (0.58, 0.56), (0.84, GROUND)]);
        self.ellipse(0.5, 0.56, 0.08, 0.025);
        self.thin_line(&[(0.46, 0.58), (0.4, 0.66), (0.42, 0.74), (0.34, GROUND - 0.02)]);
        for (x, y, r) in [(0.5, 0.44, 0.035), (0.44, 0.33, 0.03), (0.57, 0.27, 0.03), (0.36, 0.22, 0.025), (0.65, 0.16, 0.025), (0.28, 0.36, 0.02), (0.72, 0.34, 0.022), (0.5, 0.12, 0.02)] {
            self.dot(x, y, r);
        }
    }

    // ── Stellar phenomena ─────────────────────────────────────────────────────────

    fn clouds(&mut self) {
        self.stars(&[(0.12, 0.15), (0.85, 0.12), (0.18, 0.86), (0.9, 0.82), (0.6, 0.1), (0.08, 0.5)]);
        self.puffs(&[(0.3, 0.46, 0.2, 0.14), (0.52, 0.38, 0.24, 0.18), (0.72, 0.5, 0.2, 0.14), (0.5, 0.58, 0.3, 0.12)], Pen::Faint);
        // Lightning out of the storm.
        self.line(&[(0.56, 0.6), (0.49, 0.72), (0.56, 0.72), (0.48, 0.88)]);
    }

    fn anomaly(&mut self) {
        self.stars(&[(0.1, 0.12), (0.9, 0.16), (0.14, 0.88), (0.86, 0.9)]);
        self.glow(0.5, 0.5, 0.3);
        self.glow(0.5, 0.5, 0.18);
        self.ring((0.5, 0.5), 0.4, 0.1, deg(-25.0));
        self.ring((0.5, 0.5), 0.4, 0.1, deg(30.0));
        self.ellipse(0.5, 0.5, 0.1, 0.1);
        self.dot(0.5, 0.5, 0.05);
        for (x, y) in [(0.2, 0.3), (0.82, 0.36), (0.3, 0.78), (0.74, 0.72), (0.5, 0.14)] {
            self.dot(x, y, 0.014);
        }
    }

    // ── Guardian ──────────────────────────────────────────────────────────────────

    fn obelisks(&mut self) {
        self.ground();
        for (x, top, half) in [(0.26, 0.36, 0.05), (0.5, 0.16, 0.06), (0.74, 0.3, 0.05)] {
            self.poly(&[(x - half, GROUND), (x - half, top + 0.07), (x, top), (x + half, top + 0.07), (x + half, GROUND)]);
            // Glyphs down its face.
            for y in [top + 0.16, top + 0.28, top + 0.4] {
                if y < GROUND - 0.06 {
                    self.thin_line(&[(x - half * 0.5, y), (x, y - 0.03), (x + half * 0.5, y)]);
                }
            }
        }
        self.glow(0.5, 0.3, 0.07);
        self.poly(&[(0.06, GROUND), (0.1, 0.8), (0.16, 0.82), (0.18, GROUND)]);
        self.poly(&[(0.84, GROUND), (0.88, 0.78), (0.95, GROUND)]);
    }

    fn beacon(&mut self) {
        self.stars(&[(0.12, 0.14), (0.86, 0.1), (0.1, 0.84), (0.88, 0.88)]);
        self.poly(&[(0.4, 0.46), (0.16, 0.5), (0.4, 0.54)]);
        self.poly(&[(0.6, 0.46), (0.84, 0.5), (0.6, 0.54)]);
        self.poly(&[(0.5, 0.08), (0.6, 0.5), (0.5, 0.92), (0.4, 0.5)]);
        self.thin_line(&[(0.5, 0.08), (0.5, 0.92)]);
        self.ring((0.5, 0.5), 0.3, 0.07, 0.0);
        self.glow(0.5, 0.5, 0.14);
        self.dot(0.5, 0.5, 0.04);
    }

    // ── Thargoid ──────────────────────────────────────────────────────────────────

    fn interceptor(&mut self) {
        let c = (0.5, 0.5);
        for i in 0..8 {
            let a = deg(-90.0 + 45.0 * i as f32);
            let len = if i % 2 == 0 { 0.44 } else { 0.32 };
            self.leaf(c, (c.0 + len * a.cos(), c.1 + len * a.sin()), 0.08);
        }
        self.ellipse(c.0, c.1, 0.13, 0.13);
        self.glow(c.0, c.1, 0.22);
        self.dot(c.0, c.1, 0.045);
    }

    fn wreck(&mut self) {
        self.ground();
        // Petals torn off and strewn about, the core sunk into the ground.
        self.leaf((0.3, GROUND - 0.02), (0.06, GROUND - 0.16), 0.06);
        self.leaf((0.7, GROUND - 0.02), (0.95, GROUND - 0.3), 0.06);
        self.leaf((0.62, GROUND - 0.2), (0.82, 0.38), 0.06);
        self.dome(0.48, GROUND, 0.2, 0.24);
        self.thin_line(&[(0.4, 0.68), (0.46, 0.74), (0.43, 0.8)]);
        self.thin_line(&[(0.55, 0.66), (0.52, 0.74), (0.58, 0.78)]);
        self.thin_line(&[(0.46, 0.62), (0.42, 0.52), (0.48, 0.42), (0.44, 0.3)]);
        for (x, y) in [(0.18, 0.84), (0.24, 0.86), (0.84, 0.85), (0.14, 0.82)] {
            self.dot(x, y, 0.015);
        }
    }

    fn barnacle(&mut self) {
        self.ground();
        for (from, ctrl, tip) in [
            ((0.46, 0.5), (0.3, 0.5), (0.18, 0.3)),
            ((0.46, 0.66), (0.28, 0.66), (0.12, 0.52)),
            ((0.54, 0.5), (0.7, 0.5), (0.82, 0.3)),
            ((0.54, 0.66), (0.72, 0.66), (0.88, 0.52)),
        ] {
            self.quad(from, ctrl, tip);
        }
        self.poly(&[(0.43, GROUND), (0.46, 0.36), (0.54, 0.36), (0.57, GROUND)]);
        self.leaf((0.5, 0.38), (0.5, 0.1), 0.06);
        self.glow(0.5, 0.36, 0.08);
        // Coral beside it.
        self.line(&[(0.86, GROUND), (0.86, 0.72)]);
        self.line(&[(0.86, 0.8), (0.8, 0.72)]);
        self.line(&[(0.86, 0.76), (0.92, 0.68)]);
    }

    fn thargoid_site(&mut self) {
        self.ground();
        self.dome(0.46, GROUND, 0.36, 0.32);
        for (x0, x1) in [(0.24, 0.36), (0.68, 0.56)] {
            self.thin_line(&[(x0, GROUND), (x1, 0.6)]);
        }
        let hex: Vec<Pt> = (0..6)
            .map(|i| {
                let a = deg(30.0 + 60.0 * i as f32);
                (0.46 + 0.08 * a.cos(), 0.74 + 0.08 * a.sin())
            })
            .collect();
        self.poly_with(&hex, Pen::Main, Fill::Glow);
        self.dot(0.46, 0.74, 0.03);
        self.ellipse(0.88, GROUND - 0.06, 0.055, 0.06);
        self.thin_line(&[(0.88, GROUND - 0.12), (0.88, GROUND)]);
    }

    fn spire(&mut self) {
        self.ground();
        self.poly(&[(0.14, GROUND), (0.19, 0.5), (0.21, 0.5), (0.26, GROUND)]);
        self.poly(&[(0.42, GROUND), (0.48, 0.09), (0.52, 0.09), (0.58, GROUND)]);
        for (y, rx) in [(0.32, 0.12), (0.56, 0.16)] {
            self.ellipse_with(0.5, y, rx, 0.03, Pen::Thin, Fill::None);
        }
        for (a, b) in [((0.47, 0.2), (0.4, 0.14)), ((0.53, 0.2), (0.6, 0.14)), ((0.45, 0.44), (0.36, 0.38)), ((0.55, 0.44), (0.64, 0.38))] {
            self.line(&[a, b]);
        }
        self.glow(0.5, 0.09, 0.07);
        self.poly(&[(0.74, GROUND), (0.78, 0.64), (0.8, 0.64), (0.84, GROUND)]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::articles::families;

    #[test]
    fn every_codex_family_has_a_picture_and_caption() {
        for f in families() {
            assert!(picture(f.slug).is_some_and(|p| !p.is_empty()), "{} has no picture", f.slug);
            assert!(looks(f.slug).is_some(), "{} has no caption", f.slug);
        }
    }

    #[test]
    fn pictures_stay_in_their_square() {
        for f in families() {
            for m in picture(f.slug).unwrap() {
                let pts = match m.shape {
                    Shape::Path { pts, .. } => pts,
                    Shape::Ellipse { c, r } => vec![(c.0 - r.0, c.1 - r.1), (c.0 + r.0, c.1 + r.1)],
                    Shape::Rect { min, max, .. } => vec![min, max],
                };
                for (x, y) in pts {
                    assert!((-0.01..=1.01).contains(&x) && (-0.01..=1.01).contains(&y), "{} leaves its square at ({x}, {y})", f.slug);
                }
            }
        }
    }
}
