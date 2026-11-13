//! The ground detail textures of the 3DMigoto ground shaders: per ground type a tileable colour
//! map, a relief map (slopes) and a map of scattered stones, made on the player's machine when
//! the shader mods are installed. A port of `scripts/ed-shader/ground_detail_tex.py`, whose
//! docstring has the physics and the sources of every look (sand, regolith, ice, frost, rocky
//! ice, lava); the two make the same textures: a counter-based random generator (splitmix64)
//! and the same order of draws. `the_looks_match_the_script` checks it against
//! `testdata/ground_detail_golden.txt`; after a change to the script regenerate that with
//! `cd scripts/ed-shader && python3 ground_detail_golden.py > ../../crates/client/testdata/ground_detail_golden.txt`
//! and port the change here.
//!
//! The files are DDS arrays (DX10 header, one slice per type), block compressed: BC5 for the
//! colour channels in pairs and for slope pairs, BC4 for the stones' shade. 3DMigoto's loader
//! (DirectXTK) reads them as texture arrays. Six types at 1024 px come to 46 MB.

use std::f64::consts::PI;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// The tiles the layers are laid at, in metres (`ground_tint.py` `DETAIL_LAYERS`).
const SMALL: f64 = 2.0;
const LARGE: f64 = 8.0;
/// The stones' texture covers this many metres (`PEBBLE_LAYERS`).
const PEBBLES: f64 = 8.0;
const SLOPE_MAX: f64 = 1.0;
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// The ground types, in the order of the arrays' slices (`x14` in the shader).
pub const TYPES: [&str; 6] = ["sand", "regolith", "ice", "frost", "rocky_ice", "lava"];

/// The 11 planes of one type, in this order: colour R G B A, the 2 m layer's slopes x y, the 8 m
/// layer's, the stones' shade, the stones' slopes x y. 8 bits each.
const C0: usize = 0;
const R0: usize = 4;
const PS: usize = 8;
const P0: usize = 9;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Format {
    Bc4,
    Bc5,
}

/// The files: name, format, the planes (by index) they hold.
const FILES: [(&str, Format, &[usize]); 6] = [
    ("colour_a", Format::Bc5, &[C0, C0 + 1]),
    ("colour_b", Format::Bc5, &[C0 + 2, C0 + 3]),
    ("relief_a", Format::Bc5, &[R0, R0 + 1]),
    ("relief_b", Format::Bc5, &[R0 + 2, R0 + 3]),
    ("pebbles_slope", Format::Bc5, &[P0, P0 + 1]),
    ("pebbles_shade", Format::Bc4, &[PS]),
];

/// The snow's files: name, format, the planes of `snow_planes` (fractal height, slope x, slope y, churn height) of each of two identical slices (so that the shader reads arrays).
/// The height file is BC5: x the fractal's height (the lace, the hollows), y the churned snow's (the parallax march, times the snow mask); the slope file holds the slopes of the
/// fractal plus the churn (the relief).
const SNOW_FILES: [(&str, Format, &[&[usize]]); 2] = [("snow_slope", Format::Bc5, &[&[1, 2], &[1, 2]]), ("snow_height", Format::Bc5, &[&[0, 3], &[0, 3]])];

/// The snow's map is this many px a side (`SNOW_SIZE` in the script): a tile of 4 m, 2 mm a texel.
pub const SNOW_SIZE: usize = 2048;

/// The names of the files `write_all` writes into `ShaderFixes` (`edcas_ground_<name>.dds`).
pub fn file_names() -> Vec<String> {
    FILES.iter().map(|(n, _, _)| *n).chain(SNOW_FILES.iter().map(|(n, _, _)| *n)).map(|n| format!("edcas_ground_{n}.dds")).collect()
}

// ------------------------------------------------------------------ random numbers

