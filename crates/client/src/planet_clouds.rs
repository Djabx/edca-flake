//! Planet clouds: whether the sky of the planet the commander is at has clouds, and what
//! they look like, worked out from its atmosphere (the physics: `scripts/ed-shader/CLOUDS-RESEARCH.md`,
//! "the report", its sections as §).
//!
//! A gas forms ice clouds where the air is colder than the gas's frost point at the
//! pressure it has in the mix. The frost point comes from Clausius–Clapeyron, anchored at
//! the gas's triple point. A frost point above the gas's triple point means the gas condenses
//! to droplets, not ice. The margin `ΔT = T_s − T_f` between the surface temperature and the
//! frost point decides, by three regimes (§1.3): within 20 K the gas is buffered at its frost
//! point (low clouds and ground fog); the main gas 20 to 90 K above it condenses only up high
//! (sparse, faint clouds); a trace gas that still freezes at the cold side of the day
//! (`T_cold = 0.7 T_s`, §6.1) makes a thin veil. More than 90 K above every frost point the sky is
//! clear (§1.5, §6.3). How opaque a layer is follows the mass it can condense, `W = f q p / g`
//! (§1.4): the vapour column, and so the planet's gravity. When a trace gas makes the clouds
//! and the main gas is in its high regime, there is a second, higher layer.
//!
//! The drawing is done by 3DMigoto (`scripts/ed-shader/edcas_clouds.hlsl`); EDCAS writes
//! the values it reads, `x503` to `w504`, `z505`, `x501` and `y501` (the layer's base and thickness in metres), `x507`, `y507`, `w507`, `x508` and `x509` of `ShaderFixes/edcas_params.txt` (see
//! [`crate::ground_tint`], which owns that file). The shader draws either a thin shell
//! (`z505 = 0`) or a ray-marched slab of the thickness `z505`; the slab costs frames.
//!
//! The same margins tell whether the planet fogs up at night (`x507`, with the planet's radius
//! in `w507`): fog forms where the air near the ground cools to the gas's frost point overnight
//! (§2.5, §6.2): the main gas within 30 K of it, a trace gas that freezes at the cold side.
//! A trace gas makes less, a tidally locked body (long nights, §2.7) more.

use crate::journal_reader::BodyScan;

/// Gas constant, J/(mol K).
const R: f64 = 8.314;
/// How cold the top of a thin atmosphere gets, as a share of the surface temperature.
const TOP_OF_AIR: f64 = 0.6;
/// Gases with less than this share (percent) of the air don't make clouds of their own.
const MIN_PERCENT: f32 = 0.01;

// The regimes (§1.3, §6.1, §6.2), each one named constant.
/// More than this above its frost point a gas makes no clouds at any height (the report's 80 to
/// 100 K, §1.3 "ΔT ≫ 80 K: the main gas never condenses", §6.3).
const CLEAR_MARGIN_K: f64 = 90.0;
/// Within this of its frost point a gas is buffered: frost, ground fog, low clouds (§1.3).
const BUFFERED_MARGIN_K: f64 = 20.0;
/// The cold side of the day and the globe, as a share of the surface temperature: the report's
/// `T_cold ≈ T_s − (0.25 to 0.35) T_s` (§6.1 item 2), its middle.
const COLD_SIDE: f64 = 0.7;
/// The main gas fogs up overnight within this of its frost point (§6.2, "Dawn ground fog").
const MAIN_FOG_MARGIN_K: f64 = 30.0;
/// The share of the vapour column that condenses into cloud, `f` of §1.4 and §6.1 item 4 (0.1 to
/// 0.5 there; its middle).
const CONDENSED_SHARE: f64 = 0.3;

// 2026-10-26: tuned in game on a pure CO2 world (Hegoea XL-K c22-8 6 e), where the model
// gave x503 0.630, x504 0.508, w503 1.6 and the player found x503 0.35, x504 0.22, w503 0.8 right (Mars's
// cloud optical depth of 0.1 to 0.5 backs the thinner opacity). Later that day, with the clouds
// visible on the approach as well, x503 0.25 and x504 0.09 looked right (coverage x 0.72, opacity
// x 0.39 on top). The physics above stays as it was; these three map it to how the clouds of a
// thin atmosphere look, on every planet.
/// Share of the modelled coverage that is drawn (x503).
const COVERAGE_SCALE: f64 = 0.40;
/// Share of the modelled opacity that is drawn (x504), after the strength setting, where the
/// planet's gravity is unknown (today's opacity, from the gas's partial pressure). The model's
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

// Heights above the ground, in scale heights H = R T / (mu g) (§6.1 item 5, §6.2), since 2026-11-11
// (client 0.43.123). Elite's own atmosphere shell, in which z503 and z505 were shares, is 102 to 125 km
// deep on the four bodies measured (RenderDoc, README-ELITE.md) whatever their scale height (8 to 118 km),
// with its base 7 to 17 km under the ground; on 6 e (no capture) its mean, 112 km with the base 13 km
// under the ground, puts the tuned deck (z503 0.239, z505 0.1) at 8.2 to 19.3 km over the ground:
// 0.18 to 0.42 of 6 e's 45.8 km scale height.
/// The buffered deck's base: 0.05 H where the gas freezes at the ground, rising by this share of H
/// per kelvin of margin to 0.3 H (the report's 0.05 to 0.3 H for low decks); 6 e's 7.9 K come to 0.177 H.
const BUFFERED_BASE_PER_K: f64 = 0.016;
/// A slab's thickness in scale heights per share of Elite's shell it had (z505): 112 km / 45.8 km,
/// so that 6 e's 0.1 stays its 11.2 km.
const THICKNESS_SCALE_HEIGHTS: f64 = 112.0 / 45.84;
/// The main gas's high clouds sit this many scale heights up (§6.2: 5 to 8 H), or under the top of
/// Elite's atmosphere where that is lower (the shader holds them there).
const HIGH_BASE_SCALE_HEIGHTS: f64 = 5.0;

/// The wind the clouds drift with, m/s, by regime (§5.5: Mars's mesospheric clouds 15 to 107 m/s;
/// lower layers slower): the buffered deck 5 to 10, the veil 20 to 40, the main gas's high clouds 40
/// to 80, each rising with the margin (the higher in its range the layer sits). Overhead that is
/// 0.03 to 0.06 degrees a second at the heights phase 2 gives them: a drift, not a slide.
const WIND_MS: [(Regime, f64, f64); 3] = [(Regime::Buffered, 5.0, 10.0), (Regime::Veil, 20.0, 40.0), (Regime::High, 40.0, 80.0)];

/// The waves' amplitude at the bottom and the top of its range, and the heights (scale heights) where
/// it reaches them (CLOUD-TYPES-PLAN.md phase 2; CLOUDS-RESEARCH.md §2.2: waves grow as the density falls).
const WAVE_AMPLITUDE: (f64, f64) = (0.2, 0.6);
const WAVE_HEIGHTS: (f64, f64) = (2.0, 6.0);
/// The thin veil's waves, a share of that amplitude at its height: its own streaks and wisps lead.
const VEIL_WAVES: f64 = 0.5;
/// The main gas's high clouds' waves, a share of that amplitude: none. At the full share (0.5 at 5 H,
/// until client 0.43.157) the troughs fell under the cover's threshold and cut every streak into lumps
/// with a rippled edge; with none the same clouds are long smooth filaments. The owner's choice of 0.5,
/// 0.25 and 0 in replays of Slege ZL-I b43-1 2 by day and at dusk (2026-11-13,
/// `ed-captures/renderdoc/thin-clouds/streak-choices.png`): "C looks best by far". The research has
/// them filamentary and rippled both (CLOUDS-RESEARCH.md §5.4): this takes the ripple.
const HIGH_WAVES: f64 = 0.0;
/// The pattern's size `y503` (larger = smaller clouds): 8 makes a cloud about half the layer's height
/// wide. The main gas's high clouds are finer, thinner streaks and more of them at the same cover: the
/// owner's choice of 5.5, 8 and 12 in the same replays (`C-variants.png`).
const SIZE: f32 = 8.0;
const HIGH_SIZE: f32 = 12.0;

/// The amplitude of the waves across the wind in a layer of `regime` whose base is `scale_heights`
/// over the ground: none in the buffered deck (its forms are phase 3's), the amplitude rising from
/// 0.2 at 2 H to 0.6 at 6 H, of which the veil has half and the main gas's high clouds none
/// ([`HIGH_WAVES`]).
fn wave_amplitude(regime: Regime, scale_heights: f64) -> f64 {
    let share = ((scale_heights - WAVE_HEIGHTS.0) / (WAVE_HEIGHTS.1 - WAVE_HEIGHTS.0)).clamp(0.0, 1.0);
    let amplitude = WAVE_AMPLITUDE.0 + (WAVE_AMPLITUDE.1 - WAVE_AMPLITUDE.0) * share;
    match regime {
        Regime::Buffered => 0.0,
        Regime::Veil => VEIL_WAVES * amplitude,
        Regime::High => HIGH_WAVES * amplitude,
    }
}

/// The buffered deck's cells by night and rows by day (CLOUD-TYPES-PLAN.md phase 3), their strengths:
/// moderate, the deck stays the tuned one with a structure.
const DECK_CELLS: f64 = 0.5;
const DECK_ROWS: f64 = 0.5;
/// Cloud streets need a steady wind to line the convection up (CLOUD-TYPES-THIN-ATMOSPHERES.md §4,
/// radiatus): the deck's rows only from this wind (m/s) up.
const ROWS_MIN_WIND_MS: f64 = 6.0;

/// Fall streaks (virga) under an ice veil (CLOUD-TYPES-PLAN.md phase 4): one slab thickness long in air
/// of [`VIRGA_PRESSURE_PA`], longer in thinner air (crystals fall faster, CLOUD-TYPES-THIN-ATMOSPHERES.md
/// summary 4) by the square root of the pressure's ratio, 0.5 to 1.5 times; shorter the further the gas
/// is above its frost point at the ground (they sublimate sooner, S14), down to 0.3 times at the clear
/// sky's margin.
const VIRGA_PRESSURE_PA: f64 = 1000.0;
/// The fall streaks' strength: 1 draws them at the length [`virga_length`] works out. Stretching the slab
/// march they cost +0.44 to +0.54 ms of GPU time on ed_frame511315 (tails under its whole deck), over
/// CLOUD-TYPES-PLAN.md's 0.15 ms per form, and were off. With their own samples (0.43.142) they cost
/// +0.14 ms there at any length from 0.5 to 1.5 (2.198 to 2.336 ms for the pass, the same shader against
/// itself within 0.001) and +0.06 ms on a veil seen from below (ed_frame171605): timed 2026-11-12 with
/// renderdoc/passtime.py, PERFORMANCE.md. On since 0.43.144.
const VIRGA_STRENGTH: f64 = 1.0;

/// The fall streaks' length under a layer, in slab thicknesses: ice veils only.
fn virga_length(regime: Regime, droplets: bool, pressure_pa: f64, margin_k: f64) -> f64 {
    if regime != Regime::Veil || droplets || pressure_pa <= 0.0 {
        return 0.0;
    }
    let fall = (VIRGA_PRESSURE_PA / pressure_pa).sqrt().clamp(0.5, 1.5);
    let sublimation = 1.0 - 0.7 * ((margin_k - BUFFERED_MARGIN_K) / (CLEAR_MARGIN_K - BUFFERED_MARGIN_K)).clamp(0.0, 1.0);
    fall * sublimation
}

/// How much softer a droplet layer's edges are (`w530`, CLOUD-TYPES-PLAN.md phase 6).
const DROPLET_EDGES: f32 = 1.0;

/// The smooth veil's smoothness and its halo's strength (CLOUD-TYPES-PLAN.md phase 5).
const SMOOTH_VEIL: f32 = 1.0;
const HALO: f32 = 1.0;

/// The drawn cover (`x503`) from which a slow veil of water ice is the smooth one ([`smooth_veil`]).
const SMOOTH_VEIL_COVER: f64 = 0.2;

/// How strongly the cover follows the local time (`w501`; the shader's `slabThreshold`): thickest at
/// dawn, dusk and night, thinnest at noon (§2.5, §5.5, §6.4 anchor 5). The main gas's high clouds more:
/// they belong to dawn, dusk and night (§6.2, "sunlit only at twilight"), 1.64 times at the
/// terminator, 0.36 under the sun.
const LOCAL_TIME: f32 = 1.0;
const HIGH_LOCAL_TIME: f32 = 1.5;

/// The column mass (kg/m²) at which a low sun reddens as through Earth's air, the shader's (1, 0.55,
/// 0.35): Rayleigh scattering's optical depth grows with the column (§5.2: thin clear air reddens
/// little). Mars's 160 kg/m² come to 0.016, 6 e's 1 250 to 0.125.
const DENSE_COLUMN: f64 = 10_000.0;

