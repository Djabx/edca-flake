//! The writer of `ShaderFixes/edcas_params.txt`, the values 3DMigoto's IniParams take while the
//! game runs, and the planet ground tint, whose module this began as. Which row is what is in
//! `scripts/ed-shader/README.md`; the shaders that read them are `scripts/ed-shader/edcas.ini`,
//! `*.hlsl` and the patched game shaders of [`crate::shader_mods`].
//!
//! The ground tint colours the ground of the planet the commander is at after what
//! it is made of — ice towards frost, metal towards steel grey, rock left as the game
//! draws it — or, in the fixed mode, gives every planet's ground one colour the player
//! picked.
//!
//! The drawing is done by 3DMigoto with a tinted copy of the game's ground shader and a
//! patch that applies new values while the game runs (`scripts/ed-shader`). EDCAS only
//! writes those values: `ShaderFixes/edcas_params.txt` in the game folder, with the
//! tint's strength in `x` and its colour in `y`, `z` and `w`.
//!
//! The ground detail (craters, ripples, cracks, stones under foot: `scripts/ed-shader/ground_detail_tex.py`)
//! has a look per kind of ground, [`GroundType`], worked out from the body's scan
//! ([`ground_type`]); `x511` and `z511` are its strength.
//!
//! The same file carries the star's light colour (`x526` to `w526`, `x527` the witch-space rainbow, see [`crate::star_light`]) and the planet's clouds (`x503` to `w504`, `z505`, `x508`, `x509` (the canopy frost), `w509` (the lamps' glow in fog) and the
//! ground fog's `x507`, `y507` and `w507`, see [`crate::planet_clouds`]).
//! The file is replaced as a whole, so both are written here, together, and any other
//! line already in it (the glow scale `x505` the player tunes by hand) is kept.
//!
//! The file is written by a thread of its own ([`GroundTint`]), not from the GUI's frames:
//! while the game is in front EDCAS's window isn't drawn (on Wayland a hidden window gets
//! no frames at all), yet the values have to follow the game. Only the wanted values are
//! written, when they change; the fade from the old ones to the new is 3DMigoto's
//! (`[System] params_fade`, per frame).

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use tracing::{info, warn};

use crate::journal_reader::{BodyScan, JournalData, Place};
use crate::planet_clouds::{self, Clouds, Forecast};
use crate::planet_dust::{self, Dust};
use crate::star_light::{self, StarLight};
use crate::settings::game_graphics::{GameGraphicsSettings, TintMode, DUST_REACH_RANGE, FOG_DEPTH_RANGE, MAX_LOOK_PERCENT};
use crate::settings::Settings;

/// The values file, in the game folder.
pub const PARAMS_FILE: &str = "ShaderFixes/edcas_params.txt";
/// Present when the shader mods are installed (Settings → Game Graphics → Shader Mods, or `scripts/ed-shader/install.py`).
const INSTALLED_MARK: &str = "ShaderFixes/edcas.ini";

/// The tint colour of pure ice and of pure metal, as a multiple of the ground's brightness.
const ICE: [f32; 3] = [1.10, 1.30, 1.60];
const METAL: [f32; 3] = [1.00, 1.05, 1.12];

/// What a body is made of, as shares of 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Makeup {
    pub ice: f32,
    pub rock: f32,
    pub metal: f32,
}

/// The kinds of ground the detail textures have a look for, in the order of their slices
/// (`TYPES` in `ground_detail_tex.py`, `ground_detail.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundType {
    /// Rock under a thin atmosphere: wind ripples and a pebble lag, as Mars.
    Sand,
    /// Airless rock or metal: craters and rocks, as the Moon.
    Regolith,
    /// Airless ice: thermal-contraction cracks, as Europa.
    Ice,
    /// Ice under a thin atmosphere: sublimation pits and frost, as Pluto.
    Frost,
    /// Ice mixed with rock: a dust lag, knobs and boulders, as comet 67P.
    RockyIce,
    /// Rock or metal with magma or silicate volcanism: slabs, ropes and vesicles, as Io.
    Lava,
}

impl GroundType {
    /// The slice of the texture arrays (`x514` in the shader).
    pub fn index(self) -> u8 {
        self as u8
    }

    pub fn name(self) -> &'static str {
        crate::ground_detail::TYPES[self as usize]
    }
}

/// From this surface pressure (Pa) a body has an atmosphere for the ground's look: the journals'
/// scans of 15 000 landable bodies show every body with an `AtmosphereType` at 100 Pa or more
/// and a trace of pressure (under 100 Pa) on 1 070 without one.
const ATMOSPHERE_PA: f32 = 100.0;

/// The ground detail's look for the ground of `body`; `None` for what has no ground (stars, gas
/// giants). Rock and metal (`Rocky body`, `High metal content body`, `Metal rich body`) with
/// magma or silicate-vapour volcanism are lava, else sand under an atmosphere and regolith
/// without one; icy bodies are frost under an atmosphere and ice without one; rocky ice bodies
/// are rocky ice. (Water, CO2 and ammonia geysers and magma are cryovolcanism and leave the
/// ice as it is.)
pub fn ground_type(body: &BodyScan) -> Option<GroundType> {
    let airy = match &body.exo {
        Some(exo) => exo.surface_pressure >= ATMOSPHERE_PA,
        None => !body.atmosphere.trim().is_empty(),
    };
    let volcanism = body.volcanism.to_ascii_lowercase();
    let lava = ["metallic magma", "rocky magma", "silicate vapour"].iter().any(|v| volcanism.contains(v));
    match body.planet_class.as_str() {
        "Rocky body" | "High metal content body" | "Metal rich body" if lava => Some(GroundType::Lava),
        "Rocky body" | "High metal content body" | "Metal rich body" if airy => Some(GroundType::Sand),
        "Rocky body" | "High metal content body" | "Metal rich body" => Some(GroundType::Regolith),
        "Rocky ice body" => Some(GroundType::RockyIce),
        "Icy body" if airy => Some(GroundType::Frost),
        "Icy body" => Some(GroundType::Ice),
        _ => None,
    }
}

/// The variable `edcas.ini` copies into `x514` when the ground is drawn. A variable, not an
/// IniParam: 3DMigoto fades IniParams over seconds, and the ground type is a slice number that
/// must not run through the others on its way.
const GROUND_TYPE_VARIABLE: &str = "$\\ShaderFixes\\edcas.ini\\ground_type";

/// What 3DMigoto gets for the ground detail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detail {
    /// The look for the ground the commander is at; `None` switches the detail off.
    pub ground: Option<GroundType>,
    /// `x511` and `z511`: the colour's and the relief's strength, 1 = as designed.
    pub strength: f32,
}

impl Detail {
    pub const NONE: Detail = Detail { ground: None, strength: 0.0 };

    /// The values file's lines. Off, that is `x511` and `z511` of 0.0001 (the shader reads 0 as
    /// its own 1, so "off" is too small to see) and nothing else but the ice look's switches: the
    /// type stays, so the look that fades out is not another one. The ice look (the gloss and
    /// sparkle `x520`, `z520`, the colours `w520`) belongs to the detail, and its rows read 0 as on,
    /// so they are written too: -1 (off) with the detail off, 0 (the shader's default) with it on,
    /// clearing a -1 from before.
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        match self.ground {
            Some(ground) if self.strength > 0.0 => vec![
                ("x511", format!("{:.3}", self.strength.max(0.001))),
                ("z511", format!("{:.3}", self.strength.max(0.001))),
                (GROUND_TYPE_VARIABLE, ground.index().to_string()),
                ("x520", "0".to_string()),
                ("w520", "0".to_string()),
            ],
            _ => vec![("x511", "0.0001".to_string()), ("z511", "0.0001".to_string()), ("x520", "-1".to_string()), ("w520", "-1".to_string())],
        }
    }
}

/// What 3DMigoto gets: how far to tint (0–1) and towards which colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tint {
    pub strength: f32,
    pub colour: [f32; 3],
}

impl Tint {
    pub const NONE: Tint = Tint { strength: 0.0, colour: [1.0; 3] };

    /// The values file's lines: strength, then colour. Without a tint that is `x = 0`
    /// alone, like the clouds: the colour stays as it was, so 3DMigoto's fade only takes
    /// the strength down (a colour fading to white at the same time would show twice).
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        if self.strength <= 0.0 {
            return vec![("x500", "0.000".to_string())];
        }
        let [r, g, b] = self.colour;
        vec![("x500", format!("{:.3}", self.strength)), ("y500", format!("{r:.3}")), ("z500", format!("{g:.3}")), ("w500", format!("{b:.3}"))]
    }
}