/// splitmix64's output function.
fn splitmix(z: u64) -> u64 {
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn unit(bits: u64) -> f64 {
    (bits >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
}

/// Counter-based random numbers: value i of the stream depends on the seed and i alone.
struct Rng {
    seed: u64,
    n: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng { seed, n: 0 }
    }

    /// `count` uniform numbers in [0, 1).
    fn uniform(&mut self, count: usize) -> Vec<f64> {
        let first = self.n;
        self.n += count as u64;
        (0..count as u64).map(|i| unit(splitmix(self.seed.wrapping_add((first + i + 1).wrapping_mul(GOLDEN))))).collect()
    }

    fn normal(&mut self, count: usize) -> Vec<f64> {
        let u1 = self.uniform(count);
        let u2 = self.uniform(count);
        u1.iter().zip(&u2).map(|(a, b)| (-2.0 * (1.0 - a).ln()).sqrt() * (2.0 * PI * b).cos()).collect()
    }

    /// Integers 0 .. top - 1.
    fn below(&mut self, count: usize, top: usize) -> Vec<usize> {
        self.uniform(count).iter().map(|u| ((u * top as f64) as usize).min(top - 1)).collect()
    }
}

/// A uniform number from an integer key and a salt.
fn hash_uniform(key: u64, salt: u64) -> f64 {
    unit(splitmix(key.wrapping_mul(GOLDEN).wrapping_add(salt)))
}

// ------------------------------------------------------------------ small helpers

type Grid = Vec<f64>;

fn mean(a: &[f64]) -> f64 {
    a.iter().sum::<f64>() / a.len() as f64
}

/// Population standard deviation.
fn std_dev(a: &[f64]) -> f64 {
    let m = mean(a);
    (a.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / a.len() as f64).sqrt()
}

fn clip(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

/// Rounds to f32 and back: the script stores some arrays in float32.
fn f32r(v: f64) -> f64 {
    v as f32 as f64
}

/// 0.5 + (a - mean) / (spread x std) x 0.5: `spread` standard deviations reach the ends.
fn to_unit(a: &[f64], spread: f64) -> Grid {
    let (m, s) = (mean(a), std_dev(a));
    a.iter().map(|v| clip(0.5 + (v - m) / (spread * (s + 1e-12)) * 0.5, 0.0, 1.0)).collect()
}

/// 0.5 + `amount` x a, the mean put back to 0.5.
fn centred(a: &[f64], amount: f64) -> Grid {
    let m = mean(a);
    let b: Grid = a.iter().map(|v| 0.5 + amount * (v - m)).collect();
    let shift = 0.5 - mean(&b);
    b.iter().map(|v| clip(v + shift, 0.0, 1.0)).collect()
}

fn add(a: &[f64], b: &[f64]) -> Grid {
    a.iter().zip(b).map(|(x, y)| x + y).collect()
}

fn scaled(a: &[f64], k: f64) -> Grid {
    a.iter().map(|v| v * k).collect()
}

fn lin(a: &[f64], ka: f64, b: &[f64], kb: f64) -> Grid {
    a.iter().zip(b).map(|(x, y)| ka * x + kb * y).collect()
}

// ------------------------------------------------------------------ FFT noise

/// In-place radix-2 FFT of `re`, `im` (length a power of two); the inverse is not scaled.
fn fft1(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let angle = sign * 2.0 * PI / len as f64;
        let (wr, wi): (Vec<f64>, Vec<f64>) = (0..len / 2).map(|k| ((angle * k as f64).cos(), (angle * k as f64).sin())).unzip();
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * wr[k] - im[b] * wi[k], re[b] * wi[k] + im[b] * wr[k]);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
}

fn transpose(a: &mut [f64], n: usize) {
    for y in 0..n {
        for x in y + 1..n {
            a.swap(y * n + x, x * n + y);
        }
    }
}

/// 2D FFT of an n x n picture; the inverse is scaled, as numpy's.
fn fft2(re: &mut [f64], im: &mut [f64], n: usize, inverse: bool) {
    for _ in 0..2 {
        for y in 0..n {
            fft1(&mut re[y * n..(y + 1) * n], &mut im[y * n..(y + 1) * n], inverse);
        }
        transpose(re, n);
        transpose(im, n);
    }
    if inverse {
        let k = 1.0 / (n * n) as f64;
        re.iter_mut().for_each(|v| *v *= k);
        im.iter_mut().for_each(|v| *v *= k);
    }
}

/// Frequency of bin i of n, in cycles per tile (numpy's fftfreq x n).
fn freq(i: usize, n: usize) -> f64 {
    if i < n / 2 {
        i as f64
    } else {
        i as f64 - n as f64
    }
}

/// White noise kept between `lo` and `hi` cycles per tile, 1/f, unit standard deviation;
/// periodic. `stretch` > 1 squeezes the spectrum along y.
fn band_noise(rng: &mut Rng, size: usize, lo: f64, hi: f64, stretch: f64) -> Grid {
    let mut re = rng.normal(size * size);
    let mut im = vec![0.0; size * size];
    fft2(&mut re, &mut im, size, false);
    for y in 0..size {
        for x in 0..size {
            let (kx, ky) = (freq(x, size), freq(y, size));
            let r = (kx * kx + (ky * stretch) * (ky * stretch)).sqrt();
            let band = clip((r - lo) / lo.max(1.0), 0.0, 1.0) * clip((hi - r) / (0.5 * hi), 0.0, 1.0);
            let k = band / r.max(1.0);
            re[y * size + x] *= k;
            im[y * size + x] *= k;
        }
    }
    fft2(&mut re, &mut im, size, true);
    let s = std_dev(&re) + 1e-12;
    re.iter().map(|v| v / s).collect()
}

/// Gaussian blur, periodic; sigma in pixels.
fn blur(a: &[f64], size: usize, sigma: f64) -> Grid {
    let mut re = a.to_vec();
    let mut im = vec![0.0; size * size];
    fft2(&mut re, &mut im, size, false);
    for y in 0..size {
        for x in 0..size {
            let (kx, ky) = (freq(x, size) / size as f64, freq(y, size) / size as f64);
            let k = (-2.0 * PI * PI * sigma * sigma * (kx * kx + ky * ky)).exp();
            re[y * size + x] *= k;
            im[y * size + x] *= k;
        }
    }
    fft2(&mut re, &mut im, size, true);
    re
}

/// How far the height lies under its surroundings (blurred over `scale` metres): pits dark,
/// ridges light. Unit standard deviation.
fn cavity(height: &[f64], size: usize, metres: f64, scale: f64) -> Grid {
    let c: Grid = blur(height, size, scale * size as f64 / metres).iter().zip(height).map(|(b, h)| b - h).collect();
    let s = std_dev(&c) + 1e-12;
    c.iter().map(|v| v / s).collect()
}

// ------------------------------------------------------------------ features

/// Stones' shape: `crown` rounds the top (1.5 facets, 2.5 a boulder), `faces` their range.
struct Shape {
    ridge: f64,
    crown: f64,
    faces: (usize, usize),
}

const FACETED: Shape = Shape { ridge: 0.5, crown: 1.5, faces: (5, 8) };

/// Stones at (px, py) pixels of `radius` metres each, half buried: angular, elongated at random
/// angles, rising to a ridge. `tone` is each one's darkness (negative = lighter), `bury` (0..1)
/// scales a stone's height. Returns their darkness and their height in metres.
#[allow(clippy::too_many_arguments)]
fn stones(rng: &mut Rng, size: usize, metres: f64, px: &[f64], py: &[f64], radius: &[f64], tone: &[f64], bury: Option<&[f64]>, shape: &Shape) -> (Grid, Grid) {
    let mut shade: Grid = vec![0.0; size * size];
    let mut height: Grid = vec![0.0; size * size];
    let count = px.len();
    let angle: Vec<f64> = rng.uniform(count).iter().map(|u| u * PI).collect();
    let stretch: Vec<f64> = rng.uniform(count).iter().map(|u| 1.0 + 0.6 * u).collect();
    let faces: Vec<usize> = rng.below(count, shape.faces.1 - shape.faces.0 + 1).iter().map(|f| shape.faces.0 + f).collect();
    let jitter_n = rng.uniform(count * 8);
    let jitter_h = rng.uniform(count * 8);
    let n = size as i64;
    for i in 0..count {
        let r = radius[i] * size as f64 / metres;
        let reach = r * stretch[i] * 1.4;
        let (x0, x1) = ((px[i] - reach) as i64 - 1, (px[i] + reach) as i64 + 2);
        let (y0, y1) = ((py[i] - reach) as i64 - 1, (py[i] + reach) as i64 + 2);
        let f = faces[i];
        // The outline: the farthest of the faces' half-planes, each at its own angle and distance.
        let sides: Vec<(f64, f64, f64)> = (0..f)
            .map(|k| {
                let a = (k as f64 + 0.6 * jitter_n[i * 8 + k]) * 2.0 * PI / f as f64;
                (a.cos(), a.sin(), r * (0.8 + 0.3 * jitter_h[i * 8 + k]))
            })
            .collect();
        let (ca, sa) = (angle[i].cos(), angle[i].sin());
        let mut writes = Vec::with_capacity(((x1 - x0) * (y1 - y0)) as usize);
        for y in y0..y1 {
            for x in x0..x1 {
                let (dx, dy) = (x as f64 - px[i], y as f64 - py[i]);
                let u = dx * ca + dy * sa;
                let w = dy * ca - dx * sa;
                let d = sides.iter().map(|(c, s, h)| (u / stretch[i] * c + w * s) / h).fold(f64::NEG_INFINITY, f64::max);
                let inside = d < 1.0;
                let ring = if inside { 0.0 } else { clip((1.35 - d) / 0.35, 0.0, 1.0) };
                let at = (y.rem_euclid(n) * n + x.rem_euclid(n)) as usize;
                let value = if inside { tone[i] * clip((1.0 - d) * 6.0, 0.0, 1.0) } else { shade[at].max(0.25 * ring) };
                let rise = shape.ridge * bury.map_or(1.0, |b| b[i]) * radius[i] * clip(1.0 - d.max(0.0).powf(shape.crown), 0.0, 1.0);
                writes.push((at, f32r(value), f32r(rise)));
            }
        }
        for (at, value, rise) in writes {
            shade[at] = value;
            height[at] = f32r(height[at].max(rise));
        }
    }
    (shade, height)
}

/// Stone tones: a share `dark` dark, up to `grey` grey, the rest lighter than the ground.
fn tones(rng: &mut Rng, count: usize, dark: f64, grey: f64) -> Vec<f64> {
    let kind = rng.uniform(count);
    let u = rng.uniform(count);
    (0..count)
        .map(|i| {
            if kind[i] < dark {
                0.6 + 0.4 * u[i]
            } else if kind[i] < grey {
                0.25 + 0.2 * u[i]
            } else {
                -0.3 - 0.2 * u[i]
            }
        })
        .collect()
}

/// One place per cell of a square grid (count rounded to it), jittered, 15 % of the cells
/// left out. Positions in pixels.
fn grid_places(rng: &mut Rng, size: usize, count: usize) -> (Vec<f64>, Vec<f64>) {
    let cells = (count as f64).sqrt().round_ties_even() as usize;
    let total = cells * cells;
    let keep = rng.uniform(total);
    let jx = rng.uniform(total);
    let jy = rng.uniform(total);
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for i in 0..total {
        if keep[i] < 0.85 {
            let (cy, cx) = ((i / cells) as f64, (i % cells) as f64);
            xs.push((cx + 0.5 + 0.8 * (jx[i] - 0.5)) * size as f64 / cells as f64);
            ys.push((cy + 0.5 + 0.8 * (jy[i] - 0.5)) * size as f64 / cells as f64);
        }
    }
    (xs, ys)
}

/// Sizes with a cumulative number falling as size^-exponent between lo and hi.
fn power_radii(rng: &mut Rng, count: usize, lo: f64, hi: f64, exponent: f64) -> Vec<f64> {
    rng.uniform(count).iter().map(|u| lo * (1.0 - u * (1.0 - (lo / hi).powf(exponent))).powf(-1.0 / exponent)).collect()
}

/// A crater's height over its radius (0 to 3 radii, 100 steps per radius): a bowl, a rim at 1,
/// the ejecta blanket thinning as r^-3, smoothed over `soft` radii.
fn crater_profile(rim: f64, deep: f64, ejecta: bool, bowl: f64, soft: f64) -> Vec<f64> {
    let raw: Vec<f64> = (0..301)
        .map(|i| {
            let rho = i as f64 * 3.0 / 300.0;
            if rho < 1.0 {
                rim - (rim + deep) * (1.0 - rho.min(1.0).powi(2)).powf(bowl)
            } else if ejecta {
                rim * rho.max(1.0).powi(-3) * clip(3.0 - rho, 0.0, 1.0)
            } else {
                0.0
            }
        })
        .collect();
    let sigma = soft * 100.0;
    let m = (3.0 * sigma) as i64;
    let kernel: Vec<f64> = (-m - 1..m + 2).map(|i| (-0.5 * (i as f64 / sigma).powi(2)).exp()).collect();
    let total: f64 = kernel.iter().sum();
    let pad = kernel.len() / 2;
    // The reflected, then extended profile, as the script pads it, and a 'valid' convolution.
    let mut padded: Vec<f64> = (1..=pad).rev().map(|i| raw[i]).collect();
    padded.extend_from_slice(&raw);
    padded.extend(std::iter::repeat_n(raw[300], pad));
    (0..raw.len()).map(|i| (0..kernel.len()).map(|k| padded[i + k] * kernel[kernel.len() - 1 - k]).sum::<f64>() / total).collect()
}

fn interp(profile: &[f64], rho: f64) -> f64 {
    let at = rho * 100.0;
    if at >= 300.0 {
        return profile[300];
    }
    let i = at as usize;
    profile[i] + (profile[i + 1] - profile[i]) * (at - i as f64)
}

struct CraterSpec {
    diameter: (f64, f64),
    exponent: f64,
    depth: f64,
    rim: f64,
    ejecta: bool,
    age: (f64, f64),
    bowl: f64,
}

impl CraterSpec {
    /// Simple craters 0.2 deep and 0.036 high (Pike 1977), a saturated surface (exponent 2).
    fn new(diameter: (f64, f64)) -> Self {
        CraterSpec { diameter, exponent: 2.0, depth: 0.2, rim: 0.036, ejecta: true, age: (0.3, 1.0), bowl: 1.0 }
    }
}

/// Craters with a cumulative number falling as D^-exponent, each scaled by its age; a newer
/// bowl erases the older relief in it, fading in towards the rim. `places`: their positions
/// (pixels), else `count` random ones. Returns the height (m) and the brightness of the fresh
/// soil they uncover (0..1).
fn craters(rng: &mut Rng, size: usize, metres: f64, count: usize, spec: &CraterSpec, places: Option<(&[f64], &[f64])>) -> (Grid, Grid) {
    let mut height: Grid = vec![0.0; size * size];
    let mut bright: Grid = vec![0.0; size * size];
    let (px, py): (Vec<f64>, Vec<f64>) = match places {
        Some((x, y)) => (x.to_vec(), y.to_vec()),
        None => {
            let x: Vec<f64> = rng.uniform(count).iter().map(|u| u * size as f64).collect();
            let y: Vec<f64> = rng.uniform(count).iter().map(|u| u * size as f64).collect();
            (x, y)
        }
    };
    let count = px.len();
    let dia = power_radii(rng, count, spec.diameter.0, spec.diameter.1, spec.exponent);
    let fresh: Vec<f64> = rng.uniform(count).iter().map(|u| spec.age.0 + (spec.age.1 - spec.age.0) * u * u).collect();
    let n = size as i64;
    for i in 0..count {
        let r = dia[i] / 2.0 * size as f64 / metres;
        let reach = if spec.ejecta { 3.0 } else { 1.8 };
        let span = (r * reach) as i64 + 2;
        let (x0, y0) = (px[i] as i64 - span, py[i] as i64 - span);
        let profile = crater_profile(spec.rim * dia[i] * fresh[i], spec.depth * dia[i] * fresh[i].powf(1.3), spec.ejecta, spec.bowl, 0.04 + 0.3 * (1.0 - fresh[i]));
        let mut writes = Vec::new();
        for y in y0..=(py[i] as i64 + span) {
            for x in x0..=(px[i] as i64 + span) {
                let rho = ((x as f64 - px[i]).powi(2) + (y as f64 - py[i]).powi(2)).sqrt() / r;
                let at = (y.rem_euclid(n) * n + x.rem_euclid(n)) as usize;
                let keep = if rho < 1.0 { rho.powi(4) } else { 1.0 };
                let glow = fresh[i] * fresh[i] * clip((2.2 - rho) / 1.2, 0.0, 1.0);
                writes.push((at, height[at] * keep + interp(&profile, rho), bright[at].max(glow)));
            }
        }
        for (at, h, b) in writes {
            height[at] = h;
            bright[at] = b;
        }
    }
    (height, bright)
}

/// Voronoi cells on a jittered grid of n x n seeds, periodic: for each pixel the distances
/// (pixels) to its nearest and second nearest seed, their cell numbers and the offset from the
/// nearest seed.
struct Cells {
    size: usize,
    /// The key space of a pair of cells: `n * n` for the grid, the number of cells for scattered ones.
    stride: usize,
    n: usize,
    f1: Vec<f64>,
    f2: Vec<f64>,
    i1: Vec<usize>,
    i2: Vec<usize>,
    dx: Vec<f64>,
    dy: Vec<f64>,
}

impl Cells {
    fn new(rng: &mut Rng, size: usize, n: usize) -> Self {
        let jitter = 0.8;
        let jx = rng.uniform(n * n);
        let jy = rng.uniform(n * n);
        let seed_x: Vec<f64> = (0..n * n).map(|i| ((i % n) as f64 + 0.5 + jitter * (jx[i] - 0.5)) * size as f64 / n as f64).collect();
        let seed_y: Vec<f64> = (0..n * n).map(|i| ((i / n) as f64 + 0.5 + jitter * (jy[i] - 0.5)) * size as f64 / n as f64).collect();
        let total = size * size;
        let (mut d1, mut d2) = (vec![f64::INFINITY; total], vec![f64::INFINITY; total]);
        let (mut i1, mut i2) = (vec![0usize; total], vec![0usize; total]);
        let (mut ox, mut oy) = (vec![0.0; total], vec![0.0; total]);
        let ni = n as i64;
        for oy_cell in -1..=1i64 {
            for ox_cell in -1..=1i64 {
                for y in 0..size {
                    let py = y as f64 + 0.5;
                    let cyp = (py * n as f64 / size as f64) as i64;
                    let (gy, wy) = (cyp + oy_cell, (cyp + oy_cell).rem_euclid(ni));
                    for x in 0..size {
                        let px = x as f64 + 0.5;
                        let cxp = (px * n as f64 / size as f64) as i64;
                        let (gx, wx) = (cxp + ox_cell, (cxp + ox_cell).rem_euclid(ni));
                        let seed = (wy * ni + wx) as usize;
                        let dx = px - (seed_x[seed] + gx.div_euclid(ni) as f64 * size as f64);
                        let dy = py - (seed_y[seed] + gy.div_euclid(ni) as f64 * size as f64);
                        let d = dx * dx + dy * dy;
                        let at = y * size + x;
                        if d < d1[at] {
                            d2[at] = d1[at];
                            i2[at] = i1[at];
                            ox[at] = dx;
                            oy[at] = dy;
                            d1[at] = d;
                            i1[at] = seed;
                        } else if d < d2[at] {
                            d2[at] = d;
                            i2[at] = seed;
                        }
                    }
                }
            }
        }
        Cells { size, stride: n * n, n, f1: d1.iter().map(|d| d.sqrt()).collect(), f2: d2.iter().map(|d| d.sqrt()).collect(), i1, i2, dx: ox, dy: oy }
    }

    /// Cells of mixed sizes round randomly scattered seeds, periodic (port of `Scatter`: the lava's
    /// and the ice's polygons on a jittered grid lined up as a grid): seeds are thrown at random with a
    /// size factor of 0.55 to 2.05 and kept where they keep their distance to the kept ones. Returns
    /// the cells and the stream for their per-cell values; `rng` advances as the jittered grid's
    /// `n` x `n` seeds did (2 n n numbers). `warp` (amplitude in cells, lo, hi in cycles per tile): the
    /// pixels' points are moved by two band noises from the cells' own stream after the seeds, so the
    /// edges wobble (the ice's cracks read as a honeycomb, 2026-11-07).
    fn scattered(rng: &mut Rng, size: usize, n: usize, warp: Option<(f64, f64, f64)>) -> (Self, Rng) {
        let mut own = Rng::new(rng.seed.wrapping_add(rng.n.wrapping_mul(GOLDEN)).wrapping_add(0xA5CA_77E2));
        rng.n += 2 * (n * n) as u64;
        let tries = 30 * n * n;
        let (x, y, u) = (own.uniform(tries), own.uniform(tries), own.uniform(tries));
        let factor: Vec<f64> = u.iter().map(|u| 0.55 + 1.5 * u * u).collect();
        let spacing = 0.85 / n as f64;
        let mut kept: Vec<usize> = Vec::new();
        for i in 0..tries {
            let ok = kept.iter().all(|&j| {
                let (dx, dy) = ((x[i] - x[j]).abs(), (y[i] - y[j]).abs());
                let (dx, dy) = (dx.min(1.0 - dx), dy.min(1.0 - dy));
                let reach = spacing * 0.5 * (factor[i] + factor[j]);
                dx * dx + dy * dy >= reach * reach
            });
            if ok {
                kept.push(i);
            }
        }
        let count = kept.len();
        let total = size * size;
        let sz = size as f64;
        let (wx, wy) = match warp {
            Some((amount, lo, hi)) => {
                let k = amount * sz / n as f64;
                (scaled(&band_noise(&mut own, size, lo, hi, 1.0), k), scaled(&band_noise(&mut own, size, lo, hi, 1.0), k))
            }
            None => (vec![0.0; total], vec![0.0; total]),
        };
        let (mut d1, mut d2) = (vec![f64::INFINITY; total], vec![f64::INFINITY; total]);
        let (mut i1, mut i2) = (vec![0usize; total], vec![0usize; total]);
        let (mut ox, mut oy) = (vec![0.0; total], vec![0.0; total]);
        for (k, &seed) in kept.iter().enumerate() {
            let (sx, sy) = (x[seed] * sz, y[seed] * sz);
            for py in 0..size {
                for px in 0..size {
                    let at = py * size + px;
                    let dy0 = py as f64 + 0.5 + wy[at] - sy;
                    let dy = dy0 - sz * (dy0 / sz + 0.5).floor();
                    let dx0 = px as f64 + 0.5 + wx[at] - sx;
                    let dx = dx0 - sz * (dx0 / sz + 0.5).floor();
                    let d = dx * dx + dy * dy;
                    if d < d1[at] {
                        d2[at] = d1[at];
                        i2[at] = i1[at];
                        ox[at] = dx;
                        oy[at] = dy;
                        d1[at] = d;
                        i1[at] = k;
                    } else if d < d2[at] {
                        d2[at] = d;
                        i2[at] = k;
                    }
                }
            }
        }
        let cells = Cells { size, stride: count, n: count, f1: d1.iter().map(|d| d.sqrt()).collect(), f2: d2.iter().map(|d| d.sqrt()).collect(), i1, i2, dx: ox, dy: oy };
        (cells, own)
    }

    /// Distance to the nearest cell edge, in pixels.
    fn edge(&self, at: usize) -> f64 {
        (self.f2[at] - self.f1[at]) / 2.0
    }

    /// A number in [0, 1) per edge between two cells, the same on both sides.
    fn pair(&self, at: usize, salt: u64) -> f64 {
        let (a, b) = (self.i1[at].min(self.i2[at]), self.i1[at].max(self.i2[at]));
        hash_uniform((a * self.stride + b) as u64, salt)
    }
}

/// The cracks' widths, 0.6 to 1.4 times (`cracks` spread: lo, span, power).
const CRACK_SPREAD: (f64, f64, f64) = (0.6, 0.8, 1.0);

/// Cracks along the cells' edges: a trough `depth` deep and `width` wide (m) with a raised lip
/// `lip` high at the shoulders, along `share` of the edges, each with its own width: `spread`
/// (lo, span, power), lo + span u ^ power times.
fn cracks(cells: &Cells, metres: f64, share: f64, width: f64, depth: f64, lip: f64, salt: u64, spread: (f64, f64, f64)) -> Grid {
    let (lo, span, power) = spread;
    (0..cells.size * cells.size)
        .map(|at| {
            let e = cells.edge(at) * metres / cells.size as f64;
            if cells.pair(at, salt) < share {
                let wide = width * (lo + span * cells.pair(at, salt + 1).powf(power));
                let trough = depth * (-(e / wide).powi(2)).exp();
                let shoulder = lip * (-((e - 1.8 * wide) / (0.9 * wide)).powi(2)).exp();
                f32r(shoulder - trough)
            } else {
                0.0
            }
        })
        .collect()
}

/// Wind ripples, crests across x, `waves` per tile: a gentle windward and a steep lee side,
/// meandering by `sinuous` wavelengths, dying out in patches. Height in metres.
fn ripples(rng: &mut Rng, size: usize, waves: f64, sinuous: f64, height: f64, lee: f64) -> Grid {
    let warp = band_noise(rng, size, 1.0, 4.0, 1.0);
    let dies = band_noise(rng, size, 1.0, 3.0, 1.0);
    (0..size * size)
        .map(|at| {
            let x = (at % size) as f64 / size as f64;
            let phase = (waves * x + sinuous * warp[at]).rem_euclid(1.0);
            let tri = if phase < 1.0 - lee { phase / (1.0 - lee) } else { (1.0 - phase) / lee };
            let profile = 0.5 - 0.5 * (PI * tri).cos();
            f32r(profile * clip(0.75 + 0.5 * dies[at], 0.0, 1.0) * height)
        })
        .collect()
}

/// Pahoehoe ropes: arcs round a point behind each lobe (cells of n x n), `wavelength` metres
/// apart; a third of the lobes are smooth. Height in metres.
fn ropes(rng: &mut Rng, size: usize, metres: f64, n: usize, wavelength: (f64, f64), height: f64) -> Grid {
    let cells = Cells::new(rng, size, n);
    let per = n * n;
    let angle: Vec<f64> = rng.uniform(per).iter().map(|u| u * 2.0 * PI).collect();
    let behind: Vec<f64> = rng.uniform(per).iter().map(|u| 0.4 + 1.2 * u).collect();
    let length: Vec<f64> = rng.uniform(per).iter().map(|u| (wavelength.0 + (wavelength.1 - wavelength.0) * u) * size as f64 / metres).collect();
    let smooth = rng.uniform(per);
    let strength = rng.uniform(per);
    let amount: Vec<f64> = (0..per).map(|i| if smooth[i] < 0.33 { 0.0 } else { 0.5 + 0.5 * strength[i] }).collect();
    let cell_px = size as f64 / n as f64;
    (0..size * size)
        .map(|at| {
            let c = cells.i1[at];
            let (cx, cy) = (angle[c].cos() * behind[c] * cell_px, angle[c].sin() * behind[c] * cell_px);
            let phase = (cells.dx[at] - cx).hypot(cells.dy[at] - cy) / length[c];
            let fade = clip(cells.edge(at) / (0.2 * cell_px), 0.0, 1.0);
            (0.5 - 0.5 * (2.0 * PI * phase).cos()) * amount[c] * fade * height
        })
        .collect()
}

/// The height's slopes along x and y (metres per metre), wrapping round, as 0..1.
fn slopes(height: &[f64], size: usize, metres: f64) -> (Grid, Grid) {
    let step = 2.0 * metres / size as f64;
    let at = |y: usize, x: usize| height[(y % size) * size + x % size];
    let mut sx = vec![0.0; size * size];
    let mut sy = vec![0.0; size * size];
    for y in 0..size {
        for x in 0..size {
            let (gx, gy) = ((at(y, x + 1) - at(y, x + size - 1)) / step, (at(y + 1, x) - at(y + size - 1, x)) / step);
            // Flat is 128 of 255, the ends 1 and 255, as `ground_detail_tex.py` `slopes`.
            sx[y * size + x] = clip((128.0 + 127.0 * gx / SLOPE_MAX) / 255.0, 1.0 / 255.0, 1.0);
            sy[y * size + x] = clip((128.0 + 127.0 * gy / SLOPE_MAX) / 255.0, 1.0 / 255.0, 1.0);
        }
    }
    (sx, sy)
}

// ------------------------------------------------------------------ the snow's height map

/// The snow's relief (`ground_tint.py` `SNOW_RELIEF`): one tile of 4 m of rough fractal snow, hex-tiled on the chunk grid by the shader;
/// the script's `snow_height` has the reasons for every number. Arithmetic only (no sin, no cos), so the two make the same numbers.
const SNOW_TILE_M: f64 = 4.0;
const SNOW_OCTAVES: usize = 8;
const SNOW_GAIN: f64 = 0.65;
const SNOW_AMP: f64 = 0.050;
const SNOW_WARP: f64 = 0.3;
const SNOW_TURBULENT: usize = 2;
const SNOW_BOOST: [f64; 8] = [0.1, 0.5, 2.0, 3.4, 2.2, 0.9, 0.6, 0.6];
const SNOW_CRUST: (f64, f64, f64, f64) = (0.025, 0.006, 0.40, 0.025);
const SNOW_HEIGHT_RANGE: f64 = 0.16;
const SNOW_SLOPE_MAX: f64 = 1.5;
const SNOW_TURB_MEAN: f64 = 0.62;
// The churned snow (the script's `churn_height` has the reasons): rounded hollows and heaps painted onto 0, height only.
const CHURN_HOLLOWS: f64 = 82.0;
const CHURN_RADIUS: (f64, f64) = (0.040, 0.150);
const CHURN_DEPTH: (f64, f64) = (0.020, 0.060);
const CHURN_STEEP: f64 = 0.25;
const CHURN_ASPECT: (f64, f64) = (0.6, 1.0);
const CHURN_WOBBLE: f64 = 0.25;
const CHURN_DISH: f64 = 0.30;
const CHURN_HEAPS: f64 = 1.2;
const CHURN_HEAP_AT: (f64, f64) = (0.85, 1.25);
const CHURN_HEAP_SIZE: (f64, f64) = (0.35, 0.70);
const CHURN_HEAP_HEIGHT: (f64, f64) = (0.27, 0.67);
const CHURN_FREE: f64 = 25.0;
const CHURN_FREE_RADIUS: (f64, f64) = (0.040, 0.120);
const CHURN_FREE_HEIGHT: (f64, f64) = (0.017, 0.040);
const CHURN_SOFT: f64 = 0.07;
// cos and sin of sixteen angles as computed, kept to the digit: the golden texture test
// pins the output, so the standard constants (a last digit apart) are not used here.
#[allow(clippy::approx_constant)]
const GRADIENTS: [(f64, f64); 16] = [
    (1.0, 0.0),
    (0.9238795325112867, 0.3826834323650898),
    (0.7071067811865476, 0.7071067811865475),
    (0.38268343236508984, 0.9238795325112867),
    (6.123233995736766e-17, 1.0),
    (-0.3826834323650897, 0.9238795325112867),
    (-0.7071067811865475, 0.7071067811865476),
    (-0.9238795325112867, 0.3826834323650899),
    (-1.0, 1.2246467991473532e-16),
    (-0.9238795325112868, -0.3826834323650895),
    (-0.7071067811865477, -0.7071067811865475),
    (-0.38268343236508995, -0.9238795325112866),
    (-1.8369701987210297e-16, -1.0),
    (0.38268343236508945, -0.9238795325112868),
    (0.7071067811865474, -0.7071067811865477),
    (0.9238795325112865, -0.3826834323650904),
];

/// Periodic gradient noise (`gradient_noise` in the script) of the tile coordinates `px`, `py` (0..1 round the tile, any number: it wraps),
/// `cells` lattice cells a tile: about -1 to 1.
fn gradient_noise(px: &[f64], py: &[f64], cells: usize, salt: u64) -> Grid {
    let table: Vec<(f64, f64)> = (0..cells * cells)
        .map(|i| {
            let (row, col) = (i / cells, i % cells);
            let u = hash_uniform((row * 1_048_576 + col) as u64, salt);
            GRADIENTS[((u * 16.0) as usize).min(15)]
        })
        .collect();
    let fade = |t: f64| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let n = cells as f64;
    px.iter()
        .zip(py)
        .map(|(&px, &py)| {
            let (x, y) = (px * n, py * n);
            let (ix, iy) = (x.floor(), y.floor());
            let (fx, fy) = (x - ix, y - iy);
            let (ix, iy) = (ix as i64, iy as i64);
            let corner = |dx: i64, dy: i64| {
                let g = table[(iy + dy).rem_euclid(cells as i64) as usize * cells + (ix + dx).rem_euclid(cells as i64) as usize];
                g.0 * (fx - dx as f64) + g.1 * (fy - dy as f64)
            };
            let (wx, wy) = (fade(fx), fade(fy));
            let (c00, c10, c01, c11) = (corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1));
            let top = c00 + (c10 - c00) * wx;
            let bottom = c01 + (c11 - c01) * wx;
            (top + (bottom - top) * wy) * std::f64::consts::SQRT_2
        })
        .collect()
}