/// The cloud strength setting at which the drawn optical depths below are stated: the default (70 %).
const REFERENCE_STRENGTH: f32 = 0.7;

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
// m 0.127) at clouds strength 72 %, where a ground fog density x507 of 0.1 looked right. Since
// 2026-11-11 the fog's share is 1 where the gas freezes at the ground and falls with the square of
// its margin to 0 at the margin where it no longer fogs up (30 K for the main gas, the cold side
// for a trace gas): 6 e's 7.9 K of 30 come to (1 - 7.9 / 30)² = 0.54, times 0.72 times 0.24 = 0.094,
// within 10 % of the 0.10 tuned (the linear fall gave 0.127, 27 % over).
/// The fog density (x507) of a planet whose gas freezes at the ground, with the clouds at full
/// strength and the gas the whole air.
const FOG_SCALE: f64 = 0.24;
/// How much more fog a tidally locked body gets: its nights are long (§2.7), capped at [`FOG_SCALE`].
const LOCKED_FOG: f64 = 1.5;

/// A gas that can freeze into clouds: its triple point, heat of sublimation and its ice.
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
    /// The ice's density, kg/m³ (§1.4: water 920, CO2 1600; the others' solid densities).
    density: f64,
    /// The cloud particles' effective radius, m (§6.1 item 4: water 2 to 5 µm, CO2 about 1 µm;
    /// 2 µm where the report gives none).
    particle_m: f64,
}

const ICES: [Ice; 9] = [
    Ice { name: "CarbonDioxide", label: "carbon dioxide ice", droplets: "carbon dioxide clouds", t_triple: 216.58, p_triple: 518_000.0, latent: 25_200.0, density: 1600.0, particle_m: 1e-6 },
    Ice { name: "Water", label: "water ice", droplets: "water clouds", t_triple: 273.16, p_triple: 611.66, latent: 51_000.0, density: 920.0, particle_m: 3e-6 },
    Ice { name: "Ammonia", label: "ammonia ice", droplets: "ammonia clouds", t_triple: 195.4, p_triple: 6_060.0, latent: 28_900.0, density: 820.0, particle_m: 2e-6 },
    Ice { name: "Methane", label: "methane ice", droplets: "methane clouds", t_triple: 90.69, p_triple: 11_696.0, latent: 9_130.0, density: 520.0, particle_m: 2e-6 },
    Ice { name: "Nitrogen", label: "nitrogen ice", droplets: "nitrogen clouds", t_triple: 63.15, p_triple: 12_520.0, latent: 6_290.0, density: 1030.0, particle_m: 2e-6 },
    Ice { name: "Oxygen", label: "oxygen ice", droplets: "oxygen clouds", t_triple: 54.36, p_triple: 146.3, latent: 7_260.0, density: 1530.0, particle_m: 2e-6 },
    Ice { name: "SulphurDioxide", label: "sulphur dioxide ice", droplets: "sulphur dioxide clouds", t_triple: 197.69, p_triple: 1_670.0, latent: 32_300.0, density: 1900.0, particle_m: 2e-6 },
    Ice { name: "Argon", label: "argon ice", droplets: "argon clouds", t_triple: 83.81, p_triple: 68_900.0, latent: 7_610.0, density: 1620.0, particle_m: 2e-6 },
    Ice { name: "Neon", label: "neon ice", droplets: "neon clouds", t_triple: 24.56, p_triple: 43_370.0, latent: 2_050.0, density: 1440.0, particle_m: 2e-6 },
];

impl Ice {
    /// The temperature below which the gas, at this partial pressure (Pa), freezes. `None`
    /// where the extrapolation breaks down (absurd pressures).
    fn frost_point(&self, partial_pa: f64) -> Option<f64> {
        let inverse = 1.0 / self.t_triple - (R / self.latent) * (partial_pa / self.p_triple).ln();
        (inverse > 0.0).then(|| 1.0 / inverse)
    }

    /// The pressure (Pa) of the gas over its ice at `kelvin`: the same curve read the other way.
    fn vapour_pressure(&self, kelvin: f64) -> f64 {
        self.p_triple * (-(self.latent / R) * (1.0 / kelvin - 1.0 / self.t_triple)).exp()
    }
}

/// Molar mass (kg/mol) of the game's gases; others count as 30 g/mol.
pub(crate) fn molar_mass(gas: &str) -> f64 {
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

/// The air's mean molar mass (kg/mol), the gases weighed by their share; 30 g/mol without any.
pub(crate) fn mean_molar_mass(gases: &[(&str, f32)]) -> f64 {
    let total: f64 = gases.iter().map(|(_, p)| f64::from(*p)).sum();
    if total > 0.0 {
        gases.iter().map(|(g, p)| molar_mass(g) * f64::from(*p)).sum::<f64>() / total
    } else {
        0.030
    }
}

/// The three ways a gas condenses in a thin atmosphere (§1.3), by its margin `ΔT = T_s − T_f`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Regime {
    /// Within 20 K of its frost point (or under it), any gas: frost, ground fog, low clouds.
    Buffered,
    /// A trace gas 20 to 90 K above its frost point that still freezes at the cold side
    /// (`T_f ≥ 0.7 T_s`): the thin, streaky veil.
    Veil,
    /// The main gas 20 to 90 K above its frost point: it condenses only up high, sparse and faint.
    High,
}

impl Regime {
    /// The vertical optical depth through a dense patch at the default strength (70 %) the regime is
    /// drawn with, from the least to the most cloud (`CLOUDS-PLAN.md` phase 1: veils and fog keep
    /// today's visible level, main-gas high clouds are faint by day, §6.2), and the physical optical
    /// depths (log10) mapped onto it: the 5th to 95th percentile of the 2 359 landable bodies with air
    /// in the journals of 2026-11-11, rounded to whole decades, with 6 e (buffered, 3.5e5) on today's
    /// 0.36. Physically most of these columns would be far more opaque (a pure CO2 world's buffered
    /// cloud holds hundreds of kg/m²); the order between bodies is what is kept.
    ///
    /// The main gas's high clouds were 0.04 to 0.10 until client 0.43.156 (the grade's recommendation,
    /// taken by the plan without the owner's answer to its question 1): on Slege ZL-I b43-1 2 (argon,
    /// 24.5 K above its frost point) nothing was seen of them by day, where the released 0.43.121 drew
    /// a slab of 0.38 ("clouds are also super thin", the owner, 2026-11-12). The owner's choice of four
    /// levels in replays of that body by day and at dusk (`ed-captures/renderdoc/thin-clouds/`): 0.12 to
    /// 0.30, there by day without a search, and still under the low deck's range.
    fn drawn_depth(self) -> ((f32, f32), (f64, f64)) {
        match self {
            Regime::Buffered => ((0.15, 0.45), (2.0, 7.0)),
            Regime::Veil => ((0.05, 0.45), (2.0, 5.0)),
            Regime::High => ((0.12, 0.30), (3.0, 6.0)),
        }
    }
}

/// The air of a planet as its scan gives it.
#[derive(Clone, Copy, Debug)]
pub struct Air<'a> {
    /// The game's names with their share in percent.
    pub gases: &'a [(&'a str, f32)],
    pub pressure_pa: f32,
    pub surface_k: f32,
    /// m/s²; 0 where unknown.
    pub gravity_ms2: f32,
    pub tidal_lock: bool,
}

impl<'a> Air<'a> {
    /// Air of unknown gravity on a body that turns.
    pub fn new(gases: &'a [(&'a str, f32)], pressure_pa: f32, surface_k: f32) -> Self {
        Air { gases, pressure_pa, surface_k, gravity_ms2: 0.0, tidal_lock: false }
    }

    pub fn with_gravity(self, gravity_ms2: f32) -> Self {
        Air { gravity_ms2, ..self }
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
    /// `x501`: the layer's base over the ground (the planet's radius `w507`), metres; 0 where the
    /// gravity is unknown: the shader then places it by `z503` and `z505` in Elite's shell as before.
    pub base_m: f32,
    /// `y501`: the slab's thickness, metres (the Thickness setting in it, as in `thickness`).
    pub thickness_m: f32,
    /// `z501`: the wind the clouds drift with, m/s (the shader's clock is `z515`); 0 = still.
    pub wind_ms: f32,
    /// `w501`: how strongly the cover follows the local time (1: 1.43 times `x503` at the terminator
    /// and at night, 0.57 times under the sun; the mean over the day side kept); 0 = not at all.
    pub local_time: f32,
    /// `x502`: how strongly the air reddens a low sun, its column mass `p / g` over a dense one's
    /// (Earth's 10 000 kg/m²), 0.01 to 1; 0 where the gravity is unknown (the shader then reddens as
    /// before, fully).
    pub reddening: f32,
    /// `y502`: the air's scale height, metres; 0 where unknown.
    pub scale_height_m: f32,
    /// `x521` to `w521`: the main gas's high clouds over a trace gas's layer ([`Forecast::high`]),
    /// drawn as a thin shell above the slab: their cover, height over the ground (m), the alpha of a
    /// vertical path through a dense patch, and their wind (m/s). Cover 0 = none (also where the
    /// gravity, and so their height, is unknown).
    pub high: [f32; 4],
    /// `x522` to `w522`, `x528` to `z528`: the photochemical haze, also over a sky without clouds.
    pub haze: Haze,
    /// `x529`, `y529`: the amplitude of the waves across the wind (undulatus, noctilucent bands and
    /// billows) in the clouds' own layer and in the second layer ([`wave_amplitude`]); 0 = none.
    pub waves: [f32; 2],
    /// `z529`, `w529`: the buffered deck's cells by night and rows along the wind by day, their
    /// strengths (the rows only in a wind of [`ROWS_MIN_WIND_MS`] or more); 0 = none.
    pub deck: [f32; 2],
    /// `x530`: the length of the fall streaks under an ice veil, in slab thicknesses ([`virga_length`]
    /// times [`VIRGA_STRENGTH`]); 0 = none.
    pub virga: f32,
    /// `y530`: how smooth the veil is, its streaks and wisps taken away towards a sheet (cirrostratus
    /// nebulosus, [`Forecast::smooth_veil`]); 0 = as before, 1 = smooth.
    pub smooth: f32,
    /// `z530`: the 22 degree halo's strength (water ice's hexagonal crystals; the smooth veil); 0 = none.
    pub halo: f32,
    /// `w530`: a droplet layer's softer edges (less erosion: droplets do not fall out in wisps); 0 for ice.
    pub droplet_edges: f32,
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
    pub const NONE: Clouds = Clouds { coverage: 0.0, size: 0.0, height: 0.0, brightness: 0.0, opacity: 0.0, slab_opacity: 0.0, thickness: 0.0, base_m: 0.0, thickness_m: 0.0, wind_ms: 0.0, local_time: 0.0, reddening: 0.0, scale_height_m: 0.0, high: [0.0; 4], haze: Haze::NONE, waves: [0.0; 2], deck: [0.0; 2], virga: 0.0, smooth: 0.0, halo: 0.0, droplet_edges: 0.0, volumetric: false, streaks: 0.0, glow: 0.0, iridescence: 0.0, fog: 0.0, radius_m: 0.0, fog_depth_m: 0.0, softness: 0.0, frost: 0.0, lamp_glow: 0.0 };

    /// The values file's lines. Without clouds that is `x503 = 0`, `x507 = 0` and `x521 = 0` alone: the other
    /// values are left as they were, so the file keeps the last clouds' look (or the defaults of
    /// `edcas.ini` before any), but the fog of the last planet doesn't stay on one without air.
    /// `z505` is the thickness for the slab and 0 for the thin shell, and `x504` goes with it.
    /// `y507`, `x508`, `x509` and `w509` (the fog's depth, the clouds' edge, the canopy frost, the lamps' glow) come last, with clouds
    /// only: the frost needs cloud or fog anyway, and its own measure fades out without them.
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        if self.coverage <= 0.0 {
            let mut lines = vec![("x503", "0.000".to_string()), ("x507", "0.000".to_string()), ("x521", "0.000".to_string())];
            lines.extend(self.haze.assignments());
            return lines;
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
            ("x501", format!("{:.0}", self.base_m)),
            ("y501", format!("{:.0}", self.thickness_m)),
            ("z501", format!("{:.1}", self.wind_ms)),
            ("w501", format!("{:.2}", self.local_time)),
            ("x502", format!("{:.3}", self.reddening)),
            ("y502", format!("{:.0}", self.scale_height_m)),
            ("x507", format!("{:.3}", self.fog)),
            ("w507", format!("{:.0}", self.radius_m)),
            ("y507", format!("{:.0}", self.fog_depth_m)),
            ("x508", format!("{:.3}", self.softness)),
            ("x509", format!("{:.3}", self.frost)),
            ("w509", format!("{:.3}", self.lamp_glow)),
            ("x521", format!("{:.3}", self.high[0])),
            ("y521", format!("{:.0}", self.high[1])),
            ("z521", format!("{:.3}", self.high[2])),
            ("w521", format!("{:.1}", self.high[3])),
            ("x529", format!("{:.3}", self.waves[0])),
            ("y529", format!("{:.3}", self.waves[1])),
            ("z529", format!("{:.3}", self.deck[0])),
            ("w529", format!("{:.3}", self.deck[1])),
            ("x530", format!("{:.3}", self.virga)),
            ("y530", format!("{:.3}", self.smooth)),
            ("z530", format!("{:.3}", self.halo)),
            ("w530", format!("{:.3}", self.droplet_edges)),
        ])
        .chain(self.haze.assignments())
        .collect()
    }
}