/// The values file's text: the tint's, the clouds' and the dust's lines, then every other
/// assignment of the `existing` text (like the hand-tuned `x505`). Comments and lines that are ours
/// but aren't written now (the cloud look while there are no clouds, the tint colour while there
/// is no tint) stay as they were.
pub fn params_text(existing: &str, look: &Look) -> String {
    let ours = look.assignments();
    let mut text = String::from("; written by EDCAS: planet ground tint (x500 y500 z500 w500), ground detail (x511, z511, $ground_type) and its ice look's switches (x520, w520), clouds (x503 to w504, z505, x507, w507, y507, x508, x509, w509), dust (x515 to z516, x519), the star's light (x526 y526 z526 w526, x527 the witch-space rainbow)\n");
    for (name, value) in &ours {
        text.push_str(&format!("{name} = {value}\n"));
    }
    for line in existing.lines().map(str::trim).filter(|l| !l.starts_with(';') && !l.starts_with('#')) {
        let Some((name, value)) = line.split_once('=') else { continue };
        let name = moved_row(name.trim());
        if !ours.iter().any(|(n, _)| n.eq_ignore_ascii_case(&name)) {
            text.push_str(&format!("{name} = {}\n", value.trim()));
        }
    }
    text
}

/// The IniParams rows moved from N to N + 500 with client 0.43.81, out of the way of EDHM
/// (the HUD mod), whose presets set rows 1 to 333 and drove the clouds' `x3` and `x4` (a
/// Windows tester, 2026-11-08: clouds switching with the radar's ground view). A key of the
/// old numbering in an existing file is renamed, so a hand-tuned `x5` goes on as `x505` and
/// the old lines stop setting EDHM's rows. Anything else (a `$variable`, `X1`) stays.
fn moved_row(name: &str) -> String {
    let (letter, digits) = name.split_at(name.len().min(1));
    if matches!(letter, "x" | "y" | "z" | "w") && digits.len() <= 2 && digits.chars().all(|c| c.is_ascii_digit()) {
        let n: u32 = digits.parse().unwrap_or(0);
        return format!("{letter}{}", n + 500);
    }
    name.to_string()
}

/// The body's makeup from the commander's scan, or a typical one for its planet class
/// when the scan has none (bodies only known from the server).
pub fn makeup(body: &BodyScan) -> Option<Makeup> {
    if let Some(c) = &body.composition {
        return Some(Makeup { ice: c.ice, rock: c.rock, metal: c.metal });
    }
    let (ice, rock, metal) = match body.planet_class.as_str() {
        "Icy body" => (0.85, 0.10, 0.05),
        "Rocky ice body" => (0.45, 0.45, 0.10),
        "Rocky body" => (0.0, 0.85, 0.15),
        "High metal content body" => (0.0, 0.65, 0.35),
        "Metal rich body" => (0.0, 0.30, 0.70),
        _ => return None,
    };
    Some(Makeup { ice, rock, metal })
}

/// The tint for a makeup at the player's strength (0–1): the ice and metal shares pull
/// towards their colours, rock keeps the game's own.
pub fn tint(m: Makeup, strength: f32) -> Tint {
    let (ice, metal) = (m.ice.max(0.0), m.metal.max(0.0));
    let tinted = ice + metal;
    if tinted <= 0.0 || strength <= 0.0 {
        return Tint::NONE;
    }
    let colour = [0, 1, 2].map(|i| (ice * ICE[i] + metal * METAL[i]) / tinted);
    Tint { strength: (strength * tinted.min(1.0)).clamp(0.0, 1.0), colour }
}

/// A tint towards the hue of `rgb` that keeps the ground's own light and dark: the colour
/// divided by its brightness. Black has no hue to give, so it tints nothing.
pub fn fixed(rgb: (u8, u8, u8), strength: f32) -> Tint {
    let c = [rgb.0, rgb.1, rgb.2].map(|v| f32::from(v) / 255.0);
    let brightness = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    if brightness < 0.01 || strength <= 0.0 {
        return Tint::NONE;
    }
    Tint { strength: strength.clamp(0.0, 1.0), colour: c.map(|v| v / brightness) }
}

/// The game folder holding 3DMigoto (`…/Products/elite-dangerous-odyssey-64`): the one set
/// in the settings, or, in a Steam Proton prefix, the one the journal folder belongs to
/// (`…/steamapps/compatdata/359320/pfx/…` → `…/steamapps/common/Elite Dangerous/…`).
pub fn game_dir(settings: &Settings) -> Option<PathBuf> {
    game_dir_of(&settings.game_graphics.game_folder, &settings.journal_reader.journal_directory)
}

/// [`game_dir`] from the two settings it depends on.
fn game_dir_of(game_folder: &str, journal_directory: &str) -> Option<PathBuf> {
    let set = game_folder.trim();
    if !set.is_empty() {
        return Some(PathBuf::from(set));
    }
    let journal = PathBuf::from(journal_directory.trim());
    journal
        .ancestors()
        .find(|p| p.file_name().is_some_and(|n| n == "359320") && p.parent().and_then(Path::file_name).is_some_and(|n| n == "compatdata"))
        .and_then(|prefix| prefix.parent()?.parent().map(|steamapps| steamapps.join("common").join("Elite Dangerous").join("Products").join("elite-dangerous-odyssey-64")))
        .filter(|dir| dir.is_dir())
}

/// Whether the game folder has EDCAS's ground tint installed.
pub fn installed(dir: &Path) -> bool {
    dir.join(INSTALLED_MARK).is_file()
}

/// How often the background thread looks at the game state and writes the file when
/// the wanted values changed. 3DMigoto re-reads the file four times a second.
const TICK: Duration = Duration::from_millis(250);

/// Everything the values file holds from us: the ground tint, the clouds, the dust and the
/// ground detail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub tint: Tint,
    pub clouds: Clouds,
    pub dust: Dust,
    pub detail: Detail,
    /// The colour of the system's star's light (see [`crate::star_light`]).
    pub star: StarLight,
}

impl Look {
    pub const NONE: Look = Look { tint: Tint::NONE, clouds: Clouds::NONE, dust: Dust::NONE, detail: Detail::NONE, star: StarLight::NONE };

    /// The values file's lines for it.
    pub fn assignments(&self) -> Vec<(&'static str, String)> {
        let mut ours = self.tint.assignments();
        ours.extend(self.clouds.assignments());
        ours.extend(self.dust.assignments());
        ours.extend(self.detail.assignments());
        ours.extend(self.star.assignments());
        ours
    }
}

impl Default for Look {
    fn default() -> Self {
        Look::NONE
    }
}

/// The body the commander is at (see [`JournalData::body_name_here`]), when its scan is in one of the lists.
fn body_here<'a>(name: &str, lists: &[&'a [BodyScan]]) -> Option<&'a BodyScan> {
    lists.iter().find_map(|bodies| bodies.iter().find(|b| !name.is_empty() && b.body_name == name))
}

/// The tint for `body` (where the commander is), given the settings.
fn tint_at(g: &GameGraphicsSettings, body: Option<&BodyScan>) -> Tint {
    if !g.ground_tint {
        return Tint::NONE;
    }
    let strength = f32::from(g.ground_tint_strength) / 100.0;
    if g.ground_tint_mode == TintMode::Fixed {
        return crate::theme::parse_rgb(&g.ground_tint_colour).map_or(Tint::NONE, |rgb| fixed(rgb, strength));
    }
    body.and_then(makeup).map_or(Tint::NONE, |m| tint(m, strength))
}

/// The smallest `x508` written: 0 would mean the shader's own softness.
const MIN_SOFTNESS: f32 = 0.01;

/// The `x509` written for no frost: the shader reads 0 as its own 1 (full strength), so "off" and 0 %
/// ask for a strength too small to see instead.
const MIN_FROST: f32 = 0.001;

/// A look setting in percent (0 to 300) as the factor it scales the forecast's value by.
fn look_scale(percent: u16) -> f32 {
    f32::from(percent.min(MAX_LOOK_PERCENT)) / 100.0
}

/// The clouds for `body` (where the commander is), given the settings: none while they
/// are off or the commander is at no scanned body.
fn clouds_at(g: &GameGraphicsSettings, body: Option<&BodyScan>) -> Forecast {
    if !g.planet_clouds {
        return Forecast::AIRLESS;
    }
    let strength = f32::from(g.planet_clouds_strength) / 100.0;
    let mut forecast = body.map_or(Forecast::AIRLESS, |b| planet_clouds::for_body(b, strength));
    if forecast.source.is_ok() {
        let c = &mut forecast.clouds;
        c.volumetric = g.planet_clouds_volumetric;
        c.thickness *= look_scale(g.planet_cloud_thickness);
        // The shader reads x508 = 0 as its own default, so 0 % asks for the least it takes instead.
        c.softness = (c.softness * look_scale(g.planet_cloud_softness)).max(MIN_SOFTNESS);
        c.fog_depth_m = f32::from(g.planet_fog_depth.clamp(*FOG_DEPTH_RANGE.start(), *FOG_DEPTH_RANGE.end()));
        c.frost = if g.planet_canopy_frost && g.planet_canopy_frost_strength > 0 {
            (c.frost * look_scale(g.planet_canopy_frost_strength)).max(MIN_FROST)
        } else {
            MIN_FROST
        };
        c.lamp_glow = look_scale(g.planet_lamp_glow);
        if g.planet_fog {
            c.fog *= look_scale(g.planet_fog_density);
        } else {
            c.fog = 0.0;
        }
    }
    forecast
}