/// The snow's height map (metres, `size` x `size` over `SNOW_TILE_M`, periodic): the script's `snow_height`.
fn snow_height(size: usize) -> Grid {
    let axis: Vec<f64> = (0..size).map(|i| (i as f64 + 0.5) / size as f64).collect();
    let px: Grid = (0..size * size).map(|i| axis[i % size]).collect();
    let py: Grid = (0..size * size).map(|i| axis[i / size]).collect();
    let mut warps = Vec::new();
    for k in 0..(SNOW_OCTAVES - 2).max(1) {
        let cells = 4usize << k;
        warps.push((gradient_noise(&px, &py, cells, 9000 + 2 * k as u64), gradient_noise(&px, &py, cells, 9001 + 2 * k as u64)));
    }
    let amps: Vec<f64> = (0..SNOW_OCTAVES).map(|k| SNOW_AMP * SNOW_GAIN.powi(k as i32) * SNOW_BOOST[k]).collect();
    let mut octaves: Vec<Grid> = Vec::new();
    for k in 0..SNOW_OCTAVES {
        let cells = 4usize << k;
        let (wx, wy) = &warps[k.saturating_sub(2)];
        let shift = SNOW_WARP / cells as f64;
        let sx: Grid = (0..size * size).map(|i| px[i] + shift * wx[i]).collect();
        let sy: Grid = (0..size * size).map(|i| py[i] + shift * wy[i]).collect();
        let n = gradient_noise(&sx, &sy, cells, 7000 + k as u64);
        octaves.push(if k < SNOW_TURBULENT { n } else { n.iter().map(|v| SNOW_TURB_MEAN - v.abs()).collect() });
    }
    let (lo, width, keep, step) = SNOW_CRUST;
    (0..size * size)
        .map(|i| {
            let smooth = octaves[0][i] * amps[0] + octaves[1][i] * amps[1];
            let mut fine = 0.0;
            for k in 2..SNOW_OCTAVES {
                fine += octaves[k][i] * amps[k];
            }
            let mid = octaves[1][i] * amps[1] + octaves[2][i] * amps[2] + octaves[3][i] * amps[3];
            let t = clip((mid - lo) / width, 0.0, 1.0);
            let flat = t * t * (3.0 - 2.0 * t);
            smooth + fine * (1.0 - (1.0 - keep) * flat) + step * flat
        })
        .collect()
}