/// What makes a planet's photochemical haze (§4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HazeKind {
    /// Methane with nitrogen: tholins, orange-brown, blue in forward scatter (Titan, Pluto).
    Tholin,
    /// Methane alone: hydrocarbons, yellow-grey.
    Hydrocarbon,
    /// Ammonia: hydrazine, white-yellow (Jupiter's by analogy).
    Hydrazine,
    /// Sulphur dioxide with water: sulphur and sulphuric acid, pale yellow (Venus).
    Sulphur,
}

impl HazeKind {
    /// Its colour (linear) and how blue it turns in forward scatter.
    fn look(self) -> ([f32; 3], f32) {
        match self {
            HazeKind::Tholin => ([0.80, 0.52, 0.28], 1.0),
            HazeKind::Hydrocarbon => ([0.78, 0.74, 0.58], 0.0),
            HazeKind::Hydrazine => ([0.92, 0.90, 0.74], 0.0),
            HazeKind::Sulphur => ([0.95, 0.86, 0.55], 0.0),
        }
    }

    fn words(self) -> &'static str {
        match self {
            HazeKind::Tholin => "orange haze",
            HazeKind::Hydrocarbon => "yellow-grey haze",
            HazeKind::Hydrazine => "white-yellow haze",
            HazeKind::Sulphur => "pale yellow haze",
        }
    }
}

/// The photochemical haze the shader draws in closed form (`photoHaze` in `edcas_clouds.hlsl`): rows
/// `x522` (vertical optical depth from the ground, 0 = none), `y522` (scale height, m), `z522` (the
/// ground's radius, m), `w522` (blue in forward scatter), `x528` to `z528` (colour).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Haze {
    pub kind: Option<HazeKind>,
    pub depth: f32,
    pub scale_height_m: f32,
    pub radius_m: f32,
    pub forward_blue: f32,
    pub colour: [f32; 3],
}

impl Haze {
    pub const NONE: Haze = Haze { kind: None, depth: 0.0, scale_height_m: 0.0, radius_m: 0.0, forward_blue: 0.0, colour: [0.0; 3] };

    /// `x522 = 0` alone without haze (the rest stays for 3DMigoto's fade to 0).
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        if self.depth <= 0.0 || self.radius_m <= 0.0 {
            return vec![("x522", "0.000".to_string())];
        }
        vec![
            ("x522", format!("{:.4}", self.depth)),
            ("y522", format!("{:.0}", self.scale_height_m)),
            ("z522", format!("{:.0}", self.radius_m)),
            ("w522", format!("{:.2}", self.forward_blue)),
            ("x528", format!("{:.3}", self.colour[0])),
            ("y528", format!("{:.3}", self.colour[1])),
            ("z528", format!("{:.3}", self.colour[2])),
        ]
    }
}

/// The haze an air makes in sunlight (§4.2; no gate on the star's distance: Pluto's haze forms at
/// 30 to 50 AU): its parent gas at any share is the switch, the vertical optical depth 0.01 to 0.1
/// (§6.2) grows with the square roots of the parent's share and of the column mass (per 1 000 kg/m²),
/// halved for methane without nitrogen (tholins need the nitrogen), times the strength over the
/// default. 0.5 % methane in Pluto's 1.6 kg/m² comes to the floor, 0.01; 2 % in 1 kPa at 0.1 g to
/// 0.023; pure methane at 1 kPa and 0.15 g to 0.084. None without the gravity (no column, no height).
fn haze(air: &Air, strength: f32, scale_height: f64) -> Haze {
    if air.gravity_ms2 <= 0.0 || scale_height <= 0.0 || strength <= 0.0 {
        return Haze::NONE;
    }
    let share = |name: &str| air.gases.iter().filter(|(n, _)| *n == name).map(|(_, p)| f64::from(*p)).sum::<f64>();
    let column = f64::from(air.pressure_pa) / f64::from(air.gravity_ms2);
    let min = f64::from(MIN_PERCENT);
    let mut kinds = Vec::new();
    let methane = share("Methane");
    if methane >= min {
        kinds.push(if share("Nitrogen") >= 1.0 { (HazeKind::Tholin, methane, 1.0) } else { (HazeKind::Hydrocarbon, methane, 0.5) });
    }
    if share("Ammonia") >= min {
        kinds.push((HazeKind::Hydrazine, share("Ammonia"), 1.0));
    }
    if share("SulphurDioxide") >= min && share("Water") >= min {
        kinds.push((HazeKind::Sulphur, share("SulphurDioxide"), 1.0));
    }
    let depth = |percent: f64, factor: f64| (0.01 + 0.09 * ((percent / 100.0).sqrt() * (column / 1000.0).sqrt()).clamp(0.0, 1.0)) * factor;
    let Some((kind, percent, factor)) = kinds.into_iter().max_by(|a, b| depth(a.1, a.2).total_cmp(&depth(b.1, b.2))) else {
        return Haze::NONE;
    };
    let (colour, forward_blue) = kind.look();
    Haze {
        kind: Some(kind),
        depth: (depth(percent, factor) * f64::from(strength / REFERENCE_STRENGTH)) as f32,
        scale_height_m: scale_height as f32,
        radius_m: 0.0,
        forward_blue,
        colour,
    }
}

/// Why a planet has no clouds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoClouds {
    /// No atmosphere, or the scan has none of its data.
    Airless,
    /// There is air, but none of its gases freeze into clouds (helium, hydrogen, …).
    NothingFreezes,
    /// Every gas that could is too warm to freeze at any height, or more than 90 K above its frost point.
    TooWarm,
}

/// The main gas's high, faint clouds over a trace gas's layer (§3.1: Mars's CO2 clouds over its
/// water belt): decided here, drawn as the thin shell above the slab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HighLayer {
    /// What they are made of ("carbon dioxide ice").
    pub label: &'static str,
    /// The gas's margin above its frost point, K (20 to 90).
    pub margin_k: f64,
    /// Share of the sky covered, as `x503`.
    pub coverage: f32,
    /// The alpha of a vertical path through a dense patch at the strength asked for.
    pub opacity: f32,
}

/// The clouds for an atmosphere and where they come from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Forecast {
    pub clouds: Clouds,
    /// What the clouds are made of ("water ice", "water clouds" for droplets) and how high
    /// the gas's frost line is: 0 at the ground,
    /// 1 at the top of the air. Negative: the gas freezes right at the surface.
    pub source: Result<(&'static str, f64), NoClouds>,
    /// How the clouds' gas condenses; `None` without clouds.
    pub regime: Option<Regime>,
    /// The main gas's high clouds above a trace gas's layer, when it has them.
    pub high: Option<HighLayer>,
    /// The layer is droplets (the gas's frost point above its triple point): no virga, halo or
    /// iridescence, and no ice names (CLOUD-TYPES-THIN-ATMOSPHERES.md summary 3).
    pub droplets: bool,
    /// A thin veil of water ice that is broad and slow: the smooth veil, cirrostratus-like ([`smooth_veil`]).
    pub smooth_veil: bool,
}

impl Forecast {
    pub const AIRLESS: Forecast = Forecast { clouds: Clouds::NONE, source: Err(NoClouds::Airless), regime: None, high: None, droplets: false, smooth_veil: false };

    fn none(reason: NoClouds, haze: Haze) -> Forecast {
        Forecast { clouds: Clouds { haze, ..Clouds::NONE }, source: Err(reason), ..Forecast::AIRLESS }
    }

    /// One line for the settings, e.g. "Clouds: sulphur dioxide ice, cirrus-like veil, sparse".
    pub fn summary(&self) -> String {
        let text = self.clouds_summary();
        match self.clouds.haze.kind {
            Some(kind) if self.clouds.haze.depth > 0.0 => format!("{text}; {}", kind.words()),
            _ => text,
        }
    }

    fn clouds_summary(&self) -> String {
        match self.source {
            Err(NoClouds::Airless) => "No clouds: no atmosphere here".to_string(),
            Err(NoClouds::NothingFreezes) => "No clouds: none of its gases freeze".to_string(),
            Err(NoClouds::TooWarm) => "No clouds: too warm for any gas to freeze".to_string(),
            Ok((label, _)) => {
                // The words go by the modelled coverage, before it is scaled for drawing.
                let c = f64::from(self.clouds.coverage) / COVERAGE_SCALE;
                let spread = if c < 0.3 {
                    "sparse"
                } else if c < 0.5 {
                    "scattered"
                } else {
                    "widespread"
                };
                let fog = if self.clouds.fog > 0.0 { ", with ground fog" } else { "" };
                let form = match self.regime {
                    Some(Regime::Buffered) => format!("low {}, {spread}{fog}", layer_name(Regime::Buffered, self.droplets, false)),
                    Some(Regime::Veil) => format!("{}, {spread}{fog}", layer_name(Regime::Veil, self.droplets, self.smooth_veil)),
                    Some(Regime::High) | None => format!("{}, faint{fog}", layer_name(Regime::High, self.droplets, false)),
                };
                match self.high {
                    Some(high) => format!("Clouds: {label}, {form}; above: {}, {}, faint", high.label, layer_name(Regime::High, false, false)),
                    None => format!("Clouds: {label}, {form}"),
                }
            }
        }
    }
}

/// The name a layer goes by in the settings: the cloud catalogue's word for the form the physics allows
/// at 0.001 to 0.1 atm (CLOUD-TYPES-THIN-ATMOSPHERES.md §8), "-like" or "-type" since the substance is
/// rarely water. Cirrus, cirrostratus and the noctilucent forms are ice by definition: a droplet veil is a
/// "thin veil", droplet high clouds "thin high clouds". Never a name §8 says to avoid (cumulus,
/// cumulonimbus, nimbostratus, anvil, rain, drizzle).
fn layer_name(regime: Regime, droplets: bool, smooth: bool) -> &'static str {
    match regime {
        Regime::Buffered => "stratocumulus-like deck",
        Regime::Veil if droplets => "thin veil",
        Regime::Veil if smooth => "cirrostratus-like veil",
        Regime::Veil => "cirrus-like veil",
        Regime::High if droplets => "thin high clouds",
        Regime::High => "noctilucent-type high clouds",
    }
}

/// Whether a thin veil is the smooth one, cirrostratus nebulosus (CLOUD-TYPES-PLAN.md phase 5): water
/// ice (the halo's crystals), broad (drawn cover [`SMOOTH_VEIL_COVER`] or more) and in light wind (the
/// lower third of the veil's wind range, a stable layer: CT §3 "nebulosus: stable air, light wind").
fn smooth_veil(regime: Regime, gas: &str, droplets: bool, coverage: f64, wind_ms: f64) -> bool {
    let light_wind = wind(Regime::Veil, BUFFERED_MARGIN_K + (CLEAR_MARGIN_K - BUFFERED_MARGIN_K) / 3.0);
    regime == Regime::Veil && gas == "Water" && !droplets && coverage >= SMOOTH_VEIL_COVER && wind_ms <= light_wind + 1e-3
}

/// One gas of the air that can make clouds, and how.
struct Candidate {
    ice: &'static Ice,
    /// Its share of the air, percent.
    percent: f64,
    partial: f64,
    t_frost: f64,
    /// `T_s − T_f`, K.
    margin: f64,
    /// The frost line's height as a share of the air (0 at the ground, 1 at the top).
    m: f64,
    regime: Regime,
    main: bool,
}

/// The vertical optical depth a regime is drawn with at the default strength: the physical one (the
/// condensed column `W = f q p / g`, §1.4, §6.1 item 4) mapped in log10 onto the regime's range, or,
/// with the gravity unknown, today's opacity (from the gas's partial pressure), held under the
/// regime's top.
fn drawn_depth(c: &Candidate, air: &Air, mean_molar: f64) -> f32 {
    let ((least, most), (low, high)) = c.regime.drawn_depth();
    if air.gravity_ms2 <= 0.0 {
        let alpha = ((c.partial.log10() / 4.0).clamp(0.15, 0.8) as f32) * REFERENCE_STRENGTH * OPACITY_SCALE * SLAB_OPACITY_SCALE;
        return (-(1.0 - alpha).ln()).min(most);
    }
    // The vapour column the gas can hold: its own partial pressure, at most what its ice lets the
    // air at the surface temperature carry (§1.4 "the limit is the saturation vapour pressure"; at
    // the cold side it would leave a trace veil nothing, see CLOUDS-PLAN's deviations).
    let vapour = c.partial.min(c.ice.vapour_pressure(f64::from(air.surface_k)));
    let condensed = CONDENSED_SHARE * (molar_mass(c.ice.name) / mean_molar) * vapour / f64::from(air.gravity_ms2);
    let tau = 1.5 * condensed / (c.ice.density * c.ice.particle_m);
    let share = ((tau.max(1e-12).log10() - low) / (high - low)).clamp(0.0, 1.0) as f32;
    least + (most - least) * share
}

/// The slab's alpha along a vertical path (`x504` of the slab) for an optical depth drawn at the
/// default strength, at `strength` (linear in the strength, as before).
fn slab_alpha(depth: f32, strength: f32) -> f32 {
    ((1.0 - (-depth).exp()) * strength / REFERENCE_STRENGTH).min(0.999)
}