/// The wind-blown dust for `body`, given the settings: none while it is off or the commander
/// is at no scanned body.
fn dust_at(g: &GameGraphicsSettings, body: Option<&BodyScan>) -> Dust {
    let wind = if g.planet_dust { look_scale(g.planet_dust_strength) } else { 0.0 };
    let dust = body.map_or(Dust::NONE, |b| planet_dust::for_body(b, wind));
    let reach_m = 1000.0 * f32::from(g.planet_dust_reach_km.clamp(*DUST_REACH_RANGE.start(), *DUST_REACH_RANGE.end()));
    Dust { reach_m, ..dust }
}

/// The ground detail for `body` (where the commander is), given the settings: none while it is
/// off or the body is unknown (a wrong kind of ground would be worse than the game's own).
fn detail_at(g: &GameGraphicsSettings, body: Option<&BodyScan>) -> Detail {
    match body.and_then(ground_type) {
        Some(ground) if g.ground_detail => Detail { ground: Some(ground), strength: look_scale(g.ground_detail_strength) },
        _ => Detail::NONE,
    }
}

/// The star's light at `near` (the body the game says the commander is close to; with none,
/// the arrival point), given the settings. Not the targeted body: marking a far star must not
/// move the light. In witch space (`jump` the class of the star being jumped to) that star's
/// colour, with the rainbow on it unless that is off: 3DMigoto's fade then slides out of the
/// rainbow into the new star's light at arrival, as the light comes up when the tunnel opens.
fn star_at(g: &GameGraphicsSettings, near: Option<&BodyScan>, lists: &[&[BodyScan]], jump: Option<&str>) -> StarLight {
    if !g.star_light {
        return StarLight::NONE;
    }
    let strength = look_scale(g.star_light_strength());
    match jump {
        Some(class) => StarLight { rainbow: g.star_light_rainbow, ..star_light::of_class(class, strength) },
        None => star_light::at(near, lists, strength),
    }
}

/// What the settings want for `body` (the one at or targeted) and `near` (the one the game
/// says the commander is close to); `lists` are the bodies known of the system (the journal's
/// and the server's), for the stars that light it.
fn look_at(g: &GameGraphicsSettings, body: Option<&BodyScan>, near: Option<&BodyScan>, lists: &[&[BodyScan]], jump: Option<&str>) -> Look {
    let dust = dust_at(g, body);
    Look { tint: tint_at(g, body), clouds: clouds_at(g, body).clouds, dust, detail: detail_at(g, body), star: star_at(g, near, lists, jump) }
}

/// What the GUI and the journal reader hand to the background thread. Each part is its
/// own lock so none of them waits on another; the thread takes them in this order.
pub struct Feed {
    config: Mutex<Config>,
    /// Where the commander is and the bodies the journal reader knows; the reader fills it.
    place: Arc<Mutex<Place>>,
    /// The current system's bodies from the server, for those the journal never scanned.
    api_bodies: Mutex<Vec<BodyScan>>,
}

/// The settings the writer depends on.
#[derive(Clone)]
struct Config {
    graphics: GameGraphicsSettings,
    journal_directory: String,
}

impl Config {
    fn of(settings: &Settings) -> Self {
        Config { graphics: settings.game_graphics.clone(), journal_directory: settings.journal_reader.journal_directory.clone() }
    }
}

fn locked<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Feed {
    pub fn new(settings: &Settings) -> Self {
        Feed { config: Mutex::new(Config::of(settings)), place: Arc::default(), api_bodies: Mutex::default() }
    }

    /// The current settings; picked up at the thread's next tick.
    pub fn set_settings(&self, settings: &Settings) {
        *locked(&self.config) = Config::of(settings);
    }

    /// The bodies the server knows of the current system.
    pub fn set_api_bodies(&self, bodies: &[BodyScan]) {
        *locked(&self.api_bodies) = bodies.to_vec();
    }

    /// Where the journal reader publishes where the commander is.
    pub fn place(&self) -> Arc<Mutex<Place>> {
        Arc::clone(&self.place)
    }
}

/// Runs the writer on its own thread, so the values follow the game also while the GUI
/// has no window to draw (on Wayland a hidden window gets no frames). The GUI only feeds
/// it ([`Self::sync`], [`Self::set_api_bodies`]); the journal reader fills [`Self::place`].
pub struct GroundTint {
    feed: Arc<Feed>,
    /// Dropping it (or sending) stops the thread, which then resets the file.
    stop: Option<mpsc::Sender<()>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl GroundTint {
    pub fn start(settings: &Settings) -> Self {
        let feed = Arc::new(Feed::new(settings));
        let (stop, stopped) = mpsc::channel::<()>();
        let thread_feed = Arc::clone(&feed);
        let handle = thread::Builder::new()
            .name("planet-params".into())
            .spawn(move || {
                let mut updater = Updater::default();
                loop {
                    tick(&thread_feed, &mut updater);
                    // A message or the sender going away both mean: stop.
                    if !matches!(stopped.recv_timeout(TICK), Err(mpsc::RecvTimeoutError::Timeout)) {
                        break;
                    }
                }
                // `updater` drops here, after the last tick: the file is reset before
                // the thread ends, which `GroundTint::drop` waits for.
            })
            .map_err(|e| warn!("Planet params: can't start the writer thread: {e}"))
            .ok();
        GroundTint { feed, stop: Some(stop), handle }
    }

    /// Hands over the current settings. Cheap; called every frame.
    pub fn sync(&self, settings: &Settings) {
        self.feed.set_settings(settings);
    }

    pub fn set_api_bodies(&self, bodies: &[BodyScan]) {
        self.feed.set_api_bodies(bodies);
    }

    /// Where the journal reader publishes where the commander is.
    pub fn place(&self) -> Arc<Mutex<Place>> {
        self.feed.place()
    }

    /// The body the commander is at (Status.json's body, else the targeted one), when the scan is known.
    fn body_here(journal: &JournalData) -> Option<&BodyScan> {
        body_here(journal.body_name_here(), &[&journal.bodies])
    }

    /// The clouds for the body the commander is at, given the settings: none while they
    /// are off or the commander is at no scanned body.
    pub fn wanted_clouds(settings: &Settings, journal: &JournalData) -> Forecast {
        clouds_at(&settings.game_graphics, Self::body_here(journal))
    }

    /// What the clouds at the commander's body come to, and whether it fogs up, for the
    /// settings; empty while they are off or the commander is at no scanned body.
    pub fn cloud_status(settings: &Settings, journal: &JournalData) -> String {
        if !settings.game_graphics.planet_clouds || Self::body_here(journal).is_none() {
            return String::new();
        }
        let forecast = Self::wanted_clouds(settings, journal);
        let summary = forecast.summary();
        if forecast.source.is_err() {
            return summary;
        }
        // The fog's own conditions (edcas_fog.hlsl), so a clear day here isn't taken for a fault.
        let fog = if forecast.clouds.fog > 0.0 {
            "ground fog in the low ground at night and while the sun is low, dark without lamps"
        } else {
            "no ground fog here"
        };
        format!("{summary} · {fog}")
    }
}

impl Drop for GroundTint {
    /// Stops the thread and waits until it has reset the file.
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// One tick of the background thread: looks at the settings and where the commander is
/// and writes the file when that changed. Everything the thread does, bar the sleeping.
pub fn tick(feed: &Feed, updater: &mut Updater) {
    let config = locked(&feed.config).clone();
    let (body_name, wanted) = {
        let place = locked(&feed.place);
        let api = locked(&feed.api_bodies);
        let lists: [&[BodyScan]; 2] = [&place.bodies, &api];
        let body = body_here(&place.body_name, &lists);
        let near = body_here(&place.near_body_name, &lists);
        updater.note_body(&place.body_name, body, &config.graphics);
        (place.body_name.clone(), look_at(&config.graphics, body, near, &lists, place.jump.as_deref()))
    };
    let dir = game_dir_of(&config.graphics.game_folder, &config.journal_directory).filter(|d| installed(d));
    let keep_tint_on_close = config.graphics.ground_tint && config.graphics.ground_tint_mode == TintMode::Fixed;
    updater.step(dir.as_deref(), keep_tint_on_close, &body_name, wanted);
}

/// Keeps the values file in step with what is wanted. Resets the file when dropped.
#[derive(Default)]
pub struct Updater {
    /// The body last logged by [`Self::note_body`].
    noted_body: Option<String>,
    /// What the file was last asked to hold (ours), once known.
    target: Option<Look>,
    written: Option<Written>,
}

/// The file and the values last written to it.
struct Written {
    path: PathBuf,
    ours: Vec<(&'static str, String)>,
    /// A fixed colour stays when EDCAS closes; a planet's tint and clouds don't.
    keep_tint_on_close: bool,
    /// The last write failed, so a broken folder is reported once, not every tick.
    failing: bool,
}

impl Updater {
    /// Logs the body the commander is at once per change (2026-11-08: a tester's "no clouds"
    /// or wrong tint could not be told from the log without knowing what the scan held):
    /// its scan data as far as `BodyScan` has it, and, when the forecast gives no clouds,
    /// why. Whether the mods are installed does not matter, the log is for diagnosing that too.
    pub fn note_body(&mut self, body_name: &str, body: Option<&BodyScan>, graphics: &GameGraphicsSettings) {
        if self.noted_body.as_deref() == Some(body_name) {
            return;
        }
        self.noted_body = Some(body_name.to_string());
        if body_name.is_empty() {
            info!("Body here: none (not at a body, or Status.json names none)");
            return;
        }
        let Some(b) = body else {
            info!("Body here: '{body_name}', but no scan of it is known (journal or server): no tint, clouds or detail without one");
            return;
        };
        let (atmosphere, pressure) = b.exo.as_ref().map_or(("no scan data".to_string(), "unknown".to_string()), |e| {
            (e.atmosphere_type.clone(), format!("{:.0} Pa", e.surface_pressure))
        });
        info!(
            "Body here: '{body_name}', class '{}', atmosphere '{atmosphere}', surface pressure {pressure}, temperature {:.0} K, landable {}",
            b.planet_class, b.surface_temperature, b.landable
        );
        if !graphics.planet_clouds {
            info!("Clouds for '{body_name}': switched off in the settings");
        } else if let Err(why) = clouds_at(graphics, Some(b)).source {
            let why = match why {
                planet_clouds::NoClouds::Airless => "no atmosphere, or the scan has no atmosphere data",
                planet_clouds::NoClouds::NothingFreezes => "there is air but none of its gases freeze into clouds",
                planet_clouds::NoClouds::TooWarm => "every gas that could is too warm to freeze at any height",
            };
            info!("Clouds for '{body_name}': none, {why}");
        }
    }