fn smooth01(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The 48 hashed 0..1 numbers of stamp `index` (the script's `hashes`).
fn stamp_hashes(index: u64, salt: u64) -> Vec<f64> {
    (0..48).map(|k| hash_uniform(index * 64 + k, salt)).collect()
}

/// The unit vector of two hashed numbers (the script's `unit`).
fn unit_vector(a: f64, b: f64) -> (f64, f64) {
    let (ox, oy) = (2.0 * a - 1.0, 2.0 * b - 1.0);
    let norm = (ox * ox + oy * oy).powf(0.5);
    if norm > 1e-9 {
        (ox / norm, oy / norm)
    } else {
        (1.0, 0.0)
    }
}

/// The texels round a stamp's centre (metres): calls `f(index in the grid, dx, dy)` with the offsets from the centre (the grid is periodic).
fn for_window(size: usize, cx: f64, cy: f64, reach: f64, mut f: impl FnMut(usize, f64, f64)) {
    let texel = SNOW_TILE_M / size as f64;
    let (ix, iy) = ((cx / texel).floor() as i64, (cy / texel).floor() as i64);
    let r = (reach / texel).ceil() as i64;
    for y in iy - r..=iy + r {
        for x in ix - r..=ix + r {
            let dx = (x as f64 + 0.5) * texel - cx;
            let dy = (y as f64 + 0.5) * texel - cy;
            f((y.rem_euclid(size as i64) as usize) * size + x.rem_euclid(size as i64) as usize, dx, dy);
        }
    }
}

/// The elliptical radius (1 at the outline) of an offset, the outline wobbling (the script's `radius_of`).
fn churn_radius(dx: f64, dy: f64, r: f64, aspect: f64, o: (f64, f64), wobble: &[f64; 4]) -> f64 {
    let (nu, nv) = ((dx * o.0 + dy * o.1) / r, (-dx * o.1 + dy * o.0) / (r * aspect));
    let rho = (nu * nu + nv * nv).sqrt();
    let safe = rho.max(1e-12);
    let (c, sn) = (nu / safe, nv / safe);
    rho / (1.0 + CHURN_WOBBLE * (wobble[0] * c + wobble[1] * sn + wobble[2] * (c * c - sn * sn) + wobble[3] * 2.0 * c * sn))
}

/// The churned snow's height (metres, `size` x `size` over `SNOW_TILE_M`, periodic): the script's `churn_height`.
fn churn_height(size: usize) -> Grid {
    let mut h = vec![0.0; size * size];
    let area = SNOW_TILE_M * SNOW_TILE_M;
    let hollows = (CHURN_HOLLOWS * area) as u64;
    let free = (CHURN_FREE * area) as u64;
    let mut items: Vec<(f64, usize, u64)> = Vec::new();
    for k in 0..hollows {
        items.push((hash_uniform(k, 8999), 0, k));
    }
    for k in 0..hollows {
        items.push((hash_uniform(hollows + k, 8999), 1, k));
    }
    for k in 0..free {
        items.push((hash_uniform(2 * hollows + k, 8999), 2, k));
    }
    items.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let wobbles = |u: &[f64]| [2.0 * u[5] - 1.0, 2.0 * u[6] - 1.0, 2.0 * u[7] - 1.0, 2.0 * u[8] - 1.0];
    let bell = |h: &mut Grid, cx: f64, cy: f64, r: f64, aspect: f64, o: (f64, f64), wobble: &[f64; 4], height: f64| {
        for_window(size, cx, cy, r * (1.0 + CHURN_WOBBLE), |i, dx, dy| {
            let rho = churn_radius(dx, dy, r, aspect, o, wobble);
            let b = (1.0 - rho * rho).clamp(0.0, 1.0);
            h[i] += height * b * b;
        });
    };
    for (_, kind, index) in items {
        if kind == 2 {
            let u = stamp_hashes(index, 8501);
            let r = CHURN_FREE_RADIUS.0 + (CHURN_FREE_RADIUS.1 - CHURN_FREE_RADIUS.0) * u[0] * u[0];
            let height = (CHURN_FREE_HEIGHT.0 + (CHURN_FREE_HEIGHT.1 - CHURN_FREE_HEIGHT.0) * u[1]).min(CHURN_STEEP * r);
            let aspect = CHURN_ASPECT.0 + (CHURN_ASPECT.1 - CHURN_ASPECT.0) * u[2];
            bell(&mut h, u[9] * SNOW_TILE_M, u[10] * SNOW_TILE_M, r, aspect, unit_vector(u[3], u[4]), &wobbles(&u), height);
            continue;
        }
        let u = stamp_hashes(index, 8401);
        let r = CHURN_RADIUS.0 + (CHURN_RADIUS.1 - CHURN_RADIUS.0) * u[0] * u[0];
        let t = (r - CHURN_RADIUS.0) / (CHURN_RADIUS.1 - CHURN_RADIUS.0);
        let depth = (CHURN_DEPTH.0 + (CHURN_DEPTH.1 - CHURN_DEPTH.0) * t * (0.7 + 0.6 * u[1])).min(CHURN_STEEP * r);
        let aspect = CHURN_ASPECT.0 + (CHURN_ASPECT.1 - CHURN_ASPECT.0) * u[2];
        let o = unit_vector(u[3], u[4]);
        let wobble = wobbles(&u);
        let (cx, cy) = (u[9] * SNOW_TILE_M, u[10] * SNOW_TILE_M);
        if kind == 0 {
            for_window(size, cx, cy, r * (1.0 + CHURN_WOBBLE), |i, dx, dy| {
                let rho = churn_radius(dx, dy, r, aspect, o, &wobble);
                let inside = rho.min(1.0);
                h[i] -= depth * (1.0 - smooth01(rho)) * (1.0 + CHURN_DISH * (1.0 - inside * inside));
            });
            continue;
        }
        for j in 0..2 {
            if u[12 + j] >= CHURN_HEAPS / 2.0 {
                continue;
            }
            let (ax, ay) = unit_vector(u[14 + 2 * j], u[15 + 2 * j]);
            let dist = r * (CHURN_HEAP_AT.0 + (CHURN_HEAP_AT.1 - CHURN_HEAP_AT.0) * u[18 + j]);
            let hr = r * (CHURN_HEAP_SIZE.0 + (CHURN_HEAP_SIZE.1 - CHURN_HEAP_SIZE.0) * u[20 + j]);
            let height = (depth * (CHURN_HEAP_HEIGHT.0 + (CHURN_HEAP_HEIGHT.1 - CHURN_HEAP_HEIGHT.0) * u[22 + j])).min(CHURN_STEEP * hr);
            let ha = CHURN_ASPECT.0 + (CHURN_ASPECT.1 - CHURN_ASPECT.0) * u[24 + j];
            bell(&mut h, cx + ax * dist, cy + ay * dist, hr, ha, (-ay, ax), &wobble, height);
        }
    }
    h.iter().map(|v| v / (1.0 + v.abs() / CHURN_SOFT)).collect()
}

/// The snow map's planes (8 bits): the fractal's height (`SNOW_HEIGHT_RANGE`, 128 level), the slopes along x and y (`SNOW_SLOPE_MAX`) of the fractal plus the churned snow, and
/// the churn's height (the script's `snow_planes`: sh, s0, s1, ch).
pub fn snow_planes(size: usize) -> Vec<Vec<u8>> {
    let mut h = snow_height(size);
    let m = mean(&h);
    h.iter_mut().for_each(|v| *v -= m);
    let churn = churn_height(size);
    let total: Grid = h.iter().zip(&churn).map(|(a, b)| a + b).collect();
    let step = 2.0 * SNOW_TILE_M / size as f64;
    let enc = |s: f64| clip((128.0 + 127.0 * s / SNOW_SLOPE_MAX) / 255.0, 1.0 / 255.0, 1.0);
    let at = |x: usize, y: usize| total[(y % size) * size + x % size];
    let sx: Grid = (0..size * size).map(|i| enc((at(i % size + 1, i / size) - at(i % size + size - 1, i / size)) / step)).collect();
    let sy: Grid = (0..size * size).map(|i| enc((at(i % size, i / size + 1) - at(i % size, i / size + size - 1)) / step)).collect();
    let level = |t: &[f64]| -> Grid { t.iter().map(|v| clip(0.5 + v / SNOW_HEIGHT_RANGE, 0.0, 1.0)).collect() };
    vec![to_u8(&level(&h)), to_u8(&sx), to_u8(&sy), to_u8(&level(&churn))]
}

// ------------------------------------------------------------------ the looks

/// What a look makes: the colour channels, the two layers' heights (m), the stones' shade
/// (positive = darker) and height (m).
struct Look {
    colour: [Grid; 4],
    relief2: Grid,
    relief8: Grid,
    shade: Grid,
    height: Grid,
}

fn speckle(rng: &mut Rng, size: usize) -> Grid {
    to_unit(&band_noise(rng, size, 60.0, (size / 2) as f64, 1.0), 2.0)
}

fn patches(rng: &mut Rng, size: usize) -> Grid {
    to_unit(&band_noise(rng, size, 1.0, 6.0, 1.0), 2.5)
}

fn random_places(rng: &mut Rng, size: usize, count: usize) -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = rng.uniform(count).iter().map(|u| u * size as f64).collect();
    let y: Vec<f64> = rng.uniform(count).iter().map(|u| u * size as f64).collect();
    (x, y)
}

fn radii(rng: &mut Rng, count: usize, base: f64, spread: f64, power: f64) -> Vec<f64> {
    rng.uniform(count).iter().map(|u| base + spread * u.powf(power)).collect()
}

/// Mars-like wind-blown sand: two sizes of ripples, granules, a pebble lag.
fn sand(rng: &mut Rng, size: usize, psize: usize) -> Look {
    let patches = patches(rng, size);
    let (px, py) = random_places(rng, size, 2500);
    let radius = radii(rng, 2500, 0.002, 0.003, 3.0);
    let tone = tones(rng, 2500, 0.7, 0.9);
    let (granules, _) = stones(rng, size, SMALL, &px, &py, &radius, &tone, None, &FACETED);
    let impact = ripples(rng, size, 25.0, 0.15, 0.005, 0.25);
    let drag = ripples(rng, size, 22.0, 0.3, 0.02, 0.3);
    let speckle = speckle(rng, size);
    // Granules darken, coarse grains on the impact ripples' crests a little; mean back to 0.5.
    let g: Grid = (0..size * size).map(|i| 0.5 - 0.3 * granules[i] - 6.0 * impact[i]).collect();
    let shift = 0.5 - mean(&g);
    let g: Grid = g.iter().map(|v| clip(v + shift, 0.0, 1.0)).collect();
    let b = to_unit(&scaled(&drag, -1.0), 3.0);
    let (qx, qy) = grid_places(rng, psize, 900);
    let radius = radii(rng, qx.len(), 0.008, 0.032, 3.0);
    let tone = tones(rng, qx.len(), 0.7, 0.9);
    let (shade, height) = stones(rng, psize, PEBBLES, &qx, &qy, &radius, &tone, None, &FACETED);
    Look { colour: [patches, g, b, speckle], relief2: impact, relief8: drag, shade, height }
}

/// The Moon's regolith: craters of all sizes, old ones soft, fresh ones sharp and bright, rocks
/// around the young ones.
fn regolith(rng: &mut Rng, size: usize, psize: usize) -> Look {
    let spec = |d| CraterSpec { depth: 0.12, rim: 0.025, bowl: 1.3, ..CraterSpec::new(d) };
    let (h2, bright2) = craters(rng, size, SMALL, 500, &spec((0.02, 0.08)), None);
    let (h8, bright8) = craters(rng, size, LARGE, 700, &spec((0.08, 0.30)), None);
    let big = 40;
    let cx: Vec<f64> = rng.uniform(big).iter().map(|u| u * psize as f64).collect();
    let cy: Vec<f64> = rng.uniform(big).iter().map(|u| u * psize as f64).collect();
    let (hp, brightp) = craters(rng, psize, PEBBLES, big, &spec((0.3, 2.0)), Some((&cx, &cy)));
    let rocks = 420;
    let near = (rocks as f64 * 0.6) as usize;
    let owner = rng.below(near, big);
    let ring: Vec<f64> = rng.uniform(near).iter().map(|u| 1.2 + 4.0 * u.powi(2)).collect();
    let theta: Vec<f64> = rng.uniform(near).iter().map(|u| u * 2.0 * PI).collect();
    let typical = psize as f64 / PEBBLES;
    let (gx, gy) = grid_places(rng, psize, rocks - near);
    let m = psize as f64;
    let qx: Vec<f64> = (0..near).map(|i| (cx[owner[i]] + ring[i] * typical * theta[i].cos()).rem_euclid(m)).chain(gx.iter().map(|v| v.rem_euclid(m))).collect();
    let qy: Vec<f64> = (0..near).map(|i| (cy[owner[i]] + ring[i] * typical * theta[i].sin()).rem_euclid(m)).chain(gy.iter().map(|v| v.rem_euclid(m))).collect();
    let radius = power_radii(rng, qx.len(), 0.02, 0.22, 3.0);
    let tone = tones(rng, qx.len(), 0.2, 0.6);
    let (shade, rock_h) = stones(rng, psize, PEBBLES, &qx, &qy, &radius, &tone, None, &Shape { ridge: 0.6, ..FACETED });
    let shade: Grid = shade.iter().zip(&brightp).map(|(s, b)| clip(s - 0.5 * b, -0.6, 1.0)).collect();
    let patches = centred(&patches(rng, size), 0.7);
    let speckle = speckle(rng, size);
    let g = centred(&lin(&bright2, 1.0, &cavity(&h2, size, SMALL, 0.05), -0.1), 0.45);
    let b = centred(&lin(&bright8, 1.0, &cavity(&h8, size, LARGE, 0.3), -0.1), 0.45);
    Look { colour: [patches, g, b, speckle], relief2: h2, relief8: h8, shade, height: add(&hp, &rock_h) }
}

/// Airless ice: thermal-contraction polygons with raised lips, fine crazing, sintered grain,
/// bright blocks and fresh craters.
/// The ice's crack networks of the 8 m and 2 m layers (ground_detail_tex.py): (nominal cells per tile, share,
/// warp (amplitude in cells, lo, hi in cycles per tile), spread).
const ICE_CRACKS8: (usize, f64, (f64, f64, f64), (f64, f64, f64)) = (18, 0.4, (0.08, 3.0, 18.0), (0.3, 1.4, 2.0));
const ICE_CRACKS2: (usize, f64, (f64, f64, f64), (f64, f64, f64)) = (12, 0.25, (0.08, 2.0, 12.0), (0.3, 1.4, 2.0));

fn ice(rng: &mut Rng, size: usize, psize: usize) -> Look {
    // The 8 m and 2 m layers' crack networks (ground_detail_tex.py ICE_CRACKS8, ICE_CRACKS2): scattered cells, the edges
    // bent by the warp, fewer cracked, the widths spread wider.
    let (n, share8, warp, spread8) = ICE_CRACKS8;
    let cells8 = Cells::scattered(rng, size, n, Some(warp)).0;
    let broad = band_noise(rng, size, 1.0, 3.0, 1.0);
    let h8 = lin(&cracks(&cells8, LARGE, share8, 0.025, 0.006, 0.003, 11, spread8), 1.0, &broad, 0.006);
    let (n, share2, warp, spread2) = ICE_CRACKS2;
    let cells2 = Cells::scattered(rng, size, n, Some(warp)).0;
    let (qx, qy) = random_places(rng, size, 2200);
    let radius = radii(rng, 2200, 0.002, 0.003, 3.0);
    let tone = tones(rng, 2200, 0.1, 0.3);
    let (grains, grain_h) = stones(rng, size, SMALL, &qx, &qy, &radius, &tone, None, &FACETED);
    let h2 = lin(&cracks(&cells2, SMALL, share2, 0.012, 0.0022, 0.0009, 23, spread2), 1.0, &grain_h, 0.3);
    let big = 6;
    let cx: Vec<f64> = rng.uniform(big).iter().map(|u| u * psize as f64).collect();
    let cy: Vec<f64> = rng.uniform(big).iter().map(|u| u * psize as f64).collect();
    let spec = CraterSpec { exponent: 1.5, depth: 0.1, rim: 0.02, age: (0.3, 0.8), ..CraterSpec::new((0.5, 1.6)) };
    let (hp, brightp) = craters(rng, psize, PEBBLES, big, &spec, Some((&cx, &cy)));
    let polygons = cracks(&Cells::scattered(rng, psize, 6, None).0, PEBBLES, 0.8, 0.04, 0.012, 0.006, 41, CRACK_SPREAD);
    let hp = add(&hp, &polygons);
    let (gx, gy) = grid_places(rng, psize, 260);
    let radius = power_radii(rng, gx.len(), 0.03, 0.22, 2.5);
    let tone = tones(rng, gx.len(), 0.1, 0.25);
    let (shade, block_h) = stones(rng, psize, PEBBLES, &gx, &gy, &radius, &tone, None, &Shape { ridge: 0.45, crown: 2.0, ..FACETED });
    let shade: Grid = shade.iter().zip(&brightp).map(|(s, b)| clip(s - 0.4 * b, -0.6, 1.0)).collect();
    let patches = centred(&patches(rng, size), 0.9);
    let speckle = speckle(rng, size);
    let g = centred(&lin(&grains, 0.3, &cavity(&h2, size, SMALL, 0.04), -0.5), 0.35);
    let b = centred(&scaled(&cavity(&h8, size, LARGE, 0.2), -0.5), 0.35);
    let height = add(&hp, &block_h);
    Look { colour: [patches, g, b, speckle], relief2: h2, relief8: h8, shade, height }
}

/// Nitrogen and methane frost: pits of all sizes without rims, bright and smooth.
fn frost(rng: &mut Rng, size: usize, psize: usize) -> Look {
    let pits = |d, exponent, depth| CraterSpec { exponent, depth, rim: 0.0, ejecta: false, age: (0.5, 1.0), bowl: 1.5, ..CraterSpec::new(d) };
    let (h8, deep8) = craters(rng, size, LARGE, 120, &pits((0.35, 1.8), 1.2, 0.05), None);
    let (h2, _) = craters(rng, size, SMALL, 450, &pits((0.04, 0.22), 1.5, 0.07), None);
    let (qx, qy) = random_places(rng, size, 1800);
    let radius = radii(rng, 1800, 0.0015, 0.002, 3.0);
    let tone = tones(rng, 1800, 0.05, 0.1);
    let (grains, grain_h) = stones(rng, size, SMALL, &qx, &qy, &radius, &tone, None, &Shape { ridge: 0.4, crown: 2.0, ..FACETED });
    let h2 = lin(&h2, 1.0, &grain_h, 0.25);
    let patches = centred(&patches(rng, size), 0.9);
    let speckle = speckle(rng, size);
    let g = centred(&lin(&grains, 0.2, &cavity(&h2, size, SMALL, 0.05), -0.15), 0.35);
    let b = centred(&scaled(&deep8, -0.8), 0.3);
    let flat = vec![0.0; psize * psize];
    Look { colour: [patches, g, b, speckle], relief2: h2, relief8: h8, shade: flat.clone(), height: flat }
}

/// Dusty ice (comet 67P): a dark lag over bright ice, soft knobs, pits, boulders of all sizes,
/// some buried in dust.
fn rocky_ice(rng: &mut Rng, size: usize, psize: usize) -> Look {
    let (qx, qy) = random_places(rng, size, 200);
    let radius = radii(rng, 200, 0.1, 0.4, 2.0);
    let tone = tones(rng, 200, 0.6, 0.9);
    let bury: Vec<f64> = rng.uniform(200).iter().map(|u| 0.15 + 0.25 * u).collect();
    let (knobs, knob_h) = stones(rng, size, LARGE, &qx, &qy, &radius, &tone, Some(&bury), &Shape { ridge: 0.4, crown: 2.5, faces: (7, 8) });
    let spec = CraterSpec { exponent: 1.5, depth: 0.1, rim: 0.01, ejecta: false, bowl: 1.5, ..CraterSpec::new((0.3, 1.2)) };
    let (pits, _) = craters(rng, size, LARGE, 40, &spec, None);
    let h8 = add(&knob_h, &pits);
    let (sx, sy) = random_places(rng, size, 700);
    let radius = radii(rng, 700, 0.008, 0.03, 2.5);
    let tone = tones(rng, 700, 0.7, 0.9);
    let bury: Vec<f64> = rng.uniform(700).iter().map(|u| 0.4 + 0.4 * u).collect();
    let (lumps, h2) = stones(rng, size, SMALL, &sx, &sy, &radius, &tone, Some(&bury), &Shape { ridge: 0.45, crown: 2.5, faces: (7, 8) });
    let (gx, gy) = grid_places(rng, psize, 320);
    let n = gx.len();
    let radius = power_radii(rng, n, 0.012, 0.25, 2.5);
    let tone = tones(rng, n, 0.75, 0.95);
    let bury: Vec<f64> = rng.uniform(n).iter().map(|u| 0.3 + 0.7 * u).collect();
    let (shade, height) = stones(rng, psize, PEBBLES, &gx, &gy, &radius, &tone, Some(&bury), &Shape { ridge: 0.4, crown: 2.2, faces: (6, 8) });
    let patches = centred(&patches(rng, size), 1.0);
    let speckle = speckle(rng, size);
    let g = centred(&lin(&lumps, -0.3, &cavity(&h2, size, SMALL, 0.05), -0.2), 0.5);
    let b = centred(&lin(&knobs, -0.3, &cavity(&h8, size, LARGE, 0.3), -0.2), 0.5);
    Look { colour: [patches, g, b, speckle], relief2: h2, relief8: h8, shade, height }
}

/// A field of tilted slabs on `n` x n cells, `metres` wide on a grid of `grid` pixels, with
/// cracks between them: the heights and each slab's tone (-0.5..0.5) as a picture.
#[allow(clippy::too_many_arguments)]
fn slabs(rng: &mut Rng, n: usize, tilt: f64, lift: f64, metres: f64, grid: usize, depth: f64, salt: u64) -> (Grid, Grid) {
    let (cells, mut own) = Cells::scattered(rng, grid, n, None);
    let per = cells.n;
    let tx: Vec<f64> = own.uniform(per).iter().map(|u| tilt * (u - 0.5)).collect();
    let ty: Vec<f64> = own.uniform(per).iter().map(|u| tilt * (u - 0.5)).collect();
    let dz: Vec<f64> = own.uniform(per).iter().map(|u| lift * (u - 0.5)).collect();
    let tone: Vec<f64> = own.uniform(per).iter().map(|u| u - 0.5).collect();
    rng.n += 4 * (n * n) as u64;
    let px = metres / grid as f64;
    let cracked = cracks(&cells, metres, 0.85, 0.045, depth, depth / 2.0, salt, CRACK_SPREAD);
    let height = (0..grid * grid)
        .map(|at| {
            let c = cells.i1[at];
            f32r(tx[c] * cells.dx[at] * px + ty[c] * cells.dy[at] * px + dz[c] + cracked[at])
        })
        .collect();
    (height, (0..grid * grid).map(|at| tone[cells.i1[at]]).collect())
}

/// Fresh basalt: crust broken into tilted slabs with cooling cracks, ropy folds, vesicles,
/// dark blocks.
fn lava(rng: &mut Rng, size: usize, psize: usize) -> Look {
    let (h8, tone8) = slabs(rng, 14, 0.12, 0.012, LARGE, size, 0.01, 31);
    let folds = ropes(rng, size, SMALL, 6, (0.04, 0.07), 0.007);
    let spec = CraterSpec { exponent: 1.5, depth: 0.5, rim: 0.0, ejecta: false, age: (0.7, 1.0), bowl: 1.5, ..CraterSpec::new((0.006, 0.03)) };
    let (vesicles, _) = craters(rng, size, SMALL, 520, &spec, None);
    let h2 = add(&folds, &vesicles);
    let (gx, gy) = grid_places(rng, psize, 260);
    let radius = power_radii(rng, gx.len(), 0.03, 0.25, 2.5);
    let tone = tones(rng, gx.len(), 0.92, 0.98);
    let (shade, height) = stones(rng, psize, PEBBLES, &gx, &gy, &radius, &tone, None, &Shape { ridge: 0.35, crown: 2.0, ..FACETED });
    let (big, big_tone) = slabs(rng, 6, 0.2, 0.03, PEBBLES, psize, 0.014, 37);
    let height = add(&height, &big);
    let shade = lin(&shade, 1.0, &big_tone, 0.5);
    let patches = centred(&patches(rng, size), 0.9);
    let speckle = speckle(rng, size);
    let g = centred(&scaled(&cavity(&h2, size, SMALL, 0.03), -0.5), 0.3);
    let b = centred(&lin(&tone8, 0.8, &cavity(&h8, size, LARGE, 0.1), -0.6), 0.35);
    Look { colour: [patches, g, b, speckle], relief2: h2, relief8: h8, shade, height }
}

fn to_u8(v: &[f64]) -> Vec<u8> {
    v.iter().map(|x| (x * 255.0).round_ties_even().clamp(0.0, 255.0) as u8).collect()
}

/// The planes (8 bits, see `PLANES`) of ground type `index`: `size` px for the colour and
/// relief, `psize` for the stones.
pub fn generate(index: usize, size: usize, psize: usize) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(20_261_028 + 1000 * index as u64);
    let look = match index {
        0 => sand(&mut rng, size, psize),
        1 => regolith(&mut rng, size, psize),
        2 => ice(&mut rng, size, psize),
        3 => frost(&mut rng, size, psize),
        4 => rocky_ice(&mut rng, size, psize),
        _ => lava(&mut rng, size, psize),
    };
    let mut out: Vec<Vec<u8>> = look.colour.iter().map(|c| to_u8(c)).collect();
    for (height, metres) in [(&look.relief2, SMALL), (&look.relief8, LARGE)] {
        let (sx, sy) = slopes(height, size, metres);
        out.push(to_u8(&sx));
        out.push(to_u8(&sy));
    }
    out.push(to_u8(&look.shade.iter().map(|s| clip(0.5 - 0.35 * s, 0.0, 1.0)).collect::<Vec<_>>()));
    let (px, py) = slopes(&look.height, psize, PEBBLES);
    out.push(to_u8(&px));
    out.push(to_u8(&py));
    out
}