/// The clouds of an atmosphere at `strength` (0–1), how visible the clouds are made.
pub fn forecast(air: &Air, strength: f32) -> Forecast {
    let (pressure, surface) = (f64::from(air.pressure_pa), f64::from(air.surface_k));
    if air.gases.is_empty() || pressure <= 0.0 || surface <= 0.0 {
        return Forecast::AIRLESS;
    }
    let strength = strength.clamp(0.0, 1.0);
    let mean_molar = mean_molar_mass(air.gases);
    // In metres over the ground where the scale height is known.
    let scale_height = if air.gravity_ms2 > 0.0 { R * surface / (mean_molar * f64::from(air.gravity_ms2)) } else { 0.0 };
    let haze = haze(air, strength, scale_height);
    let top = TOP_OF_AIR * surface;
    let cold = COLD_SIDE * surface;
    let main_gas = air.gases.iter().fold(None, |best: Option<(&str, f32)>, &(name, percent)| match best {
        Some((_, most)) if most >= percent => best,
        _ => Some((name, percent)),
    });
    let mut candidates = Vec::new();
    let mut freezers = 0;
    for &(name, percent) in air.gases {
        let Some(ice) = ICES.iter().find(|i| i.name == name) else { continue };
        let partial = pressure * f64::from(percent) / 100.0;
        if percent < MIN_PERCENT || partial <= 0.0 {
            continue;
        }
        let Some(t_frost) = ice.frost_point(partial) else { continue };
        freezers += 1;
        let margin = surface - t_frost;
        let m = margin / (surface - top);
        if m >= 1.0 || margin > CLEAR_MARGIN_K {
            continue;
        }
        let main = main_gas.is_some_and(|(n, _)| n == name);
        let regime = if margin < BUFFERED_MARGIN_K {
            Regime::Buffered
        } else if main {
            Regime::High
        } else if t_frost >= cold {
            Regime::Veil
        } else {
            continue;
        };
        candidates.push(Candidate { ice, percent: f64::from(percent), partial, t_frost, margin, m, regime, main });
    }
    // The gas that freezes lowest makes the clouds; the main gas's high clouds only when no other
    // gas makes any, else they are the second layer above.
    let lowest = |high: bool| candidates.iter().filter(|c| (c.regime == Regime::High) == high).min_by(|a, b| a.margin.total_cmp(&b.margin));
    let Some(c) = lowest(false).or_else(|| lowest(true)) else {
        return Forecast::none(if freezers == 0 { NoClouds::NothingFreezes } else { NoClouds::TooWarm }, haze);
    };
    let (m, percent) = (c.m, c.percent);

    // A trace gas covers less of the sky: full cover from a quarter of the air up.
    let abundance = ((percent / 100.0).sqrt() + 0.3).clamp(0.0, 1.0);
    let coverage = match c.regime {
        Regime::Buffered => COVERAGE_SCALE * abundance * if m < 0.0 { 0.5 } else { (0.15 + 0.55 * (1.0 - m)).clamp(0.0, 0.7) },
        // 10 to 28 %, more the more of the cold side's range the gas is saturated over: 0 where it
        // just freezes at T_cold, 1 at the buffered regime's edge.
        Regime::Veil => 0.10 + 0.18 * ((c.t_frost - cold) / (surface - BUFFERED_MARGIN_K - cold).max(1e-6)).clamp(0.0, 1.0),
        Regime::High => high_coverage(c.margin),
    };
    let height = if m < 0.0 { 0.1 } else { (0.15 + 0.7 * m).clamp(0.1, 0.85) };
    let slab = slab_alpha(drawn_depth(c, air, mean_molar), strength);
    // Fog hugs the ground, high clouds are thin flat layers, the rest is in between.
    let thickness = if m < 0.0 {
        FOG_THICKNESS
    } else if m > 0.66 {
        HIGH_THICKNESS
    } else {
        THICKNESS
    };
    let base_scale_heights = match c.regime {
        Regime::Buffered => (0.05 + BUFFERED_BASE_PER_K * c.margin).clamp(0.05, 0.3),
        // 1 H at the buffered regime's edge to 4 H at the clear sky's (§6.2: 1 to 4 H).
        Regime::Veil => 1.0 + 3.0 * ((c.margin - BUFFERED_MARGIN_K) / (CLEAR_MARGIN_K - BUFFERED_MARGIN_K)).clamp(0.0, 1.0),
        Regime::High => HIGH_BASE_SCALE_HEIGHTS,
    };

    // Ground fog where the gas reaches its frost point near the ground on the cold side: the main
    // gas within 30 K of it, a trace gas that freezes at T_cold (§6.1, §6.2). Full where it freezes
    // at the surface, falling with the square of the margin's share of that range.
    let fog_range = if c.main { MAIN_FOG_MARGIN_K } else { surface - cold };
    let fog_share = (1.0 - c.margin.max(0.0) / fog_range).clamp(0.0, 1.0).powi(2);
    let locked = if air.tidal_lock { LOCKED_FOG } else { 1.0 };
    let fog = (FOG_SCALE * abundance * fog_share * locked).min(FOG_SCALE) * f64::from(strength);

    // The main gas's high clouds over a trace gas's layer.
    let high = if c.main {
        None
    } else {
        lowest(true).map(|h| HighLayer {
            label: h.ice.label,
            margin_k: h.margin,
            coverage: high_coverage(h.margin) as f32,
            opacity: slab_alpha(drawn_depth(h, air, mean_molar), strength),
        })
    };
    let high_row = match high {
        Some(h) if scale_height > 0.0 => {
            [h.coverage, (HIGH_BASE_SCALE_HEIGHTS * scale_height) as f32, h.opacity, wind(Regime::High, h.margin_k) as f32]
        }
        _ => [0.0; 4],
    };
    let clouds = Clouds {
        coverage: coverage as f32,
        size: if c.regime == Regime::High { HIGH_SIZE } else { SIZE },
        height: height as f32,
        brightness: BRIGHTNESS,
        opacity: slab / SLAB_OPACITY_SCALE,
        slab_opacity: slab,
        thickness,
        base_m: (base_scale_heights * scale_height) as f32,
        thickness_m: (THICKNESS_SCALE_HEIGHTS * f64::from(thickness) * scale_height) as f32,
        wind_ms: wind(c.regime, c.margin) as f32,
        local_time: if c.regime == Regime::High { HIGH_LOCAL_TIME } else { LOCAL_TIME },
        reddening: if air.gravity_ms2 > 0.0 { (pressure / f64::from(air.gravity_ms2) / DENSE_COLUMN).clamp(0.01, 1.0) as f32 } else { 0.0 },
        scale_height_m: scale_height as f32,
        high: high_row,
        haze,
        // The waves need the layer's height in scale heights, so the gravity (none where unknown).
        waves: if scale_height > 0.0 {
            [wave_amplitude(c.regime, base_scale_heights) as f32, if high_row[0] > 0.0 { wave_amplitude(Regime::High, HIGH_BASE_SCALE_HEIGHTS) as f32 } else { 0.0 }]
        } else {
            [0.0; 2]
        },
        virga: (virga_length(c.regime, c.t_frost > c.ice.t_triple, pressure, c.margin) * VIRGA_STRENGTH) as f32,
        smooth: 0.0,
        halo: 0.0,
        droplet_edges: if c.t_frost > c.ice.t_triple { DROPLET_EDGES } else { 0.0 },
        deck: if c.regime == Regime::Buffered {
            [DECK_CELLS as f32, if wind(c.regime, c.margin) >= ROWS_MIN_WIND_MS { DECK_ROWS as f32 } else { 0.0 }]
        } else {
            [0.0; 2]
        },
        volumetric: true,
        streaks: if m < 0.0 { 0.3 } else { (0.6 + 0.4 * m).clamp(0.0, 1.0) } as f32,
        glow: if height > 0.5 { 1.0 } else { 0.3 },
        // Iridescence for fresh, uniform crystals: the main gas's high clouds (Mars's CO2 twilight
        // clouds, §5.3); the shader shows it at dawn and dusk only.
        // Droplets have none (CLOUD-TYPES-PLAN.md phase 6: iridescence belongs to fresh ice crystals).
        iridescence: if c.regime == Regime::High && c.t_frost <= c.ice.t_triple { 0.5 } else { 0.0 },
        fog: fog as f32,
        radius_m: 0.0,
        fog_depth_m: FOG_DEPTH_M,
        softness: EDGE_SOFTNESS,
        frost: FROST_STRENGTH,
        lamp_glow: LAMP_GLOW,
    };
    let droplets = c.t_frost > c.ice.t_triple;
    let label = if droplets { c.ice.droplets } else { c.ice.label };
    let smooth_veil = smooth_veil(c.regime, c.ice.name, droplets, coverage, f64::from(clouds.wind_ms));
    // The smooth veil is a sheet with the 22 degree halo (water ice only, as smooth_veil asks).
    let clouds = if smooth_veil { Clouds { smooth: SMOOTH_VEIL, halo: HALO, ..clouds } } else { clouds };
    Forecast { clouds, source: Ok((label, m)), regime: Some(c.regime), high, droplets, smooth_veil }
}

/// The wind of a layer in `regime` at `margin` K above its frost point, m/s ([`WIND_MS`]).
fn wind(regime: Regime, margin: f64) -> f64 {
    let (_, slow, fast) = WIND_MS.iter().copied().find(|(r, ..)| *r == regime).unwrap_or((regime, 0.0, 0.0));
    let share = match regime {
        Regime::Buffered => margin / BUFFERED_MARGIN_K,
        Regime::Veil | Regime::High => (margin - BUFFERED_MARGIN_K) / (CLEAR_MARGIN_K - BUFFERED_MARGIN_K),
    };
    slow + (fast - slow) * share.clamp(0.0, 1.0)
}

/// The share of the sky the main gas's high clouds cover, at the clear sky's edge (90 K over the frost
/// point) and at the buffered regime's (20 K). Sporadic, a few per cent (§6.2): 3 to 6 % until client
/// 0.43.156, raised with their optical depth by the owner's choice ([`Regime::drawn_depth`]); the
/// released 0.43.121 drew 12 % on that body.
const HIGH_COVER: (f64, f64) = (0.055, 0.11);