    /// Writes `wanted` to the file when it differs from what was written. `dir` is the game
    /// folder when the mod is installed in it; without one nothing is written. 3DMigoto
    /// fades from the old values to the new ones.
    pub fn step(&mut self, dir: Option<&Path>, keep_tint_on_close: bool, body_name: &str, wanted: Look) {
        let Some(dir) = dir else { return };
        let path = dir.join(PARAMS_FILE);

        if self.target != Some(wanted) {
            let target: Vec<_> = wanted.assignments().iter().map(|(n, v)| format!("{n}={v}")).collect();
            let ground = wanted.detail.ground.map_or("no detail", GroundType::name);
            info!("Planet params for '{body_name}' ({ground}): {}", target.join(" "));
            self.target = Some(wanted);
        }

        let ours = wanted.assignments();
        let mut failing = false;
        if let Some(w) = &mut self.written {
            if w.path == path && w.ours == ours {
                w.keep_tint_on_close = keep_tint_on_close;
                return;
            }
            failing = w.failing;
        }
        let text = params_text(&std::fs::read_to_string(&path).unwrap_or_default(), &wanted);
        // Remembered either way, so a broken folder isn't retried (and reported) with every value.
        let failed = write_atomically(&path, &text).map_err(|e| {
            if !failing {
                warn!("Planet params: can't write {}: {e}", path.display());
            }
        });
        self.written = Some(Written { path, ours, keep_tint_on_close, failing: failed.is_err() });
    }
}

impl Drop for Updater {
    /// Without EDCAS running nobody would change a planet's tint, clouds, dust or ground detail
    /// at the next planet, nor the star's light (or the witch-space rainbow) at the next system,
    /// so they go; a fixed colour stays.
    fn drop(&mut self) {
        if let Some(w) = &self.written {
            let tint = if w.keep_tint_on_close { self.target.map_or(Tint::NONE, |t| t.tint) } else { Tint::NONE };
            let reset = Look { tint, ..Look::NONE };
            if w.ours != reset.assignments() {
                let text = params_text(&std::fs::read_to_string(&w.path).unwrap_or_default(), &reset);
                let _ = write_atomically(&w.path, &text);
            }
        }
    }
}

/// Replaces the file in one step, so 3DMigoto never reads it half-written.
fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("txt.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    /// The defaults, not what the developer's own settings file holds.
    fn test_settings() -> Settings {
        Settings { game_graphics: Default::default(), ..Settings::default() }
    }

    /// The tint for the body the commander is at, given the settings.
    fn wanted(settings: &Settings, journal: &JournalData) -> Tint {
        tint_at(&settings.game_graphics, GroundTint::body_here(journal))
    }

    /// A game folder with the mod installed.
    fn game_folder(name: &str) -> PathBuf {
        let game = std::env::temp_dir().join(format!("edcas-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&game);
        std::fs::create_dir_all(game.join("ShaderFixes")).unwrap();
        std::fs::write(game.join(INSTALLED_MARK), "").unwrap();
        game
    }

    /// A journal's worth of place for the writer thread's feed.
    fn put_place(feed: &Feed, journal: &JournalData) {
        *feed.place().lock().unwrap() = Place::of(journal);
    }

    fn cold_n2() -> BodyScan {
        use crate::journal_reader::{AtmosphereGas, ExoScan};
        BodyScan {
            body_name: "Cold N2".to_string(),
            planet_class: "Icy body".to_string(),
            surface_temperature: 70.0,
            exo: Some(ExoScan {
                atmosphere_type: "Nitrogen".to_string(),
                surface_pressure: 3000.0,
                composition: vec![AtmosphereGas { name: "Nitrogen".to_string(), percent: 100.0 }],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// What the file holds for a value, e.g. `value(&text, "x503")` = 0.173.
    /// Hegoea XL-K c22-8 6 e: pure CO2, 804.6 Pa, 156.03 K, radius 564697 m; the world the
    /// clouds and the fog were tuned on.
    fn six_e() -> BodyScan {
        use crate::journal_reader::{AtmosphereGas, ExoScan};
        BodyScan {
            body_name: "6 e".to_string(),
            surface_temperature: 156.03,
            radius: 564_697.0,
            exo: Some(ExoScan {
                atmosphere_type: "CarbonDioxideRich".to_string(),
                surface_pressure: 804.6,
                composition: vec![AtmosphereGas { name: "CarbonDioxide".to_string(), percent: 100.0 }],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn value(text: &str, name: &str) -> f32 {
        let prefix = format!("{name} = ");
        text.lines().find_map(|l| l.strip_prefix(&prefix)).unwrap_or_else(|| panic!("no {name} in {text}")).trim().parse().unwrap()
    }

    #[test]
    fn the_star_light_follows_the_body_the_game_says_is_near_not_the_target() {
        let star = |id: i32, name: &str, t: &str, k: f32, r: f32, d: f32| BodyScan {
            body_id: id,
            body_name: name.into(),
            star_type: t.into(),
            surface_temperature: k,
            radius: r * 695_700_000.0,
            distance_from_arrival_ls: d,
            ..Default::default()
        };
        let white = star(1, "X A", "A", 8500.0, 1.8, 0.0);
        let red = star(2, "X B", "M", 3000.0, 0.3, 4000.0);
        let bodies = vec![white, red.clone()];
        let lists: [&[BodyScan]; 1] = [&bodies];
        let g = GameGraphicsSettings::default();
        // The red star marked from beside the white one: the light stays the white's.
        let targeted = look_at(&g, Some(&red), None, &lists, None).star;
        assert!(targeted.colour[2] > 1.0 && targeted.colour[0] < 1.0, "{targeted:?}");
        // Close to the red star (the game names it), its light.
        let near = look_at(&g, Some(&red), Some(&red), &lists, None).star;
        assert!(near.colour[0] > 1.4 && near.colour[2] < 0.5, "{near:?}");
        // In witch space the colour of the star jumped to with the rainbow on it, unless that is off;
        // switched off altogether: nothing.
        let jumping = look_at(&g, Some(&red), Some(&red), &lists, Some("A")).star;
        assert!(jumping.rainbow && jumping.colour[2] > 1.0 && jumping.colour[0] < 1.0, "{jumping:?}");
        let plain = GameGraphicsSettings { star_light_rainbow: false, ..GameGraphicsSettings::default() };
        assert_eq!(look_at(&plain, Some(&red), Some(&red), &lists, Some("A")).star, StarLight { rainbow: false, ..jumping });
        let off = GameGraphicsSettings { star_light: false, ..GameGraphicsSettings::default() };
        assert_eq!(look_at(&off, Some(&red), Some(&red), &lists, None).star, StarLight::NONE);
        assert_eq!(look_at(&off, Some(&red), Some(&red), &lists, Some("A")).star, StarLight::NONE);
    }

    #[test]
    fn the_dust_reach_setting_is_x19_in_metres_within_its_range() {
        let mut g = GameGraphicsSettings::default();
        assert_eq!(dust_at(&g, None).reach_m, 15_000.0);
        g.planet_dust_reach_km = 5;
        assert_eq!(dust_at(&g, None).reach_m, 5_000.0);
        g.planet_dust_reach_km = 99;
        assert_eq!(dust_at(&g, None).reach_m, 30_000.0);
    }

    #[test]
    fn ice_and_metal_tint_and_rock_keeps_the_game_colours() {
        let rock = tint(Makeup { ice: 0.0, rock: 1.0, metal: 0.0 }, 0.7);
        assert_eq!(rock, Tint::NONE);

        let ice = tint(Makeup { ice: 1.0, rock: 0.0, metal: 0.0 }, 0.7);
        assert_eq!(ice, Tint { strength: 0.7, colour: ICE });

        // Hegoea RG-Y b42-1 1: 42 % ice, 39 % rock, 19 % metal.
        let mixed = tint(Makeup { ice: 0.422761, rock: 0.38636, metal: 0.190878 }, 1.0);
        assert!((mixed.strength - 0.613639).abs() < 1e-5);
        assert!(mixed.colour[2] > mixed.colour[0], "leans icy blue: {:?}", mixed.colour);
    }

    #[test]
    fn unscanned_bodies_fall_back_on_their_class() {
        let body = |class: &str| BodyScan { planet_class: class.to_string(), ..Default::default() };
        assert_eq!(makeup(&body("Icy body")).map(|m| m.ice), Some(0.85));
        assert_eq!(makeup(&body("Metal rich body")).map(|m| m.metal), Some(0.70));
        assert_eq!(makeup(&body("Sudarsky class I gas giant")), None);
    }

    #[test]
    fn the_params_file_names_strength_then_colour() {
        let t = Tint { strength: 0.5, colour: [1.1, 1.3, 1.6] };
        let text = params_text("", &Look { tint: t, ..Look::NONE });
        assert!(text.contains("x500 = 0.500\ny500 = 1.100\nz500 = 1.300\nw500 = 1.600\nx503 = 0.000\n"), "{text}");
        // No tint: strength 0 only, the colour is left for 3DMigoto's fade to strength to hide.
        assert_eq!(Tint::NONE.assignments(), vec![("x500", "0.000".to_string())]);
    }

    #[test]
    fn the_file_keeps_the_lines_that_are_not_written_now() {
        let cloudy = planet_clouds::forecast(&[("Nitrogen", 100.0)], 3000.0, 70.0, 1.0).clouds;
        let existing = "; old\nx500 = 0.7\ny500 = 1\nx503 = 0.5\ny503 = 9\nx505 = 0.35\nx507 = 0.3\n\nX1 = 1\n";

        // With clouds and a tint all eighteen are written (and the dust's x515) and x505 stays; stale x, x503, y503 don't double up.
        let tinted = Tint { strength: 0.5, colour: [1.1, 1.3, 1.6] };
        let star = StarLight { colour: [1.9, 0.8, 0.2], strength: 1.0, rainbow: false };
        let text = params_text(existing, &Look { tint: tinted, clouds: cloudy, dust: Dust::NONE, detail: Detail::NONE, star });
        let names: Vec<_> = text.lines().filter_map(|l| l.split_once(" = ")).map(|(n, _)| n).collect();
        assert_eq!(names, ["x500", "y500", "z500", "w500", "x503", "y503", "z503", "w503", "x504", "y504", "w504", "z505", "x507", "w507", "y507", "x508", "x509", "w509", "x515", "x511", "z511", "x520", "w520", "x526", "y526", "z526", "w526", "x527", "x505", "X1"], "{text}");
        assert!(text.contains("x505 = 0.35\n") && text.contains("y503 = 8.000\n"), "{text}");

        // Without clouds only x503 and x507 are ours: the old look of the clouds (y503) is kept, the fog goes.
        let text = params_text(existing, &Look::NONE);
        assert!(text.contains("x503 = 0.000\n") && !text.contains("x503 = 0.5"), "{text}");
        assert!(text.contains("x507 = 0.000\n") && !text.contains("x507 = 0.3") && !text.contains("w507 =") && !text.contains("y507 =") && !text.contains("x508 =") && !text.contains("x509 ="), "{text}");
        assert!(text.contains("y503 = 9\n") && text.contains("x505 = 0.35\n"), "{text}");
        assert_eq!((text.matches("x503 =").count(), text.matches("x507 =").count()), (1, 1));
        // The same for the tint: x = 0 is ours, the colour (y) stays as the file had it.
        assert!(text.contains("x500 = 0.000\n") && text.contains("y500 = 1\n"), "{text}");
    }

    #[test]
    fn the_old_row_numbers_in_an_existing_file_are_moved_up_by_500() {
        // A file from before 0.43.81: the tint on row 0, the hand-tuned glow x5, a foreign line.
        let existing = "x = 0.7\ny = 1.1\nx3 = 0.5\nx5 = 0.35\n$\\ShaderFixes\\edcas.ini\\ground_type = 3\nX1 = 1\n";
        let text = params_text(existing, &Look::NONE);
        assert!(text.contains("x505 = 0.35\n") && !text.contains("x5 ="), "{text}");
        assert!(text.contains("y500 = 1.1\n") && !text.contains("\ny = "), "{text}");
        assert_eq!(text.matches("x500 =").count(), 1, "{text}");
        assert_eq!(text.matches("x503 =").count(), 1, "{text}");
        assert!(text.contains("$\\ShaderFixes\\edcas.ini\\ground_type = 3\n") && text.contains("X1 = 1\n"), "{text}");
        assert_eq!(moved_row("w"), "w500");
        assert_eq!(moved_row("z25"), "z525");
        assert_eq!(moved_row("x505"), "x505");
    }

    /// A body as the journal's scan describes it: class, surface pressure (Pa) and volcanism.
    fn ground_of(class: &str, pressure: f32, volcanism: &str) -> BodyScan {
        use crate::journal_reader::ExoScan;
        BodyScan {
            planet_class: class.to_string(),
            volcanism: volcanism.to_string(),
            exo: Some(ExoScan { surface_pressure: pressure, ..Default::default() }),
            ..Default::default()
        }
    }

    #[test]
    fn the_ground_follows_class_air_and_volcanism() {
        use GroundType::*;
        let cases = [
            // 7 c in Pro Thaa RR-W d1-67: CO2, 4.7 kPa.
            ("Rocky body", 4700.0, "", Some(Sand)),
            ("High metal content body", 846.0, "", Some(Sand)),
            ("High metal content body", 5.0, "", Some(Regolith)),
            ("Rocky body", 0.0, "", Some(Regolith)),
            ("Metal rich body", 0.0, "", Some(Regolith)),
            ("Rocky body", 99.9, "", Some(Regolith)),
            ("Rocky body", 100.0, "", Some(Sand)),
            ("Rocky body", 0.0, "minor rocky magma volcanism", Some(Lava)),
            ("High metal content body", 2000.0, "major silicate vapour geysers volcanism", Some(Lava)),
            ("Metal rich body", 0.0, "metallic magma volcanism", Some(Lava)),
            // Cryovolcanism leaves the ground as it is.
            ("Rocky body", 0.0, "minor water magma volcanism", Some(Regolith)),
            ("Icy body", 0.0, "", Some(Ice)),
            ("Icy body", 0.0, "major water geysers volcanism", Some(Ice)),
            ("Icy body", 1354.0, "", Some(Frost)),
            ("Rocky ice body", 0.0, "", Some(RockyIce)),
            ("Rocky ice body", 1149.0, "", Some(RockyIce)),
            ("Sudarsky class I gas giant", 0.0, "", None),
            ("", 0.0, "", None),
        ];
        for (class, pressure, volcanism, want) in cases {
            assert_eq!(ground_type(&ground_of(class, pressure, volcanism)), want, "{class} {pressure} Pa '{volcanism}'");
        }
        // Without the scan's details (a body from the server) the atmosphere's name tells.
        let server = |atmosphere: &str| BodyScan { planet_class: "Rocky body".into(), atmosphere: atmosphere.into(), ..Default::default() };
        assert_eq!(ground_type(&server("")), Some(Regolith));
        assert_eq!(ground_type(&server("thin carbon dioxide atmosphere")), Some(Sand));
        // The slices are the textures': names and numbers in ground_detail_tex.py's order.
        let names: Vec<_> = [Sand, Regolith, Ice, Frost, RockyIce, Lava].iter().map(|t| (t.index(), t.name())).collect();
        assert_eq!(names, [(0, "sand"), (1, "regolith"), (2, "ice"), (3, "frost"), (4, "rocky_ice"), (5, "lava")]);
    }

    #[test]
    fn the_detail_is_a_strength_and_a_type_that_does_not_fade() {
        let lava = Detail { ground: Some(GroundType::Lava), strength: 1.0 };
        let on: Vec<_> = lava.assignments().into_iter().map(|(n, v)| format!("{n} = {v}")).collect();
        assert_eq!(on, ["x511 = 1.000", "z511 = 1.000", "$\\ShaderFixes\\edcas.ini\\ground_type = 5", "x520 = 0", "w520 = 0"]);
        // Off (the setting, no ground or no strength): too small to see, 0 would be the shader's 1; the type stays.
        for off in [Detail::NONE, Detail { ground: Some(GroundType::Ice), strength: 0.0 }, Detail { ground: None, strength: 1.0 }] {
            // The ice look (gloss, sparkle, colours: 0 reads as on) goes off with it.
            assert_eq!(off.assignments(), vec![("x511", "0.0001".to_string()), ("z511", "0.0001".to_string()), ("x520", "-1".to_string()), ("w520", "-1".to_string())]);
        }
        // The file keeps the type's line when the detail goes off, and ours replace the file's.
        let text = params_text("x511 = 0.5\n$\\ShaderFixes\\edcas.ini\\ground_type = 2\n", &Look { detail: lava, ..Look::NONE });
        assert!(text.contains("x511 = 1.000\n") && !text.contains("x511 = 0.5") && text.contains("ground_type = 5\n") && !text.contains("ground_type = 2"), "{text}");
        let text = params_text(&text, &Look::NONE);
        assert!(text.contains("x511 = 0.0001\n") && text.contains("x520 = -1\n") && text.contains("w520 = -1\n") && text.contains("ground_type = 5\n"), "{text}");
        // Back on: a -1 from before is cleared; a hand-set y520 stays.
        let text = params_text(&format!("{text}y520 = 0.3\n"), &Look { detail: lava, ..Look::NONE });
        assert!(text.contains("x520 = 0\n") && text.contains("w520 = 0\n") && !text.contains("= -1") && text.contains("y520 = 0.3\n"), "{text}");
    }

    #[test]
    fn the_settings_switch_the_detail_and_scale_it() {
        let mut g = GameGraphicsSettings::default();
        assert!(g.ground_detail, "on by default");
        let rock = ground_of("Rocky body", 0.0, "");
        assert_eq!(detail_at(&g, Some(&rock)), Detail { ground: Some(GroundType::Regolith), strength: 1.0 });
        g.ground_detail_strength = 150;
        assert_eq!(detail_at(&g, Some(&rock)).strength, 1.5);
        g.ground_detail_strength = 0;
        assert_eq!(detail_at(&g, Some(&rock)).assignments()[0], ("x511", "0.0001".to_string()));
        g.ground_detail_strength = 100;
        // Unknown body, or one without ground: off.
        assert_eq!(detail_at(&g, None), Detail::NONE);
        assert_eq!(detail_at(&g, Some(&ground_of("Sudarsky class II gas giant", 0.0, ""))), Detail::NONE);
        g.ground_detail = false;
        assert_eq!(detail_at(&g, Some(&rock)), Detail::NONE);
    }

    #[test]
    fn the_clouds_follow_the_body_in_status_and_the_settings() {
        use crate::journal_reader::{AtmosphereGas, ExoScan};
        let mut journal = JournalData::new();
        journal.bodies.push(BodyScan {
            body_name: "Mars B 1".to_string(),
            surface_temperature: 210.0,
            exo: Some(ExoScan {
                atmosphere_type: "CarbonDioxideRich".to_string(),
                surface_pressure: 600.0,
                composition: vec![AtmosphereGas { name: "CarbonDioxide".to_string(), percent: 95.0 }],
                ..Default::default()
            }),
            ..Default::default()
        });
        let mut settings = test_settings();
        assert_eq!(GroundTint::cloud_status(&settings, &journal), "", "off");
        settings.game_graphics.planet_clouds = true;
        assert_eq!(GroundTint::cloud_status(&settings, &journal), "", "not at a body");
        assert_eq!(GroundTint::wanted_clouds(&settings, &journal).clouds, Clouds::NONE);
        journal.status.body_name = "Mars B 1".to_string();
        let f = GroundTint::wanted_clouds(&settings, &journal);
        assert!(f.clouds.coverage > 0.0);
        assert!((f.clouds.opacity - 0.689 * 0.7 * 0.18).abs() < 0.002, "{:?}", f.clouds);
        assert!(f.clouds.volumetric, "volumetric is the default");
        settings.game_graphics.planet_clouds_volumetric = false;
        assert!(!GroundTint::wanted_clouds(&settings, &journal).clouds.volumetric);
        settings.game_graphics.planet_clouds_volumetric = true;
        assert_eq!(GroundTint::cloud_status(&settings, &journal), "Clouds: carbon dioxide ice, high, sparse · no ground fog here");
        settings.game_graphics.planet_clouds = false;
        assert_eq!(GroundTint::wanted_clouds(&settings, &journal).clouds, Clouds::NONE);
    }

    #[test]
    fn the_tint_follows_the_body_in_status_and_the_settings() {
        use crate::journal_reader::BodyComposition;
        let mut journal = JournalData::new();
        journal.bodies.push(BodyScan {
            body_name: "Hegoea RG-Y b42-1 1".to_string(),
            composition: Some(BodyComposition { ice: 0.422761, rock: 0.38636, metal: 0.190878 }),
            ..Default::default()
        });
        let mut settings = test_settings();
        settings.game_graphics.ground_tint = true;

        assert_eq!(wanted(&settings, &journal), Tint::NONE, "not at a body");
        journal.status.body_name = "Hegoea RG-Y b42-1 1".to_string();
        let at = wanted(&settings, &journal);
        assert!((at.strength - 0.7 * 0.613639).abs() < 1e-5, "{at:?}");
        settings.game_graphics.ground_tint = false;
        assert_eq!(wanted(&settings, &journal), Tint::NONE, "switched off");
    }

    #[test]
    fn the_file_follows_the_tint_and_is_reset_when_edcas_closes() {
        let game = game_folder("ground-tint-file");
        let mut settings = test_settings();
        settings.game_graphics.ground_tint = true;
        settings.game_graphics.game_folder = game.display().to_string();
        let mut journal = JournalData::new();
        journal.bodies.push(BodyScan { body_name: "Ice ball".to_string(), planet_class: "Icy body".to_string(), ..Default::default() });
        journal.status.body_name = "Ice ball".to_string();
        let read = || std::fs::read_to_string(game.join(PARAMS_FILE)).unwrap();

        let feed = Feed::new(&settings);
        put_place(&feed, &journal);
        let mut updater = Updater::default();
        tick(&feed, &mut updater);
        assert!(read().contains("x500 = 0.630\ny500 = 1.094"), "{}", read());
        drop(updater);
        assert!(read().contains("x500 = 0.000"), "{}", read());
        std::fs::remove_dir_all(&game).unwrap();
    }

    #[test]
    fn the_star_light_and_the_rainbow_are_reset_when_edcas_closes_in_open_space() {
        let game = game_folder("ground-tint-star-reset");
        let mut settings = test_settings();
        settings.game_graphics.game_folder = game.display().to_string();
        let mut journal = JournalData::new();
        journal.bodies.push(BodyScan { body_id: 0, body_name: "Red".to_string(), star_type: "M".to_string(), surface_temperature: 3000.0, ..Default::default() });
        let read = || std::fs::read_to_string(game.join(PARAMS_FILE)).unwrap();

        // At no body (no tint, clouds, dust or detail), the star's light alone, in a jump with the rainbow.
        let feed = Feed::new(&settings);
        let mut place = Place::of(&journal);
        place.jump = Some("K".to_string());
        *feed.place().lock().unwrap() = place;
        let mut updater = Updater::default();
        tick(&feed, &mut updater);
        assert!(value(&read(), "w526") > 0.0 && value(&read(), "x527") == 1.0, "{}", read());
        drop(updater);
        assert!(value(&read(), "w526") == 0.0 && value(&read(), "x527") == 0.0, "the next system's light is not EDCAS's to know: {}", read());
        std::fs::remove_dir_all(&game).unwrap();
    }

    #[test]
    fn tint_and_clouds_share_the_file_and_the_clouds_reset_on_close() {
        let game = game_folder("ground-tint-clouds");
        std::fs::write(game.join(PARAMS_FILE), "x500 = 0.7\nx503 = 0.5\nx505 = 0.35\n").unwrap();
        let mut settings = test_settings();
        settings.game_graphics.ground_tint = true;
        settings.game_graphics.planet_clouds = true;
        settings.game_graphics.game_folder = game.display().to_string();
        let mut journal = JournalData::new();
        journal.bodies.push(cold_n2());
        let read = || std::fs::read_to_string(game.join(PARAMS_FILE)).unwrap();

        let feed = Feed::new(&settings);
        put_place(&feed, &journal);
        let mut updater = Updater::default();
        tick(&feed, &mut updater);
        assert!(read().contains("x503 = 0.000") && read().contains("x505 = 0.35"), "not at a body: {}", read());

        // Arriving at the body writes the full values at once (3DMigoto does the fading).
        journal.status.body_name = "Cold N2".to_string();
        put_place(&feed, &journal);
        tick(&feed, &mut updater);
        let text = read();
        assert!(text.contains("x500 = 0.630") && text.contains("x503 = 0.173") && text.contains("y503 = 8.000"), "{text}");
        assert!(text.contains("z505 = 0.100"), "{text}");
        assert!(text.contains("x505 = 0.35") && text.matches("x503 =").count() == 1, "{text}");

        // A hand edit of x505 survives the next write.
        std::fs::write(game.join(PARAMS_FILE), format!("{}x505 = 0.5\n", text.replace("x505 = 0.35\n", ""))).unwrap();
        settings.game_graphics.planet_clouds_strength = 100;
        feed.set_settings(&settings);
        tick(&feed, &mut updater);
        assert!(read().contains("x505 = 0.5\n") && read().contains("x504 = 0.480"), "{}", read());

        drop(updater);
        let text = read();
        assert!(text.contains("x500 = 0.000") && text.contains("x503 = 0.000") && text.contains("x505 = 0.5\n"), "{text}");
        std::fs::remove_dir_all(&game).unwrap();
    }

    #[test]
    fn the_fog_follows_the_body_the_setting_and_the_lack_of_air() {
        let game = game_folder("ground-tint-fog");
        let mut settings = test_settings();
        settings.game_graphics.planet_clouds = true;
        settings.game_graphics.game_folder = game.display().to_string();
        let read = || std::fs::read_to_string(game.join(PARAMS_FILE)).unwrap();
        // 6 e: pure CO2, 804.6 Pa, 156.03 K, radius 564697 m, here at a strength of 72 %.
        let six_e = six_e();
        settings.game_graphics.planet_clouds_strength = 72;
        let feed = Feed::new(&settings);
        let mut updater = Updater::default();
        *feed.place().lock().unwrap() = Place { body_name: "6 e".to_string(), bodies: vec![six_e, BodyScan { body_name: "Rock".to_string(), ..Default::default() }], ..Default::default() };
        tick(&feed, &mut updater);
        let text = read();
        assert_eq!((value(&text, "x507"), value(&text, "w507")), (0.1, 564_697.0), "{text}");
        assert!(text.contains("x503 = 0.") && value(&text, "x503") > 0.0, "{text}");

        // The setting off: no density, the radius stays.
        settings.game_graphics.planet_fog = false;
        feed.set_settings(&settings);
        tick(&feed, &mut updater);
        assert_eq!((value(&read(), "x507"), value(&read(), "w507")), (0.0, 564_697.0), "{}", read());
        settings.game_graphics.planet_fog = true;
        feed.set_settings(&settings);
        tick(&feed, &mut updater);
        assert_eq!(value(&read(), "x507"), 0.1);

        // An airless body: x503 and x507 go to 0, so the fog doesn't stay on it; the radius is left.
        feed.place().lock().unwrap().body_name = "Rock".to_string();
        tick(&feed, &mut updater);
        let text = read();
        assert_eq!((value(&text, "x503"), value(&text, "x507"), value(&text, "w507")), (0.0, 0.0, 564_697.0), "{text}");

        drop(updater);
        std::fs::remove_dir_all(&game).unwrap();
    }

    #[test]
    fn the_look_settings_start_at_what_the_shader_looked_right_with_on_6_e() {
        let mut g = GameGraphicsSettings { planet_clouds: true, planet_clouds_strength: 72, ..Default::default() };
        let get = |g: &GameGraphicsSettings, name: &str| {
            clouds_at(g, Some(&six_e())).clouds.assignments().into_iter().find(|(n, _)| *n == name).unwrap().1
        };
        let all = |g: &GameGraphicsSettings, names: [&str; 4]| names.map(|n| get(g, n));
        assert_eq!(all(&g, ["x507", "z505", "y507", "x508"]), ["0.100", "0.100", "800", "0.500"]);
        assert_eq!(get(&g, "x509"), "1.000");

        g.planet_fog_density = 200;
        assert_eq!(get(&g, "x507"), "0.200");
        g.planet_fog_density = 0;
        assert_eq!(get(&g, "x507"), "0.000");
        g.planet_fog_density = 100;

        g.planet_fog_depth = 1500;
        assert_eq!(get(&g, "y507"), "1500");
        // Out of range values from an edited settings file are held to the range.
        g.planet_fog_depth = 5;
        assert_eq!(get(&g, "y507"), "100");
        g.planet_fog_depth = 9999;
        assert_eq!(get(&g, "y507"), "3000");
        // The depth is written with the fog off too: only the density goes.
        g.planet_fog = false;
        assert_eq!((get(&g, "x507"), get(&g, "y507")), ("0.000".to_string(), "3000".to_string()));
        g.planet_fog = true;
        g.planet_fog_depth = 800;

        g.planet_cloud_thickness = 300;
        assert_eq!(get(&g, "z505"), "0.300");
        g.planet_cloud_thickness = 0;
        assert_eq!(get(&g, "z505"), "0.000");
        g.planet_cloud_thickness = 100;

        g.planet_cloud_softness = 200;
        assert_eq!(get(&g, "x508"), "1.000");
        // 0 would be the shader's own 0.5, so 0 % writes the least instead.
        g.planet_cloud_softness = 0;
        assert_eq!(get(&g, "x508"), "0.010");
        g.planet_cloud_softness = 9999;
        assert_eq!(get(&g, "x508"), "1.500", "held to 300 %");

        g.planet_canopy_frost_strength = 50;
        assert_eq!(get(&g, "x509"), "0.500");
        g.planet_canopy_frost_strength = 300;
        assert_eq!(get(&g, "x509"), "3.000");
        g.planet_canopy_frost_strength = 9999;
        assert_eq!(get(&g, "x509"), "3.000", "held to 300 %");
        // The shader reads 0 as its default 1, so 0 % and the toggle off write the least instead.
        g.planet_canopy_frost_strength = 0;
        assert_eq!(get(&g, "x509"), "0.001");
        g.planet_canopy_frost_strength = 100;
        g.planet_canopy_frost = false;
        assert_eq!(get(&g, "x509"), "0.001");
        g.planet_canopy_frost = true;
        assert_eq!(get(&g, "x509"), "1.000");

        // Without a body nothing of it is written: x503 and x507 alone.
        let none = clouds_at(&g, None).clouds.assignments();
        assert_eq!(none.iter().map(|(n, _)| *n).collect::<Vec<_>>(), ["x503", "x507"]);
    }

    #[test]
    fn changing_a_look_setting_rewrites_the_file() {
        let game = game_folder("ground-tint-look");
        let mut settings = test_settings();
        settings.game_graphics.planet_clouds = true;
        settings.game_graphics.planet_clouds_strength = 72;
        settings.game_graphics.game_folder = game.display().to_string();
        let read = || std::fs::read_to_string(game.join(PARAMS_FILE)).unwrap();
        let feed = Feed::new(&settings);
        let mut updater = Updater::default();
        *feed.place().lock().unwrap() = Place { body_name: "6 e".to_string(), bodies: vec![six_e()], ..Default::default() };
        tick(&feed, &mut updater);
        let text = read();
        assert_eq!((value(&text, "x507"), value(&text, "z505"), value(&text, "y507"), value(&text, "x508")), (0.1, 0.1, 800.0, 0.5), "{text}");
        assert_eq!(value(&text, "x509"), 1.0, "{text}");

        for (change, name, wanted) in [
            (&|g: &mut GameGraphicsSettings| g.planet_fog_density = 200, "x507", 0.2),
            (&|g: &mut GameGraphicsSettings| g.planet_fog_depth = 1500, "y507", 1500.0),
            (&|g: &mut GameGraphicsSettings| g.planet_cloud_thickness = 200, "z505", 0.2),
            (&|g: &mut GameGraphicsSettings| g.planet_cloud_softness = 0, "x508", 0.01),
            (&|g: &mut GameGraphicsSettings| g.planet_canopy_frost_strength = 50, "x509", 0.5),
            (&|g: &mut GameGraphicsSettings| g.planet_canopy_frost = false, "x509", 0.001),
            (&|g: &mut GameGraphicsSettings| {
                g.planet_canopy_frost = true;
                g.planet_canopy_frost_strength = 0;
            }, "x509", 0.001),
            (&|g: &mut GameGraphicsSettings| g.planet_canopy_frost_strength = 200, "x509", 2.0),
        ] as [(&dyn Fn(&mut GameGraphicsSettings), &str, f32); 8]
        {
            change(&mut settings.game_graphics);
            feed.set_settings(&settings);
            tick(&feed, &mut updater);
            assert_eq!(value(&read(), name), wanted, "{name}: {}", read());
        }
        // The earlier changes stay in the later writes.
        let text = read();
        assert_eq!((value(&text, "x507"), value(&text, "y507"), value(&text, "z505"), value(&text, "x508")), (0.2, 1500.0, 0.2, 0.01), "{text}");
        assert_eq!(value(&text, "x509"), 2.0, "{text}");

        drop(updater);
        std::fs::remove_dir_all(&game).unwrap();
    }

    #[test]
    fn a_fixed_colour_keeps_the_ground_light_and_stays_after_closing() {
        let ice = fixed((0xa8, 0xc8, 0xff), 0.7);
        let brightness = 0.299 * ice.colour[0] + 0.587 * ice.colour[1] + 0.114 * ice.colour[2];
        assert!((brightness - 1.0).abs() < 1e-5, "{ice:?}");
        assert!(ice.colour[2] > ice.colour[0]);
        assert_eq!(fixed((0, 0, 0), 0.7), Tint::NONE);

        let game = game_folder("ground-tint-fixed");
        let mut settings = test_settings();
        settings.game_graphics.ground_tint = true;
        settings.game_graphics.ground_tint_mode = TintMode::Fixed;
        settings.game_graphics.game_folder = game.display().to_string();
        let feed = Feed::new(&settings); // at no body: a fixed colour doesn't care
        let mut updater = Updater::default();
        tick(&feed, &mut updater);
        drop(updater);
        let text = std::fs::read_to_string(game.join(PARAMS_FILE)).unwrap();
        assert!(text.contains("x500 = 0.700"), "{text}");
        std::fs::remove_dir_all(&game).unwrap();
    }

    #[test]
    fn the_game_folder_is_found_from_a_proton_journal_folder() {
        let tmp = std::env::temp_dir().join(format!("edcas-ground-tint-{}", std::process::id()));
        let steamapps = tmp.join("steamapps");
        let journal = steamapps.join("compatdata/359320/pfx/drive_c/users/steamuser/Saved Games/Frontier Developments/Elite Dangerous");
        let game = steamapps.join("common/Elite Dangerous/Products/elite-dangerous-odyssey-64");
        std::fs::create_dir_all(&journal).unwrap();
        std::fs::create_dir_all(&game).unwrap();
        let mut settings = test_settings();
        settings.journal_reader.journal_directory = journal.display().to_string();
        assert_eq!(game_dir(&settings), Some(game.clone()));
        settings.game_graphics.game_folder = "/elsewhere".to_string();
        assert_eq!(game_dir(&settings), Some(PathBuf::from("/elsewhere")));
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn the_writer_writes_the_wanted_values_when_they_change_without_any_gui() {
        let game = game_folder("ground-tint-writes");
        let mut settings = test_settings();
        settings.game_graphics.planet_clouds = true;
        settings.game_graphics.game_folder = game.display().to_string();
        let read = || std::fs::read_to_string(game.join(PARAMS_FILE)).unwrap();

        // Only the feed: no App, no GUI frame anywhere.
        let feed = Feed::new(&settings);
        let mut updater = Updater::default();
        tick(&feed, &mut updater);
        assert_eq!(value(&read(), "x503"), 0.0, "written as soon as the writer runs");

        // The journal reader publishes the arrival at the planet: the values are there at
        // once, in full, for 3DMigoto to fade to.
        *feed.place().lock().unwrap() = Place { body_name: "Cold N2".to_string(), bodies: vec![cold_n2()], ..Default::default() };
        tick(&feed, &mut updater);
        let text = read();
        assert_eq!(value(&text, "x503"), 0.173, "{text}");
        assert_eq!(value(&text, "y503"), 8.0, "{text}");
        assert_eq!(value(&text, "x504"), 0.336, "the slab: {text}");
        assert_eq!(value(&text, "z505"), 0.1, "{text}");

        // Leaving writes x503 = 0 and leaves the clouds' look as it was: only the coverage fades out.
        *feed.place().lock().unwrap() = Place::default();
        tick(&feed, &mut updater);
        let text = read();
        assert_eq!(value(&text, "x503"), 0.0, "{text}");
        assert_eq!(value(&text, "y503"), 8.0, "{text}");
        assert_eq!(value(&text, "x504"), 0.336, "{text}");

        // A settings change is written too.
        *feed.place().lock().unwrap() = Place { body_name: "Cold N2".to_string(), bodies: vec![cold_n2()], ..Default::default() };
        settings.game_graphics.planet_clouds_strength = 100;
        feed.set_settings(&settings);
        tick(&feed, &mut updater);
        assert_eq!(value(&read(), "x504"), 0.48, "full strength: {}", read());

        // Switching the slab off writes the thin shell: z505 = 0 and the thin opacity.
        settings.game_graphics.planet_clouds_volumetric = false;
        feed.set_settings(&settings);
        tick(&feed, &mut updater);
        assert_eq!(value(&read(), "z505"), 0.0, "thin shell: {}", read());
        assert_eq!(value(&read(), "x504"), 0.144, "thin shell: {}", read());
        settings.game_graphics.planet_clouds_volumetric = true;
        feed.set_settings(&settings);
        tick(&feed, &mut updater);
        assert_eq!(value(&read(), "z505"), 0.1, "slab again: {}", read());

        // The file isn't rewritten while nothing changed (3DMigoto would re-read it for nothing).
        let before = std::fs::metadata(game.join(PARAMS_FILE)).unwrap().modified().unwrap();
        thread::sleep(Duration::from_millis(30));
        tick(&feed, &mut updater);
        assert_eq!(std::fs::metadata(game.join(PARAMS_FILE)).unwrap().modified().unwrap(), before);

        drop(updater);
        assert_eq!(value(&read(), "x503"), 0.0);
        std::fs::remove_dir_all(&game).unwrap();
    }

    #[test]
    fn the_writer_thread_follows_the_journal_reader_and_the_settings_by_itself() {
        let game = game_folder("ground-tint-thread");
        let mut settings = test_settings();
        settings.game_graphics.ground_tint = true;
        settings.game_graphics.ground_tint_mode = TintMode::Fixed;
        settings.game_graphics.game_folder = game.display().to_string();
        let read = || std::fs::read_to_string(game.join(PARAMS_FILE)).unwrap_or_default();
        let wait_for = |what: &str, ok: &dyn Fn(&str) -> bool| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !ok(&read()) {
                assert!(Instant::now() < deadline, "{what}: {}", read());
                thread::sleep(Duration::from_millis(20));
            }
        };

        // Nothing here draws a frame or drains a channel: the thread alone gets the values
        // from the place the reader fills and from the settings handed over.
        let tint = GroundTint::start(&settings);
        wait_for("first write", &|t| t.contains("x500 = 0.700"));
        settings.game_graphics.ground_tint = false;
        tint.sync(&settings);
        wait_for("settings change", &|t| value_below(t, "x500", 0.7));
        settings.game_graphics.ground_tint = true;
        settings.game_graphics.ground_tint_mode = TintMode::PerPlanet;
        tint.sync(&settings);
        *tint.place().lock().unwrap() = Place {
            body_name: "Ice ball".to_string(),
            bodies: vec![BodyScan { body_name: "Ice ball".to_string(), planet_class: "Icy body".to_string(), ..Default::default() }],
            ..Default::default()
        };
        wait_for("arrival", &|t| t.contains("y500 = 1.094"));

        drop(tint);
        assert!(read().contains("x500 = 0.000"), "reset when the thread ends: {}", read());
        std::fs::remove_dir_all(&game).unwrap();
    }

    fn value_below(text: &str, name: &str, limit: f32) -> bool {
        text.lines().any(|l| l.strip_prefix(&format!("{name} = ")).and_then(|v| v.trim().parse::<f32>().ok()).is_some_and(|v| v < limit))
    }
}