// ------------------------------------------------------------------ files

/// A plane's mip chain, down to 1 x 1 (2 x 2 boxes, rounded to even).
fn mips(plane: &[u8], size: usize) -> Vec<(usize, Vec<u8>)> {
    let mut levels = vec![(size, plane.to_vec())];
    while levels.last().unwrap().0 > 1 {
        let (n, prev) = levels.last().unwrap();
        let half = n / 2;
        let next = (0..half * half)
            .map(|i| {
                let (y, x) = (i / half * 2, i % half * 2);
                let sum = prev[y * n + x] as f32 + prev[(y + 1) * n + x] as f32 + prev[y * n + x + 1] as f32 + prev[(y + 1) * n + x + 1] as f32;
                (sum / 4.0).round_ties_even() as u8
            })
            .collect();
        levels.push((half, next));
    }
    levels
}

/// The first endpoint's weight of BC4's eight-value palette (the second gets the rest).
const BC_W0: [f64; 8] = [1.0, 0.0, 6.0 / 7.0, 5.0 / 7.0, 4.0 / 7.0, 3.0 / 7.0, 2.0 / 7.0, 1.0 / 7.0];

/// A plane as BC4 blocks (8 bytes per 4 x 4): the block's maximum and minimum are the endpoints
/// and the texels take the nearest of the eight values between them.
fn bc4(plane: &[u8], size: usize) -> Vec<u8> {
    let padded = size.max(4);
    let at = |y: usize, x: usize| plane[y.min(size - 1) * size + x.min(size - 1)] as f64;
    let mut out = Vec::with_capacity(padded * padded / 2);
    for by in 0..padded / 4 {
        for bx in 0..padded / 4 {
            let texels: Vec<f64> = (0..16).map(|t| at(by * 4 + t / 4, bx * 4 + t % 4)).collect();
            let (r0, r1) = (texels.iter().cloned().fold(f64::MIN, f64::max), texels.iter().cloned().fold(f64::MAX, f64::min));
            let palette: Vec<f64> = BC_W0.iter().map(|w| r0 * w + r1 * (1.0 - w)).collect();
            let mut bits = 0u64;
            for (t, v) in texels.iter().enumerate() {
                let mut best = 0;
                for (k, p) in palette.iter().enumerate() {
                    if (v - p).abs() < (v - palette[best]).abs() {
                        best = k;
                    }
                }
                bits |= (best as u64) << (3 * t);
            }
            out.push(r0 as u8);
            out.push(r1 as u8);
            out.extend_from_slice(&bits.to_le_bytes()[..6]);
        }
    }
    out
}