/// The share of the sky the main gas's high clouds cover ([`HIGH_COVER`]).
fn high_coverage(margin: f64) -> f64 {
    HIGH_COVER.0 + (HIGH_COVER.1 - HIGH_COVER.0) * ((CLEAR_MARGIN_K - margin) / (CLEAR_MARGIN_K - BUFFERED_MARGIN_K)).clamp(0.0, 1.0)
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
    let air = Air { tidal_lock: body.tidal_lock, ..Air::new(&gases, exo.surface_pressure, body.surface_temperature).with_gravity(body.surface_gravity) };
    let mut forecast = forecast(&air, strength);
    if forecast.source.is_ok() {
        forecast.clouds.radius_m = body.radius;
    }
    if forecast.clouds.haze.depth > 0.0 {
        forecast.clouds.haze.radius_m = body.radius;
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

    /// The forecast with the gravity unknown (today's opacity).
    fn sky(gases: &[(&str, f32)], pressure: f32, kelvin: f32, strength: f32) -> Forecast {
        forecast(&Air::new(gases, pressure, kelvin), strength)
    }

    /// The forecast at a gravity in g.
    fn sky_g(gases: &[(&str, f32)], pressure: f32, kelvin: f32, gravity_g: f32, strength: f32) -> Forecast {
        forecast(&Air::new(gases, pressure, kelvin).with_gravity(gravity_g * 9.81), strength)
    }

    /// The vertical optical depth through a dense patch a slab alpha stands for, at the default strength.
    fn depth_70(slab: f32, strength: f32) -> f64 {
        -(1.0 - f64::from(slab) * 0.7 / f64::from(strength)).ln()
    }

    /// Hegoea XL-K c22-8 6 e from its scan: pure CO2, 804.6 Pa, 156.03 K, 0.643 m/s².
    const SIX_E: (f32, f32, f32) = (804.617_98, 156.032_67, 0.643_268);

    #[test]
    fn frost_points_follow_clausius_clapeyron() {
        // CO2 at 570 Pa: 1/Tf = 1/216.58 - (8.314/25200) ln(570/518000)
        //   = 4.6172e-3 + 3.2992e-4 * 6.8118 = 6.8647e-3 -> 145.7 K.
        assert!(near(ice("CarbonDioxide").frost_point(570.0).unwrap(), 145.67, 0.05));
        // At its triple point pressure a gas freezes at its triple point.
        assert!(near(ice("Water").frost_point(611.66).unwrap(), 273.16, 1e-9));
        // Water at 0.18 Pa: 1/Tf = 1/273.16 + (8.314/51000) * 8.1307 = 4.9862e-3 -> 200.6 K.
        assert!(near(ice("Water").frost_point(0.18).unwrap(), 200.55, 0.05));
        // The vapour pressure is the same curve read the other way.
        let co2 = ice("CarbonDioxide");
        assert!(near(co2.vapour_pressure(co2.frost_point(570.0).unwrap()), 570.0, 1e-6));
    }

    /// What a worked example must come to.
    #[derive(Debug)]
    enum Expect {
        Clear(NoClouds),
        /// The regime, whether it fogs up, whether a second (high) layer is decided.
        Clouds(Regime, bool, bool),
    }

    /// One worked example: its number, its air (gases, Pa, K, gravity in g) and what it must come to.
    type Example = (u8, &'static [(&'static str, f32)], f32, f32, f32, Expect);

    /// CLOUDS-GRADE.md's twenty worked examples (gravity in g; 6 e from its scan: the grade's 0.13 g
    /// was not its own; 11, 12, 13, 16, 19 from the grade's scale heights; 17 and 18 0.3 g), and
    /// two veils from the journals (21, 22).
    fn worked_examples() -> [Example; 22] {
        use Expect::{Clear, Clouds};
        use NoClouds::{NothingFreezes, TooWarm};
        use Regime::{Buffered, High, Veil};
        let six_g = SIX_E.2 / 9.81;
        [
            (1, &[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, Clouds(High, false, false)),
            (2, &[("CarbonDioxide", 99.97), ("Water", 0.03)], 608.0, 210.0, 0.38, Clouds(Buffered, true, true)),
            (3, &[("CarbonDioxide", 100.0)], SIX_E.0, SIX_E.1, six_g, Clouds(Buffered, true, false)),
            (4, &[("CarbonDioxide", 100.0)], 5000.0, 500.0, 0.5, Clear(TooWarm)),
            (5, &[("SulphurDioxide", 100.0)], 101.0, 150.0, 0.18, Clouds(Buffered, true, false)),
            (6, &[("SulphurDioxide", 100.0)], 1000.0, 250.0, 0.3, Clouds(High, false, false)),
            (7, &[("Nitrogen", 100.0)], 1000.0, 50.0, 0.1, Clouds(Buffered, true, false)),
            // Methane freezes at 59.4 K, under the cold side's 63 K: no veil (its haze is phase 7's).
            (8, &[("Nitrogen", 98.0), ("Methane", 2.0)], 1000.0, 90.0, 0.1, Clear(TooWarm)),
            (9, &[("Methane", 100.0)], 1000.0, 90.0, 0.15, Clouds(Buffered, true, false)),
            (10, &[("Ammonia", 100.0)], 1000.0, 180.0, 0.3, Clouds(Buffered, true, false)),
            (11, &[("Helium", 100.0)], 5000.0, 150.0, 0.3, Clear(NothingFreezes)),
            (12, &[("Neon", 100.0)], 5000.0, 40.0, 0.1, Clear(TooWarm)),
            (13, &[("Neon", 100.0)], 5000.0, 25.0, 0.1, Clouds(Buffered, true, false)),
            (14, &[("Water", 100.0)], 1000.0, 280.0, 0.5, Clouds(Buffered, true, false)),
            (15, &[("Water", 100.0)], 5000.0, 400.0, 0.5, Clear(TooWarm)),
            (16, &[("Argon", 85.0), ("Nitrogen", 10.0), ("Methane", 5.0)], 2000.0, 200.0, 0.3, Clear(TooWarm)),
            (17, &[("CarbonDioxide", 100.0)], 10_132.5, 280.0, 0.3, Clear(TooWarm)),
            (18, &[("SulphurDioxide", 100.0)], 10_132.5, 320.0, 0.3, Clear(TooWarm)),
            (19, &[("Oxygen", 100.0)], 1000.0, 120.0, 0.2, Clear(TooWarm)),
            (20, &[("CarbonDioxide", 100.0)], 10_000.0, 250.0, 2.0, Clouds(High, false, false)),
            // Kongga 2: sulphur dioxide over carbon dioxide; Blae Drye LC-S d5-13 AB 1 g: a trace of CO2 in argon.
            (21, &[("CarbonDioxide", 58.35), ("SulphurDioxide", 41.65)], 605.0, 209.4, 4.18 / 9.81, Clouds(Veil, true, true)),
            (22, &[("Argon", 99.42), ("CarbonDioxide", 0.58)], 223.0, 151.8, 3.31 / 9.81, Clouds(Veil, true, false)),
        ]
    }

    #[test]
    fn the_worked_examples_of_the_grade_come_out_as_the_report_has_them() {
        use Expect::{Clear, Clouds};
        use Regime::{Buffered, High, Veil};
        for (n, gases, pressure, kelvin, g, expect) in worked_examples() {
            let f = sky_g(gases, pressure, kelvin, g, 0.7);
            let c = f.clouds;
            match expect {
                Clear(reason) => assert_eq!((f.source, c.coverage, c.fog), (Err(reason), 0.0, 0.0), "example {n}: {f:?}"),
                Clouds(regime, fog, high) => {
                    assert_eq!((f.regime, c.fog > 0.0, f.high.is_some()), (Some(regime), fog, high), "example {n}: {f:?}");
                    let ((least, most), _) = regime.drawn_depth();
                    let tau = depth_70(c.slab_opacity, 0.7);
                    assert!(tau >= f64::from(least) - 1e-3 && tau <= f64::from(most) + 1e-3, "example {n}: tau {tau}, {f:?}");
                    let cover = f64::from(c.coverage);
                    let (low, high_cover) = match regime {
                        Buffered => (0.02, 0.28),
                        Veil => (0.10, 0.28),
                        High => HIGH_COVER,
                    };
                    assert!(cover >= low - 1e-6 && cover <= high_cover + 1e-6, "example {n}: cover {cover}, {f:?}");
                    if let Some(h) = f.high {
                        assert!(f64::from(h.coverage) >= HIGH_COVER.0 - 1e-6 && f64::from(h.coverage) <= HIGH_COVER.1 + 1e-6 && depth_70(h.opacity, 0.7) <= 0.30 + 1e-3, "example {n}: {h:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn six_e_keeps_what_was_tuned_in_game() {
        // Before (client 0.43.121) at strength 72 %: x503 0.252, thin x504 0.0941, slab 0.3138,
        // z503 0.239, z505 0.1, x507 0.0998. Within 10 % now, with its gravity and without.
        let (p, t, g) = SIX_E;
        for f in [forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.72), sky(&[("CarbonDioxide", 100.0)], p, t, 0.72)] {
            let c = f.clouds;
            assert_eq!(f.regime, Some(Regime::Buffered));
            for (now, before) in [(c.coverage, 0.252), (c.opacity, 0.0941), (c.slab_opacity, 0.3138), (c.height, 0.239), (c.thickness, 0.1), (c.fog, 0.0998)] {
                assert!(near(f64::from(now), before, 0.1 * before), "{now} against {before}: {c:?}");
            }
            assert!(near(f.source.unwrap().1, 0.127, 0.001));
            assert_eq!(f.summary(), "Clouds: carbon dioxide ice, low stratocumulus-like deck, widespread, with ground fog");
        }
        // Without its gravity the opacity is exactly today's: log10(804.6) / 4 * 0.72 * 0.18.
        let c = sky(&[("CarbonDioxide", 100.0)], p, t, 0.72).clouds;
        assert!(near(c.opacity.into(), 0.7264 * 0.72 * 0.18, 1e-4), "{c:?}");
    }

    #[test]
    fn the_main_gas_high_clouds_are_seen_by_day() {
        // Slege ZL-I b43-1 2: argon, 1097.7 Pa, 85.27 K, 2.19 m/s²; argon freezes at 60.8 K there, 24.5 K
        // under the surface. The level the owner chose on this body (2026-11-12): cover 10.6 %, a slab alpha
        // of 0.21 at 70 %; before it 5.8 % and 0.074, of which nothing was seen by day.
        let c = sky_g(&[("Argon", 100.0)], 1097.7366, 85.266, 2.190548 / 9.81, 0.7).clouds;
        assert!(near(c.coverage.into(), 0.106, 0.002) && near(c.slab_opacity.into(), 0.207, 0.004), "{c:?}");
        // Long smooth filaments, finer than the other layers' clouds (the owner's choice "C1", 2026-11-13).
        assert_eq!((c.size, c.waves), (12.0, [0.0; 2]));
    }

    #[test]
    fn the_same_air_holds_less_cloud_under_more_gravity() {
        // Example 20: CO2 10 kPa at 250 K, 81 K above its frost point: the main gas's high clouds. The
        // column p / g is twenty times smaller at 2 g than at 0.1 g.
        let heavy = sky_g(&[("CarbonDioxide", 100.0)], 10_000.0, 250.0, 2.0, 0.7).clouds;
        let light = sky_g(&[("CarbonDioxide", 100.0)], 10_000.0, 250.0, 0.1, 0.7).clouds;
        assert!(heavy.slab_opacity < light.slab_opacity, "{heavy:?} {light:?}");
        assert!(depth_70(light.slab_opacity, 0.7) <= 0.3 + 1e-6);
        // A buffered world too: 6 e's air at Mars's gravity is thinner than at its own.
        let (p, t, g) = SIX_E;
        let own = forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.7).clouds;
        let mars = sky_g(&[("CarbonDioxide", 100.0)], p, t, 0.38, 0.7).clouds;
        assert!(mars.slab_opacity < own.slab_opacity, "{mars:?} {own:?}");
    }

    #[test]
    fn hot_worlds_are_clear_and_the_gate_is_ninety_kelvin() {
        // Example 15: water 5 kPa at 400 K, 98 K above its frost point (m 0.61 < 1 let it through before).
        assert_eq!(sky(&[("Water", 100.0)], 5000.0, 400.0, 1.0).source, Err(NoClouds::TooWarm));
        // Water 2000 Pa freezes at 288.4 K (droplets: above its triple point). At 370 K that is 81.6 K
        // under the ground: high water clouds; at 380 K, 91.6 K: clear.
        let f = sky(&[("Water", 100.0)], 2000.0, 370.0, 1.0);
        assert_eq!((f.source.unwrap().0, f.regime), ("water clouds", Some(Regime::High)));
        assert_eq!(f.summary(), "Clouds: water clouds, thin high clouds, faint", "droplets get no ice name");
        assert_eq!(sky(&[("Water", 100.0)], 2000.0, 380.0, 1.0).source, Err(NoClouds::TooWarm));
        assert_eq!(sky(&[("Water", 100.0)], 2000.0, 380.0, 1.0).summary(), "No clouds: too warm for any gas to freeze");
    }

    #[test]
    fn a_mars_like_sky_has_high_faint_carbon_dioxide_clouds() {
        // CO2 95 % of 600 Pa = 570 Pa -> Tf 145.67 K; surface 210 K, top 126 K:
        // m = (210 - 145.67) / 84 = 0.766, 64.3 K above: the main gas's high clouds.
        let f = sky_g(&[("CarbonDioxide", 95.0), ("Nitrogen", 5.0)], 600.0, 210.0, 0.38, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!((label, f.regime), ("carbon dioxide ice", Some(Regime::High)));
        assert!(near(m, 0.766, 0.002), "{m}");
        let c = f.clouds;
        // Cover 0.055 + 0.055 * (90 - 64.3) / 70 = 0.075; height 0.15 + 0.7 * 0.766 = 0.686.
        assert!(near(c.coverage.into(), 0.075, 0.001), "{c:?}");
        assert!(near(c.height.into(), 0.686, 0.002), "{c:?}");
        assert!(near(c.streaks.into(), 0.906, 0.002), "{c:?}");
        assert!(depth_70(c.slab_opacity, 1.0) >= 0.12 - 1e-6 && depth_70(c.slab_opacity, 1.0) <= 0.30 + 1e-6, "{c:?}");
        assert_eq!((c.size, c.brightness, c.glow, c.iridescence, c.fog), (12.0, 0.8, 1.0, 0.5, 0.0));
        assert_eq!(f.summary(), "Clouds: carbon dioxide ice, noctilucent-type high clouds, faint");
        // Without its gravity the opacity is that of the partial pressure (0.345), held under the regime's 0.30.
        let c = sky(&[("CarbonDioxide", 95.0), ("Nitrogen", 5.0)], 600.0, 210.0, 1.0).clouds;
        assert!(near(depth_70(c.slab_opacity, 1.0), 0.30, 1e-4), "{c:?}");
    }

    #[test]
    fn a_trace_of_water_makes_the_low_clouds_and_the_carbon_dioxide_the_high_ones() {
        // Water 0.03 % of 600 Pa = 0.18 Pa -> Tf 200.55 K: 9.45 K above, buffered, m = 9.45 / 84 = 0.1125.
        let f = sky(&[("CarbonDioxide", 95.0), ("Water", 0.03), ("Nitrogen", 4.97)], 600.0, 210.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!((label, f.regime), ("water ice", Some(Regime::Buffered)));
        assert!(near(m, 0.1125, 0.002), "{m}");
        let c = f.clouds;
        // coverage 0.15 + 0.55 * 0.8875 = 0.638, times the trace factor
        // sqrt(0.0003) + 0.3 = 0.317 -> 0.202, times 0.40 = 0.081; height 0.15 + 0.7 * 0.1125 = 0.229
        assert!(near(c.coverage.into(), 0.202 * 0.40, 0.002), "{c:?}");
        assert!(near(c.height.into(), 0.229, 0.002), "{c:?}");
        assert_eq!((c.glow, c.iridescence), (0.3, 0.0), "low and not carbon dioxide");
        // Gravity unknown: today's opacity, log10(0.18) / 4 floored at 0.15, times 0.18 = 0.027.
        assert!(near(c.opacity.into(), 0.15 * 0.18, 1e-6), "{c:?}");
        // The carbon dioxide, 64.3 K above its frost point, is the second layer.
        let high = f.high.unwrap();
        assert_eq!(high.label, "carbon dioxide ice");
        assert!(near(high.margin_k, 64.33, 0.05) && near(high.coverage.into(), 0.075, 0.001), "{high:?}");
        assert_eq!(
            f.summary(),
            "Clouds: water ice, low stratocumulus-like deck, sparse, with ground fog; above: carbon dioxide ice, noctilucent-type high clouds, faint"
        );
    }

    #[test]
    fn a_trace_gas_that_freezes_at_the_cold_side_makes_a_thin_veil() {
        // Kongga 2: SO2 41.65 % of 605 Pa freezes at 180.3 K, 29.1 K under a 209.4 K ground and over the
        // cold side's 146.6 K: a veil. Saturated over (180.3 - 146.6) / (189.4 - 146.6) = 0.79 of the
        // range: cover 0.10 + 0.18 * 0.79 = 0.242.
        let gases = [("CarbonDioxide", 58.35), ("SulphurDioxide", 41.65)];
        let f = sky_g(&gases, 605.0, 209.4, 4.18 / 9.81, 0.7);
        assert_eq!((f.source.unwrap().0, f.regime), ("sulphur dioxide ice", Some(Regime::Veil)));
        assert!(near(f.clouds.coverage.into(), 0.242, 0.003), "{:?}", f.clouds);
        assert_eq!(
            f.summary(),
            "Clouds: sulphur dioxide ice, cirrus-like veil, widespread, with ground fog; above: carbon dioxide ice, noctilucent-type high clouds, faint"
        );
        // Warmer, the SO2 freezes under the cold side (T_f 180.3 < 0.7 * 260 = 182): no veil, and the
        // CO2 is 96 K above its own frost point: clear.
        assert_eq!(sky_g(&gases, 605.0, 260.0, 0.43, 0.7).source, Err(NoClouds::TooWarm));
        // A trace barely saturated at the cold side covers least: Blae Drye LC-S d5-13 AB 1 g.
        let f = sky_g(&[("Argon", 99.42), ("CarbonDioxide", 0.58)], 223.0, 151.8, 3.31 / 9.81, 0.7);
        assert_eq!(f.regime, Some(Regime::Veil));
        assert!(f.clouds.coverage < 0.15, "{:?}", f.clouds);
        // A veil holds more cloud the more vapour it has.
        let thin = sky_g(&[("Argon", 99.42), ("CarbonDioxide", 0.58)], 223.0, 151.8, 3.31 / 9.81, 0.7);
        let thick = sky_g(&[("Argon", 97.0), ("CarbonDioxide", 3.0)], 223.0, 151.8, 3.31 / 9.81, 0.7);
        assert_eq!((thin.regime, thick.regime), (Some(Regime::Veil), Some(Regime::Veil)));
        assert!(thin.clouds.slab_opacity < thick.clouds.slab_opacity, "{thin:?} {thick:?}");
    }

    #[test]
    fn neon_this_cold_and_thin_is_too_warm_to_freeze() {
        // Neon 2000 Pa: 1/Tf = 1/24.56 - (8.314/2050) ln(2000/43370)
        //   = 0.040717 + 4.0556e-3 * 3.0768 = 0.053195 -> 18.8 K. Surface 40 K, top 24 K:
        // m = (40 - 18.8) / 16 = 1.33 >= 1: the air never gets cold enough.
        assert!(near(ice("Neon").frost_point(2000.0).unwrap(), 18.80, 0.05));
        let f = sky(&[("Neon", 100.0)], 2000.0, 40.0, 1.0);
        assert_eq!(f.source, Err(NoClouds::TooWarm));
        assert_eq!(f.clouds.coverage, 0.0);
        // Colder (25 K, top 15 K): m = (25 - 18.8) / 10 = 0.62 -> clouds.
        let f = sky(&[("Neon", 100.0)], 2000.0, 25.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!(label, "neon ice");
        assert!(near(m, 0.62, 0.005), "{m}");
    }

    #[test]
    fn gases_that_never_freeze_and_hot_air_make_no_clouds() {
        let f = sky(&[("Helium", 100.0)], 5000.0, 100.0, 1.0);
        assert_eq!(f.source, Err(NoClouds::NothingFreezes));
        assert_eq!(sky(&[("Hydrogen", 50.0), ("Iron", 50.0)], 5000.0, 100.0, 1.0).source, Err(NoClouds::NothingFreezes));
        // SO2 1000 Pa -> Tf 192.7 K; surface 400 K, top 240 K: m = 1.3.
        let f = sky(&[("SulphurDioxide", 100.0)], 1000.0, 400.0, 1.0);
        assert_eq!(f.source, Err(NoClouds::TooWarm));
        assert_eq!(f.summary(), "No clouds: too warm for any gas to freeze");
        // A share under 0.01 % isn't a cloud maker, and zero pressure is airless.
        assert_eq!(sky(&[("Water", 0.005)], 600.0, 210.0, 1.0).source, Err(NoClouds::NothingFreezes));
        assert_eq!(sky(&[("Water", 100.0)], 0.0, 210.0, 1.0).source, Err(NoClouds::Airless));
    }

    #[test]
    fn a_cold_nitrogen_sky_is_buffered() {
        // N2 3000 Pa: Tf 56.43 K; surface 70 K, top 42 K: m = 13.57 / 28 = 0.485, 13.6 K above: buffered.
        let f = sky(&[("Nitrogen", 100.0)], 3000.0, 70.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!((label, f.regime), ("nitrogen ice", Some(Regime::Buffered)));
        assert!(near(ice("Nitrogen").frost_point(3000.0).unwrap(), 56.43, 0.05));
        assert!(near(m, 0.485, 0.002), "{m}");
        assert_eq!(f.clouds.iridescence, 0.0);
        assert_eq!(f.clouds.glow, 0.3, "height 0.49 is not above 0.5");
        // opacity (gravity unknown): log10(3000) / 4 = 0.8693 -> capped at 0.8, times the 50 % strength and 0.18 = 0.072.
        let half = sky(&[("Nitrogen", 100.0)], 3000.0, 70.0, 0.5);
        assert!(near(half.clouds.opacity.into(), 0.4 * 0.18, 1e-6), "{:?}", half.clouds);
    }

    #[test]
    fn gases_freezing_at_the_surface_give_a_low_fog() {
        // Water 100 % of 600 Pa -> Tf 273.2 K; surface 200 K: m < 0.
        let f = sky(&[("Water", 100.0)], 600.0, 200.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!(label, "water ice");
        assert!(m < 0.0);
        let c = f.clouds;
        assert_eq!((c.coverage, c.height, c.streaks, c.glow, c.iridescence), (0.5 * 0.40, 0.1, 0.3, 0.3, 0.0));
        assert_eq!(f.summary(), "Clouds: water ice, low stratocumulus-like deck, widespread, with ground fog");
        // With the fog off (the setting, ground_tint) the words leave it out.
        let mut off = f;
        off.clouds.fog = 0.0;
        assert_eq!(off.summary(), "Clouds: water ice, low stratocumulus-like deck, widespread");
    }

    #[test]
    fn no_summary_names_a_cloud_the_physics_forbids() {
        // CLOUD-TYPES-THIN-ATMOSPHERES.md §8: towers, rain decks and their features need dense, moist
        // air. Whole words, so the stratocumulus-like deck passes and a cumulus would not.
        const FORBIDDEN: [&str; 7] = ["cumulus", "cumulonimbus", "nimbostratus", "anvil", "rain", "drizzle", "pyrocumulus"];
        let mut summaries: Vec<String> =
            worked_examples().into_iter().map(|(_, gases, p, t, g, _)| sky_g(gases, p, t, g, 0.7).summary()).collect();
        // And the forms the examples lack: droplets low and high, a smooth veil, a veil of droplets' gas as ice.
        summaries.push(sky(&[("Water", 100.0)], 2000.0, 300.0, 1.0).summary());
        summaries.push(sky(&[("Water", 100.0)], 2000.0, 370.0, 1.0).summary());
        summaries.push(sky_g(&[("Nitrogen", 98.5), ("Water", 1.5)], 2000.0, 280.0, 0.5, 0.7).summary());
        summaries.push(sky_g(&[("Nitrogen", 99.5), ("Water", 0.5)], 2000.0, 280.0, 0.5, 0.7).summary());
        for summary in &summaries {
            let lower = summary.to_lowercase();
            let words: Vec<&str> = lower.split(|c: char| !c.is_alphabetic()).collect();
            for name in FORBIDDEN {
                assert!(!words.contains(&name), "{summary:?} names {name}");
            }
        }
        // Every layer is named by the catalogue: a deck, a veil or high clouds.
        for summary in summaries.iter().filter(|s| s.starts_with("Clouds:")) {
            assert!(["deck", "veil", "high clouds"].iter().any(|w| summary.contains(w)), "{summary}");
        }
    }

    #[test]
    fn a_broad_slow_veil_of_water_ice_is_the_smooth_one() {
        // Water 1.5 % of 2 kPa (30 Pa) freezes at 240.8 K, 39.2 K under a 280 K ground and over the cold
        // side's 196 K: a veil, saturated over (240.8 - 196) / (260 - 196) = 0.70 of the range (cover
        // 0.226), its wind 20 + 20 * 19.2 / 70 = 25.5 m/s, in the lower third (up to 26.7).
        let f = sky_g(&[("Nitrogen", 98.5), ("Water", 1.5)], 2000.0, 280.0, 0.5, 0.7);
        assert_eq!((f.regime, f.droplets, f.smooth_veil), (Some(Regime::Veil), false, true), "{f:?}");
        assert!(near(f.clouds.coverage.into(), 0.226, 0.003) && near(f.clouds.wind_ms.into(), 25.5, 0.1), "{:?}", f.clouds);
        assert!(f.summary().starts_with("Clouds: water ice, cirrostratus-like veil, "), "{}", f.summary());
        // Drawn as a sheet with the 22 degree halo.
        assert_eq!((f.clouds.smooth, f.clouds.halo), (1.0, 1.0));
        // A third of that water (10 Pa, 230.9 K): faster and sparser, the streaky one.
        let f = sky_g(&[("Nitrogen", 99.5), ("Water", 0.5)], 2000.0, 280.0, 0.5, 0.7);
        assert_eq!((f.regime, f.smooth_veil), (Some(Regime::Veil), false), "{f:?}");
        assert!(f.summary().starts_with("Clouds: water ice, cirrus-like veil, "), "{}", f.summary());
        assert_eq!((f.clouds.smooth, f.clouds.halo), (0.0, 0.0));
        // Carbon dioxide ice makes no smooth veil (its halo angles are not known): Kongga 2's SO2 neither.
        let f = sky_g(&[("CarbonDioxide", 58.35), ("SulphurDioxide", 41.65)], 605.0, 209.4, 4.18 / 9.81, 0.7);
        assert!(!f.smooth_veil);
    }

    /// The settings line of each worked example (`cargo test -p edcas-client --lib summaries -- --nocapture`).
    #[test]
    fn summaries_of_the_worked_examples() {
        for (n, gases, p, t, g, _) in worked_examples() {
            println!("{n:>2}: {}", sky_g(gases, p, t, g, 0.7).summary());
        }
    }

    #[test]
    fn a_warm_thin_water_world_has_droplets_not_ice() {
        // Water 2000 Pa: 1/Tf = 1/273.16 - (8.314/51000) ln(2000/611.66) = 3.4677e-3 -> 288.4 K,
        // above the triple point 273.16 K. Surface 300 K, top 180 K: m = 11.6 / 120 = 0.097, buffered.
        let f = sky(&[("Water", 100.0)], 2000.0, 300.0, 1.0);
        let (label, m) = f.source.unwrap();
        assert_eq!(label, "water clouds");
        assert!(near(m, 0.097, 0.005), "{m}");
        // Below the triple point (100 Pa -> 252.8 K) it is ice.
        assert_eq!(sky(&[("Water", 100.0)], 100.0, 260.0, 1.0).source.unwrap().0, "water ice");
    }

    #[test]
    fn a_trace_gas_covers_less_of_the_sky_down_to_the_fog() {
        // Water 1 % of 60000 Pa = 600 Pa -> Tf 273.2 K; surface 200 K: fog, 0.5 * (0.1 + 0.3) = 0.2, times 0.40 = 0.08.
        let f = sky(&[("Water", 1.0), ("Helium", 99.0)], 60_000.0, 200.0, 1.0);
        assert!(f.source.unwrap().1 < 0.0);
        assert!(near(f.clouds.coverage.into(), 0.2 * 0.40, 0.001), "{:?}", f.clouds);
        // From about half the air (sqrt(0.7) + 0.3 > 1) the cover is the full 0.5 * 0.40 = 0.2.
        let f = sky(&[("Water", 70.0), ("Helium", 30.0)], 1500.0, 200.0, 1.0);
        assert!(near(f.clouds.coverage.into(), 0.5 * 0.40, 0.001), "{:?}", f.clouds);
    }

    #[test]
    fn no_clouds_write_x3_and_x7_alone() {
        // x507 too, so the fog of the last planet doesn't stay on an airless one.
        let none = vec![("x503", "0.000".to_string()), ("x507", "0.000".to_string()), ("x521", "0.000".to_string()), ("x522", "0.000".to_string())];
        assert_eq!(Clouds::NONE.assignments(), none);
        let names: Vec<_> = sky(&[("Nitrogen", 100.0)], 3000.0, 70.0, 1.0).clouds.assignments().iter().map(|(n, _)| *n).collect();
        assert_eq!(names, ["x503", "y503", "z503", "w503", "x504", "y504", "w504", "z505", "x501", "y501", "z501", "w501", "x502", "y502", "x507", "w507", "y507", "x508", "x509", "w509", "x521", "y521", "z521", "w521", "x529", "y529", "z529", "w529", "x530", "y530", "z530", "w530", "x522"]);
    }

    #[test]
    fn the_fog_comes_from_the_cold_side() {
        // Water 100 % of 600 Pa on a 200 K ground: it freezes at the surface, share 1: 0.24 * strength.
        let f = sky(&[("Water", 100.0)], 600.0, 200.0, 1.0);
        assert!(f.source.unwrap().1 < 0.0);
        assert!(near(f.clouds.fog.into(), 0.24, 1e-6), "{:?}", f.clouds);
        let half = sky(&[("Water", 100.0)], 600.0, 200.0, 0.5).clouds;
        assert!(near(half.fog.into(), 0.12, 1e-6), "{half:?}");
        // The Mars-like sky's CO2 is 64 K above its frost point, past the main gas's 30: no fog.
        assert_eq!(sky(&[("CarbonDioxide", 95.0), ("Nitrogen", 5.0)], 600.0, 210.0, 1.0).clouds.fog, 0.0);
        // Example 9, methane at 90 K, 14.6 K above: fog now (m 0.41 was past the old 0.3).
        // (1 - 14.6 / 30)² = 0.263, times 0.24 * 0.7 = 0.044.
        let ch4 = sky(&[("Methane", 100.0)], 1000.0, 90.0, 0.7).clouds;
        assert!(near(ch4.fog.into(), 0.044, 0.001), "{ch4:?}");
        // The cold nitrogen sky, 13.6 K above: (1 - 13.6 / 30)² * 0.24 = 0.072.
        assert!(near(sky(&[("Nitrogen", 100.0)], 3000.0, 70.0, 1.0).clouds.fog.into(), 0.072, 0.001));
    }

    #[test]
    fn the_fog_of_6_e_is_within_a_tenth_of_the_x7_that_looked_right() {
        // Hegoea XL-K c22-8 6 e: pure CO2, 804.6 Pa, 156.03 K. Tf 148.1 K, 7.93 K above:
        // (1 - 7.93 / 30)² = 0.541, times 0.72 times 0.24 = 0.0935 (0.10 tuned).
        let (p, t, _) = SIX_E;
        let f = sky(&[("CarbonDioxide", 100.0)], p, t, 0.72);
        assert!(near(f.source.unwrap().1, 0.127, 0.001));
        assert!(near(f.clouds.fog.into(), 0.0935, 0.001), "{:?}", f.clouds);
        // On a 180 K ground the same air is 31.9 K above: no fog.
        let warm = sky(&[("CarbonDioxide", 100.0)], p, 180.0, 0.72);
        assert!(warm.clouds.coverage > 0.0 && warm.clouds.fog == 0.0, "{:?}", warm.clouds);
    }

    #[test]
    fn a_trace_gas_makes_less_fog_and_a_locked_body_more() {
        // The trace of water over carbon dioxide: 9.45 K above, of the cold side's 63 K:
        // (1 - 9.45 / 63)² = 0.7225, abundance 0.317, times 0.24.
        let f = sky(&[("CarbonDioxide", 95.0), ("Water", 0.03), ("Nitrogen", 4.97)], 600.0, 210.0, 1.0);
        assert!(near(f.clouds.fog.into(), 0.7225 * 0.317 * 0.24, 0.001), "{:?}", f.clouds);
        // The same gas as the whole air freezes at the ground: 0.24.
        let full = sky(&[("Water", 100.0)], 600.0, 210.0, 1.0).clouds.fog;
        assert!(full > 3.0 * f.clouds.fog, "{full}");
        // Tidally locked: 1.5 times, at most the full 0.24 * strength.
        let (p, t, g) = SIX_E;
        let turning = forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.72).clouds.fog;
        let locked = forecast(&Air { tidal_lock: true, ..Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g) }, 0.72).clouds.fog;
        assert!(near(locked.into(), 1.5 * f64::from(turning), 1e-6), "{locked} {turning}");
        let water = forecast(&Air { tidal_lock: true, ..Air::new(&[("Water", 100.0)], 600.0, 200.0) }, 1.0).clouds.fog;
        assert!(near(water.into(), 0.24, 1e-6), "{water}");
    }

    #[test]
    fn the_fog_and_the_radius_are_written_after_the_clouds() {
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        let mut c = sky(&[("CarbonDioxide", 100.0)], 804.6, 156.03, 0.72).clouds;
        c.radius_m = 564_697.0;
        assert_eq!((get(&c, "x507").as_str(), get(&c, "w507").as_str()), ("0.094", "564697"), "{c:?}");
        // The fog setting (see ground_tint) zeroes the density, the radius stays.
        c.fog = 0.0;
        assert_eq!((get(&c, "x507").as_str(), get(&c, "w507").as_str()), ("0.000", "564697"), "{c:?}");
    }

    #[test]
    fn the_fog_depth_the_edge_softness_and_the_frost_start_at_the_shaders_defaults() {
        let mut c = sky(&[("CarbonDioxide", 100.0)], 804.6, 156.03, 0.72).clouds;
        c.radius_m = 564_697.0;
        let get = |name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!((get("y507").as_str(), get("x508").as_str(), get("x509").as_str()), ("800", "0.500", "1.000"), "{c:?}");
        assert_eq!((get("x507").as_str(), get("z505").as_str()), ("0.094", "0.100"), "{c:?}");
    }

    #[test]
    fn the_slab_is_thinner_up_high_and_thicker_in_fog() {
        // Mid height: N2 at m 0.485. High: the Mars-like sky (m 0.766). Fog: water on a 200 K ground.
        assert_eq!(sky(&[("Nitrogen", 100.0)], 3000.0, 70.0, 1.0).clouds.thickness, 0.1);
        let high = sky(&[("CarbonDioxide", 95.0), ("Nitrogen", 5.0)], 600.0, 210.0, 1.0);
        assert_eq!(high.clouds.thickness, 0.06);
        assert_eq!(sky(&[("Water", 100.0)], 600.0, 200.0, 1.0).clouds.thickness, 0.2);
    }

    #[test]
    fn the_slab_opacity_is_the_thin_one_scaled() {
        // A buffered CO2 world of unknown gravity: pure CO2 at 600 Pa, 160 K. log10(600) / 4 = 0.6945,
        // times 0.18 = 0.125 thin, times 0.3 / 0.09 = 0.417.
        let f = sky(&[("CarbonDioxide", 100.0)], 600.0, 160.0, 1.0).clouds;
        assert!(near(f.opacity.into(), 0.6945 * 0.18, 0.001), "{f:?}");
        assert!(near(f.slab_opacity.into(), 0.6945 * 0.18 * 0.3 / 0.09, 0.003), "{f:?}");
        // Where the thin opacity is the 0.09 of 6 e, the slab one is 0.3.
        let half = sky(&[("CarbonDioxide", 100.0)], 600.0, 160.0, 0.72).clouds;
        assert!(near(half.opacity.into(), 0.09, 0.002) && near(half.slab_opacity.into(), 0.3, 0.006), "{half:?}");
        // The densest air: 0.8 * 0.18 = 0.144 thin, 0.48 in the slab.
        let dense = sky(&[("Nitrogen", 100.0)], 100_000.0, 70.0, 1.0).clouds;
        assert!(near(dense.opacity.into(), 0.144, 1e-6) && near(dense.slab_opacity.into(), 0.48, 1e-5), "{dense:?}");
        // With the gravity known the strength still scales it linearly.
        let (p, t, g) = SIX_E;
        let at = |s| forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), s).clouds.slab_opacity;
        assert!(near(f64::from(at(0.35)) * 2.0, at(0.7).into(), 1e-6));
    }

    #[test]
    fn the_thin_shell_writes_z5_zero_and_its_own_opacity() {
        let mut c = sky(&[("CarbonDioxide", 100.0)], 600.0, 160.0, 0.72).clouds;
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!((get(&c, "z505").as_str(), get(&c, "x504").as_str()), ("0.100", "0.300"), "{c:?}");
        c.volumetric = false;
        assert_eq!((get(&c, "z505").as_str(), get(&c, "x504").as_str()), ("0.000", "0.090"), "{c:?}");
        // A slab of no thickness is the thin shell too, with the shell's opacity.
        c.volumetric = true;
        c.thickness = 0.0;
        assert_eq!((get(&c, "z505").as_str(), get(&c, "x504").as_str()), ("0.000", "0.090"), "{c:?}");
        // Without clouds x503 and x507 stay alone in either mode.
        assert_eq!(Clouds { volumetric: true, ..Clouds::NONE }.assignments(), vec![("x503", "0.000".to_string()), ("x507", "0.000".to_string()), ("x521", "0.000".to_string()), ("x522", "0.000".to_string())]);
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

    #[test]
    fn a_scan_gives_its_gravity_and_tidal_lock() {
        let (p, t, g) = SIX_E;
        let six_e = BodyScan {
            surface_temperature: t,
            surface_gravity: g,
            tidal_lock: true,
            radius: 564_697.0,
            exo: Some(ExoScan { atmosphere_type: "CarbonDioxide".to_string(), surface_pressure: p, composition: vec![], so2_percent: 0.0 }),
            ..Default::default()
        };
        let air = Air { tidal_lock: true, ..Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g) };
        let mut want = forecast(&air, 0.72);
        want.clouds.radius_m = 564_697.0;
        assert_eq!(for_body(&six_e, 0.72), want);
    }

    #[test]
    fn the_layers_sit_in_scale_heights_over_the_ground() {
        // 6 e: H = 8.314 * 156.03 / (0.044 * 0.643) = 45.84 km. Its deck: base (0.05 + 0.016 * 7.93) H
        // = 8.11 km, 11.2 km thick (0.1 of Elite's 112 km shell): where it was, 8.2 to 19.3 km up.
        let (p, t, g) = SIX_E;
        let c = forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.72).clouds;
        assert!(near(c.base_m.into(), 8110.0, 30.0) && near(c.thickness_m.into(), 11_200.0, 30.0), "{c:?}");
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!((get(&c, "x501").as_str(), get(&c, "y501").as_str()), ("8089", "11198"), "{c:?}");
        // Twice the gravity, half the scale height: everything comes down by half.
        let heavy = forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(2.0 * g), 0.72).clouds;
        assert!(near(heavy.base_m.into(), f64::from(c.base_m) / 2.0, 1.0) && near(heavy.thickness_m.into(), f64::from(c.thickness_m) / 2.0, 1.0));
        // A gas freezing at the ground: 0.05 H, the fog-type deck twice as thick.
        let w = sky_g(&[("Water", 100.0)], 600.0, 200.0, 0.3, 1.0).clouds;
        let h = 8.314 * 200.0 / (0.018 * 0.3 * 9.81);
        assert!(near(w.base_m.into(), 0.05 * h, 1.0) && near(w.thickness_m.into(), 0.2 * 112.0 / 45.84 * h, 2.0), "{w:?}");
        // A veil 29.1 K above its frost point (Kongga 2): 1 + 3 * 9.1 / 70 = 1.39 H up.
        let gases = [("CarbonDioxide", 58.35), ("SulphurDioxide", 41.65)];
        let v = sky_g(&gases, 605.0, 209.4, 4.18 / 9.81, 0.7).clouds;
        let h = 8.314 * 209.4 / (mean_molar_mass(&gases) * 4.18);
        assert!(near(v.base_m.into(), 1.39 * h, 0.01 * h), "{v:?} {h}");
        // The main gas's high clouds: 5 H (the shader holds them under the atmosphere's top).
        let mars = sky_g(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, 0.7).clouds;
        assert!(near(mars.base_m.into(), 5.0 * 8.314 * 210.0 / (0.044 * 0.38 * 9.81), 5.0), "{mars:?}");
        // Without the gravity nothing is written but 0: the shader places the layer as before.
        let unknown = sky(&[("CarbonDioxide", 100.0)], p, t, 0.72).clouds;
        assert_eq!((get(&unknown, "x501").as_str(), get(&unknown, "y501").as_str()), ("0", "0"));
    }

    #[test]
    fn the_clouds_drift_slower_the_lower_they_are() {
        // 6 e's buffered deck, 7.9 K above: 5 + 5 * 7.9 / 20 = 7.0 m/s.
        let (p, t, g) = SIX_E;
        let c = forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.72).clouds;
        assert!(near(c.wind_ms.into(), 6.98, 0.01), "{c:?}");
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(get(&c, "z501"), "7.0");
        // A veil 29.1 K above (Kongga 2): 20 + 20 * 9.1 / 70 = 22.6; Mars's CO2 63.9 K above: 40 + 40 * 43.9 / 70 = 65.1.
        let v = sky_g(&[("CarbonDioxide", 58.35), ("SulphurDioxide", 41.65)], 605.0, 209.4, 4.18 / 9.81, 0.7).clouds;
        let mars = sky_g(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, 0.7).clouds;
        assert!(near(v.wind_ms.into(), 22.6, 0.1) && near(mars.wind_ms.into(), 65.1, 0.1), "{v:?} {mars:?}");
        // A gas freezing at the ground: 5 m/s, the slowest.
        assert_eq!(sky(&[("Water", 100.0)], 600.0, 200.0, 1.0).clouds.wind_ms, 5.0);
    }

    #[test]
    fn the_cover_follows_the_local_time_and_high_clouds_more() {
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        let (p, t, g) = SIX_E;
        let deck = forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.72).clouds;
        assert_eq!(get(&deck, "w501"), "1.00");
        let mars = sky_g(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, 0.7).clouds;
        assert_eq!(get(&mars, "w501"), "1.50");
    }

    #[test]
    fn thin_air_reddens_a_low_sun_little_and_the_scale_height_is_written() {
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        // 6 e: 804.6 Pa over 0.643 m/s² = 1 251 kg/m², an eighth of Earth's column.
        let (p, t, g) = SIX_E;
        let c = forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.72).clouds;
        assert_eq!((get(&c, "x502").as_str(), get(&c, "y502").as_str()), ("0.125", "45833"), "{c:?}");
        // Mars: 608 Pa over 3.73 m/s² = 163 kg/m²; a dense 10 kPa at 0.1 g is past Earth's: 1.
        assert_eq!(get(&sky_g(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, 0.7).clouds, "x502"), "0.016");
        assert_eq!(get(&sky_g(&[("Nitrogen", 100.0)], 10_000.0, 60.0, 0.1, 0.7).clouds, "x502"), "1.000");
        // Unknown gravity: 0, the shader's full reddening of before.
        assert_eq!((get(&sky(&[("CarbonDioxide", 100.0)], p, t, 0.72).clouds, "x502").as_str(), get(&sky(&[("CarbonDioxide", 100.0)], p, t, 0.72).clouds, "y502").as_str()), ("0.000", "0"));
        // Iridescence only for the main gas's high clouds, of any gas.
        assert_eq!(sky_g(&[("SulphurDioxide", 100.0)], 1000.0, 250.0, 0.3, 0.7).clouds.iridescence, 0.5);
        assert_eq!(c.iridescence, 0.0);
    }

    #[test]
    fn the_main_gas_high_clouds_over_a_trace_gas_layer_are_written_as_the_second_layer() {
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        // Mars with a trace of water: the water's low deck, the CO2's high clouds 5 H up.
        let f = sky_g(&[("CarbonDioxide", 99.97), ("Water", 0.03)], 608.0, 210.0, 0.38, 0.7);
        let h = 8.314 * 210.0 / (mean_molar_mass(&[("CarbonDioxide", 99.97), ("Water", 0.03)]) * 0.38 * 9.81);
        let [cover, height, opacity, wind] = f.clouds.high;
        assert!(near(cover.into(), 0.075, 0.001) && near(height.into(), 5.0 * h, 5.0) && depth_70(opacity, 0.7) <= 0.3 + 1e-4 && near(wind.into(), 65.1, 0.2), "{:?}", f.clouds);
        assert_eq!(get(&f.clouds, "x521"), "0.076");
        // The main gas's own high clouds have no second layer; nor has a layer whose gravity is unknown.
        assert_eq!(sky_g(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, 0.7).clouds.high, [0.0; 4]);
        assert_eq!(sky(&[("CarbonDioxide", 99.97), ("Water", 0.03)], 608.0, 210.0, 0.7).clouds.high, [0.0; 4]);
    }

    #[test]
    fn the_veil_carries_waves_that_grow_with_its_height_and_the_high_clouds_none() {
        // The main gas's high clouds sit 5 H up, where the amplitude is 0.2 + 0.4 * 3 / 4 = 0.5: they have
        // none of it (HIGH_WAVES), long filaments. Example 1, Mars-like.
        let f = sky_g(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, 0.7);
        assert_eq!(f.clouds.waves, [0.0; 2]);
        // Example 2: the water's deck has none, nor the CO2 above it as the second layer.
        let f = sky_g(&[("CarbonDioxide", 99.97), ("Water", 0.03)], 608.0, 210.0, 0.38, 0.7);
        assert_eq!(f.clouds.waves, [0.0; 2]);
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!((get(&f.clouds, "x529"), get(&f.clouds, "y529")), ("0.000".to_string(), "0.000".to_string()));
        // A veil, half the high layers' at its height: Kongga 2's SO2 29.1 K above its frost point sits
        // 1 + 3 * 9.1 / 70 = 1.39 H up, under 2 H: 0.5 * 0.2.
        let f = sky_g(&[("CarbonDioxide", 58.35), ("SulphurDioxide", 41.65)], 605.0, 209.4, 4.18 / 9.81, 0.7);
        assert!(near(f.clouds.waves[0].into(), 0.1, 1e-6) && f.clouds.waves[1] == 0.0, "{:?}", f.clouds.waves);
        // 6 e's deck has none; nor has anything whose gravity (so its height in scale heights) is unknown.
        let (p, t, g) = SIX_E;
        assert_eq!(forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.72).clouds.waves, [0.0; 2]);
        assert_eq!(sky(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.7).clouds.waves, [0.0; 2]);
        assert_eq!((wave_amplitude(Regime::Veil, 1.0), wave_amplitude(Regime::Veil, 9.0)), (0.1, 0.3));
        assert_eq!((wave_amplitude(Regime::High, 1.0), wave_amplitude(Regime::High, 9.0)), (0.0, 0.0));
    }

    #[test]
    fn ice_veils_trail_fall_streaks_longer_in_thinner_air() {
        // Kongga 2: 605 Pa, sqrt(1000 / 605) = 1.29; 29.1 K above, 1 - 0.7 * 9.1 / 70 = 0.909: 1.17,
        // written times the default strength (0: over the GPU budget).
        assert!(near(virga_length(Regime::Veil, false, 605.0, 29.1), 1.172, 0.005));
        let f = sky_g(&[("CarbonDioxide", 58.35), ("SulphurDioxide", 41.65)], 605.0, 209.4, 4.18 / 9.81, 0.7);
        assert!(near(f.clouds.virga.into(), 1.172 * VIRGA_STRENGTH, 0.005), "{:?}", f.clouds.virga);
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(get(&f.clouds, "x530"), format!("{:.3}", f.clouds.virga));
        // Thinner air, longer tails, at most 1.5 times; further above the frost point, shorter.
        assert!(virga_length(Regime::Veil, false, 223.0, 30.0) > virga_length(Regime::Veil, false, 605.0, 30.0));
        assert_eq!(virga_length(Regime::Veil, false, 10.0, 20.0), 1.5);
        assert!(near(virga_length(Regime::Veil, false, 1000.0, 90.0), 0.3, 1e-9));
        // Droplets do not fall out in streaks; decks and high clouds have none.
        assert_eq!(virga_length(Regime::Veil, true, 605.0, 30.0), 0.0);
        assert_eq!(sky_g(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, 0.7).clouds.virga, 0.0);
        let (p, t, g) = SIX_E;
        assert_eq!(forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.72).clouds.virga, 0.0);
    }

    #[test]
    fn droplet_layers_have_no_ice_optics_and_softer_edges() {
        // Example 14: water 1 kPa on a 280 K ground condenses above its triple point: droplets. No
        // iridescence, fall streaks or halo, softer edges.
        let f = sky_g(&[("Water", 100.0)], 1000.0, 280.0, 0.5, 0.7);
        let c = f.clouds;
        assert!(f.droplets && f.source.unwrap().0 == "water clouds", "{f:?}");
        assert_eq!((c.iridescence, c.virga, c.smooth, c.halo, c.droplet_edges), (0.0, 0.0, 0.0, 0.0, 1.0));
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!((get(&c, "w504"), get(&c, "w530")), ("0.000".to_string(), "1.000".to_string()));
        // High water droplets (2 kPa at 370 K): the high regime, but no iridescence.
        let f = sky(&[("Water", 100.0)], 2000.0, 370.0, 1.0);
        assert_eq!((f.regime, f.droplets, f.clouds.iridescence), (Some(Regime::High), true, 0.0));
        // Ice keeps its iridescence and its edges: Mars-like CO2.
        let c = sky_g(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, 0.7).clouds;
        assert_eq!((c.iridescence, c.droplet_edges), (0.5, 0.0));
        // Six e's CO2 ice: as before.
        let (p, t, g) = SIX_E;
        assert_eq!(forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.72).clouds.droplet_edges, 0.0);
    }

    #[test]
    fn the_buffered_deck_has_cells_by_night_and_rows_in_a_steady_wind() {
        // 6 e: 7.9 K above its frost point, wind 5 + 5 * 7.9 / 20 = 7.0 m/s: cells and rows.
        let (p, t, g) = SIX_E;
        let f = forecast(&Air::new(&[("CarbonDioxide", 100.0)], p, t).with_gravity(g), 0.72);
        assert_eq!(f.clouds.deck, [0.5, 0.5]);
        // Example 5, SO2 freezing at the ground (margin under 0): 5 m/s, too light for rows.
        let f = sky_g(&[("SulphurDioxide", 100.0)], 101.0, 150.0, 0.18, 0.7);
        assert!(f.clouds.wind_ms < 6.0 && f.clouds.deck == [0.5, 0.0], "{:?}", f.clouds);
        let get = |c: &Clouds, name: &str| c.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!((get(&f.clouds, "z529"), get(&f.clouds, "w529")), ("0.500".to_string(), "0.000".to_string()));
        // The high layers and the veil have none.
        assert_eq!(sky_g(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, 0.7).clouds.deck, [0.0; 2]);
        assert_eq!(sky_g(&[("CarbonDioxide", 58.35), ("SulphurDioxide", 41.65)], 605.0, 209.4, 4.18 / 9.81, 0.7).clouds.deck, [0.0; 2]);
    }

    #[test]
    fn methane_ammonia_and_sulphur_with_water_make_a_haze_even_under_a_clear_sky() {
        // Example 8, N2 with 2 % methane, clear of condensate: tholins, 0.01 + 0.09 * sqrt(0.02 * 1.02) = 0.023.
        let f = sky_g(&[("Nitrogen", 98.0), ("Methane", 2.0)], 1000.0, 90.0, 0.1, 0.7);
        assert_eq!((f.source, f.clouds.haze.kind), (Err(NoClouds::TooWarm), Some(HazeKind::Tholin)));
        assert!(near(f.clouds.haze.depth.into(), 0.0229, 0.0005), "{:?}", f.clouds.haze);
        assert_eq!(f.summary(), "No clouds: too warm for any gas to freeze; orange haze");
        // Its rows go out without clouds too, once the radius is in.
        let mut c = f.clouds;
        c.haze.radius_m = 1_000_000.0;
        let names: Vec<_> = c.assignments().into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["x503", "x507", "x521", "x522", "y522", "z522", "w522", "x528", "y528", "z528"]);
        // Example 9, pure methane: hydrocarbons, half the tholins' depth: (0.01 + 0.09 * 0.82) * 0.5.
        let ch4 = sky_g(&[("Methane", 100.0)], 1000.0, 90.0, 0.15, 0.7).clouds.haze;
        assert!(ch4.kind == Some(HazeKind::Hydrocarbon) && near(ch4.depth.into(), 0.0418, 0.0005), "{ch4:?}");
        // Ammonia, and sulphur dioxide only with water.
        assert_eq!(sky_g(&[("Ammonia", 100.0)], 1000.0, 180.0, 0.3, 0.7).clouds.haze.kind, Some(HazeKind::Hydrazine));
        assert_eq!(sky_g(&[("SulphurDioxide", 100.0)], 1000.0, 250.0, 0.3, 0.7).clouds.haze.kind, None);
        assert_eq!(sky_g(&[("SulphurDioxide", 99.0), ("Water", 1.0)], 1000.0, 250.0, 0.3, 0.7).clouds.haze.kind, Some(HazeKind::Sulphur));
        // Never more than 0.1 at the default strength; nothing on CO2, without gravity or at strength 0.
        assert!(sky_g(&[("Methane", 50.0), ("Nitrogen", 50.0)], 10_000.0, 150.0, 0.05, 0.7).clouds.haze.depth <= 0.1 + 1e-6);
        assert_eq!(sky_g(&[("CarbonDioxide", 100.0)], 608.0, 210.0, 0.38, 0.7).clouds.haze, Haze::NONE);
        assert_eq!(sky(&[("Methane", 100.0)], 1000.0, 90.0, 0.7).clouds.haze, Haze::NONE);
        assert_eq!(sky_g(&[("Methane", 100.0)], 1000.0, 90.0, 0.15, 0.0).clouds.haze, Haze::NONE);
    }
}