/// Writes one DDS array: `slices[type][plane of the file]` = the plane (8 bits, `size` px).
fn write_dds(path: &Path, format: Format, size: usize, slices: &[Vec<&Vec<u8>>]) -> Result<(), String> {
    let chain = |p: &Vec<u8>| mips(p, size);
    let count = size.trailing_zeros() as usize + 1;
    let per_block = if format == Format::Bc4 { 8 } else { 16 };
    let top = (size.max(4) / 4).pow(2) * per_block;
    let mut head = Vec::with_capacity(148);
    head.extend_from_slice(b"DDS ");
    let flags: u32 = 0x1 | 0x2 | 0x4 | 0x1000 | 0x2_0000 | 0x8_0000; // caps height width pixelformat mipmapcount linearsize
    for v in [124, flags, size as u32, size as u32, top as u32, 0, count as u32] {
        head.extend_from_slice(&v.to_le_bytes());
    }
    head.extend_from_slice(&[0; 44]);
    head.extend_from_slice(&32u32.to_le_bytes());
    head.extend_from_slice(&4u32.to_le_bytes());
    head.extend_from_slice(b"DX10");
    head.extend_from_slice(&[0; 20]);
    for v in [0x1000 | 0x8 | 0x40_0000u32, 0, 0, 0, 0] {
        head.extend_from_slice(&v.to_le_bytes());
    }
    for v in [if format == Format::Bc4 { 80u32 } else { 83 }, 3, 0, slices.len() as u32, 0] {
        head.extend_from_slice(&v.to_le_bytes());
    }
    let io = |e: std::io::Error| format!("can't write {}: {e}", path.display());
    let mut f = BufWriter::new(File::create(path).map_err(io)?);
    f.write_all(&head).map_err(io)?;
    for planes in slices {
        let chains: Vec<Vec<(usize, Vec<u8>)>> = planes.iter().map(|p| chain(p)).collect();
        for level in 0..count {
            let n = chains[0][level].0;
            let blocks: Vec<Vec<u8>> = chains.iter().map(|c| bc4(&c[level].1, n)).collect();
            if format == Format::Bc4 {
                f.write_all(&blocks[0]).map_err(io)?;
            } else {
                for (a, b) in blocks[0].chunks(8).zip(blocks[1].chunks(8)) {
                    f.write_all(a).map_err(io)?;
                    f.write_all(b).map_err(io)?;
                }
            }
        }
    }
    f.flush().map_err(io)
}

/// Makes all types and writes the six files, and the snow's two, into `dir` (`ShaderFixes`); `progress` hears each
/// type. `size` and `psize` are powers of two (1024 each: 46 MB).
pub fn write_all(dir: &Path, size: usize, psize: usize, snow_size: usize, mut progress: impl FnMut(&str)) -> Result<(), String> {
    let mut made = Vec::new();
    for (index, name) in TYPES.iter().enumerate() {
        progress(&format!("Making the {name} ground texture"));
        made.push(generate(index, size, psize));
    }
    for (name, format, planes) in FILES {
        // The stones' files are at their own size.
        let n = if planes[0] >= P0 || planes[0] == PS { psize } else { size };
        let slices: Vec<Vec<&Vec<u8>>> = made.iter().map(|m| planes.iter().map(|&p| &m[p]).collect()).collect();
        write_dds(&dir.join(format!("edcas_ground_{name}.dds")), format, n, &slices)?;
    }
    progress("Making the snow's height map");
    let snow = snow_planes(snow_size);
    for (name, format, planes) in SNOW_FILES {
        let slices: Vec<Vec<&Vec<u8>>> = planes.iter().map(|keys| keys.iter().map(|&p| &snow[p]).collect()).collect();
        write_dds(&dir.join(format!("edcas_ground_{name}.dds")), format, snow_size, &slices)?;
    }
    Ok(())
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    /// The 8 x 8 block means of a `size` px plane.
    fn block_means(plane: &[u8], size: usize) -> Vec<f64> {
        let b = size / 8;
        (0..64).map(|i| (0..b * b).map(|k| plane[((i / 8) * b + k / b) * size + (i % 8) * b + k % b] as f64).sum::<f64>() / (b * b) as f64).collect()
    }

    /// Every look's planes at 128 px agree with the script's (`ground_detail_golden.py`, which
    /// makes the same textures with numpy): same random numbers, same shapes. Block means,
    /// within 2 of 255 (the FFT and float32 rounding differ a little).
    #[test]
    fn the_looks_match_the_script() {
        let golden = include_str!("../testdata/ground_detail_golden.txt");
        let names = ["c0", "c1", "c2", "c3", "r0", "r1", "r2", "r3", "ps", "p0", "p1"];
        let snow = snow_planes(128);
        let looks: Vec<Vec<Vec<u8>>> = (0..TYPES.len()).map(|i| generate(i, 128, 128)).collect();
        let mut checked = 0;
        for line in golden.lines().filter(|l| !l.starts_with('#')) {
            let mut words = line.split_whitespace();
            let (kind, plane) = (words.next().unwrap(), words.next().unwrap());
            let want: Vec<f64> = words.map(|w| w.parse().unwrap()).collect();
            let got = if kind == "snow" {
                block_means(&snow[["sh", "s0", "s1", "ch"].iter().position(|n| *n == plane).unwrap()], 128)
            } else {
                let index = TYPES.iter().position(|t| *t == kind).unwrap();
                block_means(&looks[index][names.iter().position(|n| *n == plane).unwrap()], 128)
            };
            let worst = want.iter().zip(&got).map(|(w, g)| (w - g).abs()).fold(0.0, f64::max);
            assert!(worst <= 2.0, "{kind} {plane}: a block mean differs by {worst:.1}\nwant {want:?}\ngot  {got:?}");
            checked += 1;
        }
        assert_eq!(checked, TYPES.len() * names.len() + 4);
    }

    #[test]
    fn the_random_numbers_are_the_scripts() {
        // ground_detail_tex.Rng(1).uniform(3)
        let mut rng = Rng::new(1);
        let u = rng.uniform(3);
        assert!((u[0] - 0.566_561_58).abs() < 1e-7 && (u[2] - 0.971_002_75).abs() < 1e-7, "{u:?}");
        assert!(u.iter().all(|v| (0.0..1.0).contains(v)));
    }

    /// BC4 keeps each texel within a seventh of its block's range of the plane.
    #[test]
    fn bc4_blocks_hold_their_texels() {
        let plane: Vec<u8> = (0..64).map(|i| ((i % 8) * 20 + (i / 8) * 3) as u8).collect();
        let data = bc4(&plane, 8);
        assert_eq!(data.len(), 4 * 8);
        for (b, block) in data.chunks(8).enumerate() {
            let (r0, r1) = (block[0] as f64, block[1] as f64);
            let bits = u64::from_le_bytes([block[2], block[3], block[4], block[5], block[6], block[7], 0, 0]);
            for t in 0..16 {
                let (y, x) = ((b / 2) * 4 + t / 4, (b % 2) * 4 + t % 4);
                let k = ((bits >> (3 * t)) & 7) as usize;
                let value = r0 * BC_W0[k] + r1 * (1.0 - BC_W0[k]);
                assert!((value - plane[y * 8 + x] as f64).abs() <= (r0 - r1) / 14.0 + 0.5, "block {b} texel {t}");
            }
        }
    }

    #[test]
    fn mips_go_down_to_one_texel() {
        let chain = mips(&[10, 20, 30, 41], 2);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[1], (1, vec![25])); // 101 / 4 = 25.25
        assert_eq!(mips(&[0; 16], 4).len(), 3);
    }

    #[test]
    fn the_files_are_dds_arrays_of_all_types() {
        let dir = std::env::temp_dir().join(format!("edcas-ground-detail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut steps = Vec::new();
        write_all(&dir, 32, 64, 64, |s| steps.push(s.to_string())).unwrap();
        assert_eq!(steps.len(), TYPES.len() + 1);
        let word = |d: &[u8], at: usize| u32::from_le_bytes(d[at..at + 4].try_into().unwrap());
        for (name, format, planes) in FILES {
            let n: usize = if planes[0] >= P0 || planes[0] == PS { 64 } else { 32 };
            let data = std::fs::read(dir.join(format!("edcas_ground_{name}.dds"))).unwrap();
            assert_eq!(&data[..4], b"DDS ");
            assert_eq!((word(&data, 12), word(&data, 16)), (n as u32, n as u32), "{name}");
            assert_eq!(word(&data, 28), n.trailing_zeros() + 1, "{name}: mips");
            assert_eq!(&data[84..88], b"DX10");
            assert_eq!(word(&data, 128), if format == Format::Bc4 { 80 } else { 83 });
            assert_eq!(word(&data, 140), TYPES.len() as u32, "{name}: array size");
            let per_block = if format == Format::Bc4 { 8 } else { 16 };
            let slice: usize = (0..=n.trailing_zeros()).map(|l| ((n >> l).max(4) / 4).pow(2) * per_block).sum();
            assert_eq!(data.len(), 148 + TYPES.len() * slice, "{name}");
        }
        // The snow's two files: two identical slices of its own size, both BC5.
        for (name, _, planes) in SNOW_FILES {
            let data = std::fs::read(dir.join(format!("edcas_ground_{name}.dds"))).unwrap();
            assert_eq!((word(&data, 12), word(&data, 16), word(&data, 28)), (64, 64, 7), "{name}");
            assert_eq!(word(&data, 128), 83, "{name}: BC5");
            assert_eq!(word(&data, 140) as usize, planes.len(), "{name}: two slices, so that the shader reads an array");
            let slice: usize = (0..=6u32).map(|l| ((64usize >> l).max(4) / 4).pow(2) * 16).sum();
            assert_eq!(data.len(), 148 + planes.len() * slice, "{name}");
            assert_eq!(data[148..148 + slice], data[148 + slice..], "{name}: the slices are alike");
        }
        assert_eq!(file_names().len(), FILES.len() + SNOW_FILES.len());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The real thing, at 1024 px (`EDCAS_TEXTURE_SIZE` for another, `EDCAS_TEXTURE_DIR` to keep
    /// the files there): `cargo test --release -p edcas-client --lib -- --ignored --nocapture the_full_size`.
    #[test]
    #[ignore = "makes about 70 MB of textures; seconds in release, minutes in debug"]
    fn the_full_size_textures_are_made() {
        let size: usize = std::env::var("EDCAS_TEXTURE_SIZE").ok().and_then(|v| v.parse().ok()).unwrap_or(1024);
        let keep = std::env::var_os("EDCAS_TEXTURE_DIR").map(std::path::PathBuf::from);
        let dir = keep.clone().unwrap_or_else(|| std::env::temp_dir().join(format!("edcas-ground-detail-full-{}", std::process::id())));
        std::fs::create_dir_all(&dir).unwrap();
        let start = std::time::Instant::now();
        write_all(&dir, size, size, SNOW_SIZE, |s| eprintln!("{:?} {s}", start.elapsed())).unwrap();
        let bytes: u64 = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.metadata().unwrap().len()).sum();
        eprintln!("{bytes} bytes in {:?}", start.elapsed());
        assert!(bytes < 80_000_000);
        if keep.is_none() {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
