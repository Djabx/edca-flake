//! The 3DMigoto shader mods (planet ground tint and detail, clouds, ground fog, dust, canopy frost,
//! star light): EDCAS installs, updates and removes them in Elite's game folder, from Settings →
//! Game Graphics, and patches the game's own shaders that 3DMigoto exported. The values the
//! shaders read at run time are [`crate::ground_tint`]'s.
//!
//! What the mods are is in `scripts/ed-shader/README.md`. Twins in `scripts/ed-shader`, kept in
//! step by the tests at the end of this file:
//!
//! - `install.py`, the by-hand installer this module follows step by step ([`install`],
//!   [`update_shaders`]); `the_embedded_shaders_are_the_ones_install_py_installs` compares
//!   [`SKY_SHADERS`].
//! - `ground_tint.py`: [`tint`], [`detail`], [`gloss`], [`mask`] (and `sparkle_lines`) are its ports.
//!   `detail_makes_what_the_script_makes` compares [`patch`] with `testdata/ground_fake_patched.txt`
//!   (regenerate: `python3 scripts/ed-shader/ground_tint.py crates/client/testdata/ground_fake_ps.txt
//!   crates/client/testdata/ground_fake_patched.txt`); `the_detail_template_is_the_scripts` runs
//!   `ground_tint.py --template` against [`GROUND_DETAIL`].
//! - `star_light.py`: [`star_light`], [`star_light_list`] and the hash lists;
//!   `star_light_py_patches_as_the_port_does` compares the patches byte for byte.
//!
//! `crates/client/shader_mods/` holds byte-identical copies of `scripts/ed-shader`'s `edcas.ini`,
//! `*.hlsl` and `edcas_ground_detail.asm` (the exported client source has no `scripts/`); edit
//! the scripts' files and copy them over, `the_embedded_files_equal_scripts_ed_shader` fails
//! otherwise. The pieces:
//!
//! - **The runtime** is a download built by CI (`/download/edcas-client-3dmigoto-x64.zip`,
//!   checked against `/download/SHA256SUMS`): 3DMigoto 1.4.9 with EDCAS's patched
//!   `d3d11.dll`, laid out as the game folder is.
//! - **EDCAS's own files** are embedded in this binary ([`EDCAS_INI`], [`SKY_SHADERS`]) and
//!   written into `ShaderFixes`, next to `edcas_version.txt` (the client version that wrote
//!   them: a different one means an update is due). The ground detail's textures are made
//!   on the spot ([`crate::ground_detail`]).
//! - **The ground tint and detail** need the game's own ground shaders, which 3DMigoto exports when the
//!   player lands on a planet with `export_shaders=1`. [`install`] switches that on and says
//!   so; [`finish_game_shaders`] turns the export into the tinted copies (with the detail) once it is there,
//!   and the star light's copies of the sun and sky passes ([`star_light`]) as their exports turn up.
//!
//! Everything here blocks: [`start`] runs a job on a thread of its own and reports to the GUI.

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tracing::{info, warn};

use crate::ground_detail;

/// The client version, written next to the installed files.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The Steam launch option that makes Proton load 3DMigoto's DLLs (Windows needs nothing).
pub const STEAM_LAUNCH_OPTION: &str = r#"WINEDLLOVERRIDES="d3d11,d3dcompiler_47=n,b" %command%"#;

const ZIP_NAME: &str = "edcas-client-3dmigoto-x64.zip";
const SUMS_NAME: &str = "SHA256SUMS";
const DEFAULT_BASE_URL: &str = "https://edcas.de";

/// The game folder holds this when it is Elite's.
const GAME_EXE: &str = "EliteDangerous64.exe";
pub(crate) const FIXES: &str = "ShaderFixes";
pub(crate) const VERSION_FILE: &str = "edcas_version.txt";
pub(crate) const PARAMS_FILE: &str = "edcas_params.txt";
pub(crate) const INI_FILE: &str = "edcas.ini";
const PARAMS_FADE: &str = "4";
const INCLUDE: &str = "ShaderFixes\\edcas.ini";

/// The game's ground shaders (game version 2026.09.03), as `scripts/ed-shader/ground_tint.py`.
pub const GROUND_SHADERS: [&str; 3] = ["7ce82fabb300f52e", "50fd337fd22daec6", "695ededf10e98696"];

/// The game's deferred sun pass (game version 2026.11.09) in its two forms: open space and
/// airless ground, and over a planet with an atmosphere. Both take the sun's colour from
/// `cb2[40]` (`scripts/ed-shader/README-ELITE.md`, "The sun's colour"); [`star_light`] tints it.
pub const SUN_SHADERS: [&str; 2] = ["e569985b041c66db", "968737c608cc1c08"];

/// The game's sky pass and its two head-look variants: the sunlight the atmosphere scatters is
/// `cb1[8]` there, a constant whatever the star; [`star_light`] tints it.
pub const ATMOSPHERE_SHADERS: [&str; 3] = ["236d6421041d3892", "7932556525410d85", "8c9b2131f7ef329f"];

/// The game's forward-lit shaders that take the sun's colour from the frame constant `cb1[293]`
/// (the same value as the sun pass's `cb2[40]`; `cb1[291]` is the light's direction): the
/// planetary rings (`5884b7af`, `0f9b6dd4`, found by pixel history on a ring, 2026-11-09) and
/// the others of the ShaderCache that read it, bar the three ground shaders (they read it once
/// in the G-buffer pass and carry the ground patch; still open). Then the nine exported inside a
/// ring (2026-11-09): the ring's fog pass `bfdd0b84` (the grey haze that covered the asteroids; it
/// reads the row three times: luminance, self-dot, multiply) and the ring's asteroid materials.
/// Every cached pixel shader reading the row is here; [`star_light`] tints each read.
pub const FORWARD_SHADERS: [&str; 21] = [
    "5884b7af49750195",
    "0f9b6dd47e012780",
    "ba1c9d5a60b02eb0",
    "d7e5ff175bc127a8",
    "f5938f31368af521",
    "024a16df97355112",
    "19d5a1562d5dc57a",
    "1aaeb7a5c0a72063",
    "3c3c269ed361e6a3",
    "585da4349c0b97d9",
    "62c11e6c13cb8434",
    "c82fa6c0e4d3eb98",
    "bfdd0b8425d4e069",
    "19bd8b0ae8ef2f90",
    "5726c31ce6eb0586",
    "dfe746f9b1af9d84",
    "733d90eac46d25c3",
    "ed8e230ed3175188",
    "f39c637091a03c6b",
    "898e00b2b6f63c80",
    "8a60a7639589a64b",
];

/// The game's shaders that take the sun from a light list instead of a constant: the cockpit
/// glass (`953f692f`, `README-ELITE.md`: a raw buffer at `t1`, 48-byte records, the colour at byte
/// 0 and the direction at byte 16, record 0 the sun; the colour load of each record is followed by
/// the direction's), and `3aa99569d49e7e14`, a 5984-line forward shader with the same list seen
/// at a parked ship's canopy from outside (`ed_frame511315`, events 3216 and 3218).
/// [`star_light_list`] tints each colour load.
pub const LIGHT_LIST_SHADERS: [&str; 2] = ["953f692f27531387", "3aa99569d49e7e14"];

/// Where one of Elite's shaders takes the sun's colour from, for [`star_light_shaders`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SunSource {
    /// A constant register, every read of it tinted ([`star_light`]).
    Row(&'static str),
    /// A light list in a raw buffer slot, each record's colour load tinted ([`star_light_list`]).
    List(&'static str),
}

impl SunSource {
    pub fn patch(self, asm: &str) -> Result<String, String> {
        match self {
            SunSource::Row(source) => star_light(asm, source),
            SunSource::List(slot) => star_light_list(asm, slot),
        }
    }
}

/// Game-shader replacements of EDCAS's from before client 0.43.39 (the landing dust redraw,
/// removed 2026-11-03, `scripts/ed-shader/README.md` "Landing dust"): 3DMigoto loads a file in
/// `ShaderFixes` by the game shader's hash for as long as it exists, so installing and
/// uninstalling delete them.
const STALE_SHADERS: [&str; 2] = ["11fd8229e9918376-ps_replace.txt", "133ffe1001522288-vs.txt"];

/// `ShaderFixes/edcas.ini`, as in `scripts/ed-shader`. The copies in `crates/client/shader_mods`
/// are what gets embedded (the exported client source has no `scripts/`); a test fails
/// when they differ from `scripts/ed-shader`.
pub const EDCAS_INI: &str = include_str!("../shader_mods/edcas.ini");

/// The ground detail's assembly (`scripts/ed-shader/ground_tint.py --template`): the texture
/// declarations, then the block that goes into each ground shader, with the [`DETAIL_TEMPS`]
/// scratch names, `{src}` and `{normal}` for the registers. A copy like the others: a test fails when it differs.
pub const GROUND_DETAIL: &str = include_str!("../shader_mods/edcas_ground_detail.asm");

/// Scratch registers the detail block uses, by their names in the template, in register order
/// (`ground_tint.py` `DETAIL_TEMPS`; `{m}` holds the snow's mask, `{n}` to `{z}` are the hex-tiling's).
const DETAIL_NAMES: &str = "abcdefghijkqmnoprstuvwz";
const DETAIL_TEMPS: u32 = DETAIL_NAMES.len() as u32;

/// The size of the ground textures' colour and relief layers and of the stones (pixels):
/// 1024 each makes 46 MB (see `ground_detail`). Tests use small ones: the real ones take
/// minutes to make in a debug build.
const TEXTURE_SIZE: usize = if cfg!(test) { 32 } else { 1024 };
/// The snow's height map (pixels a side: 2048 is 4 m at 2 mm a texel, 17 MB in two files).
const SNOW_TEXTURE_SIZE: usize = if cfg!(test) { 64 } else { ground_detail::SNOW_SIZE };

/// The ground textures before the ground types (one colour, one relief, one stones' texture) and the snow crust's mask (0.43.52: the snow
/// has a height map of its own).
const OLD_TEXTURES: [&str; 4] = ["edcas_ground_detail.dds", "edcas_ground_relief.dds", "edcas_ground_pebbles.dds", "edcas_ground_crust.dds"];

/// Every shader `edcas.ini` runs or they include, as `SKY_SHADERS` in `install.py` (a test
/// checks they are the same list).
pub const SKY_SHADERS: [(&str, &str); 18] = [
    ("edcas_noise.hlsl", include_str!("../shader_mods/edcas_noise.hlsl")),
    ("edcas_clouds.hlsl", include_str!("../shader_mods/edcas_clouds.hlsl")),
    ("edcas_clouds_final.hlsl", include_str!("../shader_mods/edcas_clouds_final.hlsl")),
    ("edcas_clouds_composite.hlsl", include_str!("../shader_mods/edcas_clouds_composite.hlsl")),
    ("edcas_fog.hlsl", include_str!("../shader_mods/edcas_fog.hlsl")),
    ("edcas_dust.hlsl", include_str!("../shader_mods/edcas_dust.hlsl")),
    ("edcas_ground_grid.hlsl", include_str!("../shader_mods/edcas_ground_grid.hlsl")),
    ("edcas_ground_ref.hlsl", include_str!("../shader_mods/edcas_ground_ref.hlsl")),
    ("edcas_canopy_contact.hlsl", include_str!("../shader_mods/edcas_canopy_contact.hlsl")),
    ("edcas_canopy_frost.hlsl", include_str!("../shader_mods/edcas_canopy_frost.hlsl")),
    ("edcas_canopy_ice.hlsl", include_str!("../shader_mods/edcas_canopy_ice.hlsl")),
    ("edcas_lights.hlsl", include_str!("../shader_mods/edcas_lights.hlsl")),
    ("edcas_lamps.hlsl", include_str!("../shader_mods/edcas_lamps.hlsl")),
    ("edcas_cockpit_light.hlsl", include_str!("../shader_mods/edcas_cockpit_light.hlsl")),
    ("edcas_fullscreen_vs.hlsl", include_str!("../shader_mods/edcas_fullscreen_vs.hlsl")),
    ("edcas_sky_vs.hlsl", include_str!("../shader_mods/edcas_sky_vs.hlsl")),
    ("edcas_copy.hlsl", include_str!("../shader_mods/edcas_copy.hlsl")),
    ("edcas_debug_overlay.hlsl", include_str!("../shader_mods/edcas_debug_overlay.hlsl")),
];

// ------------------------------------------------------------------ status

/// What the game folder has of the shader mods.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    NotInstalled,
    /// Installed by this client version; `ground_tint` is false while the tinted ground
    /// shaders are still to be made (the player has to land on a planet once), `star_light`
    /// counts the star light's shader copies made so far (of [`StarLightShaders::TOTAL`]).
    Installed { ground_tint: bool, star_light: usize },
    /// Installed by another client version (`None`: before versions were recorded, by hand).
    UpdateAvailable { installed: Option<String> },
}

/// What is in `dir` (the game folder), for the client at `version`.
pub fn status(dir: &Path, version: &str) -> Status {
    if !dir.join(FIXES).join(INI_FILE).is_file() {
        return Status::NotInstalled;
    }
    let installed = fs::read_to_string(dir.join(FIXES).join(VERSION_FILE)).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    if installed.as_deref() != Some(version) {
        return Status::UpdateAvailable { installed };
    }
    Status::Installed { ground_tint: ground_tint_done(dir), star_light: star_light_made(dir) }
}

/// Whether all tinted ground shaders are in `ShaderFixes`.
fn ground_tint_done(dir: &Path) -> bool {
    GROUND_SHADERS.iter().all(|h| dir.join(FIXES).join(format!("{h}-ps.txt")).is_file())
}

/// How many of the star light's shader copies are in `ShaderFixes` (of [`StarLightShaders::TOTAL`]).
fn star_light_made(dir: &Path) -> usize {
    star_light_shaders().filter(|(h, _)| dir.join(FIXES).join(format!("{h}-ps.txt")).is_file()).count()
}

/// The star light's game shaders with where each takes the sun's colour from.
fn star_light_shaders() -> impl Iterator<Item = (&'static str, SunSource)> {
    SUN_SHADERS
        .iter()
        .map(|h| (*h, SunSource::Row("cb2[40]")))
        .chain(ATMOSPHERE_SHADERS.iter().map(|h| (*h, SunSource::Row("cb1[8]"))))
        .chain(FORWARD_SHADERS.iter().map(|h| (*h, SunSource::Row("cb1[293]"))))
        .chain(LIGHT_LIST_SHADERS.iter().map(|h| (*h, SunSource::List("t1"))))
}

// ------------------------------------------------------------------ ground tint

/// The tinted copy of a ground shader, made from the one 3DMigoto exported (`asm`): the ground
/// colour is blended towards its brightness × a tint colour that EDCAS sets in `IniParams` (`x`
/// strength, `y z w` colour), in place, just before the normal is encoded, where the
/// detail goes too (after it): Elite's colour, the tint, then the detail and the ice colours.
/// `mov o1.xyzw, rN.xyzw` stays as the game has it. Port of `ground_tint.tint` in
/// `scripts/ed-shader`.
pub fn tint(asm: &str) -> Result<String, String> {
    let mut lines = asm_lines(asm);
    let colour: Vec<(usize, &str)> = lines.iter().enumerate().filter_map(|(i, l)| colour_move(l).map(|r| (i, r))).collect();
    let (Some((temps_line, n)), [(colour_line, src)], Some((encode_line, _))) = (dcl_temps(&lines), colour.as_slice(), encode_of(&lines)) else {
        return Err("this doesn't look like the ground shader EDCAS knows (no single 'mov o1.xyzw, rN.xyzw', or no encoding of the normal); the game may have changed it".into());
    };
    if asm.contains("t120") {
        return Err("this shader already reads t120 (IniParams); is it tinted already?".into());
    }
    let (colour_line, src) = (*colour_line, src.to_string());
    // From the encoding to the move into o1 nothing may change the colour's rgb.
    if lines[encode_line..colour_line.max(encode_line)].iter().any(|l| changes_colour(l, &src)) {
        return Err(format!("the ground colour {src} changes after the normal is encoded; the game may have changed the shader"));
    }
    let Some(last_resource) = lines.iter().rposition(|l| l.starts_with("dcl_resource_")) else {
        return Err("this shader has no 'dcl_resource_' declaration to add the IniParams texture after".into());
    };
    let (a, b) = (format!("r{n}"), format!("r{}", n + 1));

    lines[temps_line] = format!("dcl_temps {}", n + 2);
    let block = [
        "// EDCAS planet ground tint: x500 = strength, y500 z500 w500 = tint colour".to_string(),
        format!("ld_indexable(texture1d)(float,float,float,float) {a}.xyzw, l(500, 0, 0, 0), t120.xyzw"),
        format!("dp3 {b}.w, {src}.xyzx, l(0.299000, 0.587000, 0.114000, 0.000000)"),
        format!("mul {b}.xyz, {b}.wwww, {a}.yzwy"),
        format!("min {b}.xyz, {b}.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)"),
        format!("add {b}.xyz, {b}.xyzx, -{src}.xyzx"),
        format!("mad {src}.xyz, {a}.xxxx, {b}.xyzx, {src}.xyzx"),
    ];
    // The later line first, so the earlier one's index stays right.
    lines.splice(encode_line..encode_line, block);
    lines.insert(last_resource + 1, T120_DECL.to_string());
    Ok(lines.join("\n"))
}

/// A copy of one of Elite's lighting shaders with the sun's colour multiplied by the star's
/// ([`crate::star_light`]: IniParams row 526, `x y z` the colour at luminance 1, `w` the
/// strength; 1 + (colour − 1) × strength, so an unset row changes nothing; `x527` blends it into
/// the witch-space rainbow, see [`tint_block`]). `source` is the
/// register the shader reads the colour from (`cb2[40]` in the sun pass, `cb1[8]` in the sky
/// pass, `cb1[293]` in the forward shaders: rings, glass, effects); every read of it becomes
/// the tinted register, worked out once after the declarations. A constant register is only
/// ever a source operand, so the op does not matter. Proven in the RenderDoc replay before the
/// game ran it (`ed-captures/star-light/`, 2026-11-09).
pub fn star_light(asm: &str, source: &str) -> Result<String, String> {
    let mut lines = unpatched_lines(asm)?;
    let needle = format!("{source}.");
    let reads: Vec<usize> = lines.iter().enumerate().filter(|(_, l)| !l.starts_with("//") && l.contains(&needle)).map(|(i, _)| i).collect();
    let (Some((temps_line, n)), false) = (dcl_temps(&lines), reads.is_empty()) else {
        return Err(format!("this doesn't look like the lighting shader EDCAS knows (no read of {source}, or no dcl_temps); the game may have changed it"));
    };
    let places = TintPlaces::of(&lines)?;
    let (reg, clock) = (format!("r{n}"), format!("r{}", n + 1));
    for &i in &reads {
        lines[i] = lines[i].replace(&needle, &format!("{reg}."));
    }
    // The sun pass has the pixel's screen position in v0 (0 to 1, it scales it by the resolution
    // cb2[1].xy for its texel loads): its rainbow runs in bands out from the screen's centre, with the
    // witch-space clouds that stream past the ship (several colours at once, 2026-11-09: one hue over
    // the whole picture looked flat); the other copies, without it, cycle.
    let swirl = (source == "cb2[40]" && lines.iter().any(|l| l == "dcl_input_ps linear v0.xy")).then_some("v0");
    let mut block = tint_block(&reg, &clock, swirl);
    block.push(format!("mul {reg}.xyz, {reg}.xyzx, {source}.xyzx"));
    Ok(places.insert(lines, temps_line, n, block))
}

/// Where the star light's tint goes into a shader: the block after the last declaration,
/// the IniParams texture's declaration after the last resource's.
struct TintPlaces {
    last_decl: usize,
    last_resource: usize,
}

impl TintPlaces {
    fn of(lines: &[String]) -> Result<Self, String> {
        let (Some(last_decl), Some(last_resource)) = (lines.iter().rposition(|l| l.starts_with("dcl_")), lines.iter().rposition(|l| l.starts_with("dcl_resource_"))) else {
            return Err("this shader has no 'dcl_resource_' declaration to add the IniParams texture after".into());
        };
        Ok(TintPlaces { last_decl, last_resource })
    }

    /// `lines` with the tint `block` and t120 in place and two more temporaries (`r{n}`, `r{n+1}`)
    /// declared on `temps_line`.
    fn insert(&self, mut lines: Vec<String>, temps_line: usize, n: u32, block: Vec<String>) -> String {
        lines[temps_line] = format!("dcl_temps {}", n + 2);
        // The later place first, so the earlier one's index stays right.
        lines.splice(self.last_decl + 1..self.last_decl + 1, block);
        lines.insert(self.last_resource + 1, T120_DECL.to_string());
        lines.join("\n")
    }
}

/// The lines that leave the star's tint in `reg.xyz`: 1 + (colour − 1) × strength, blended by `x527`
/// (0 to 1) into the witch-space rainbow (`clock` is scratch; the hue turns with `z515`, the clock the
/// ini sets every frame, [`RAINBOW_HZ`] times a second: three cosines a third of a turn apart, 0.2 to
/// 0.4: witch space is dark grey with faint colour accents, so the light is a third of the star's with
/// a little hue on top, 2026-11-09). The client writes `x527` 1 during a hyperspace jump together with
/// the colour of the star it is jumping to ([`crate::star_light::StarLight::rainbow`]); 3DMigoto's
/// fade takes both there over seconds, so the colour slides from the old star's into the rainbow, and
/// at arrival, `x527` back at 0, straight into the new star's, as the light comes up when the tunnel
/// opens. With `swirl`, a register holding the pixel's screen position (0 to 1; `cb2[1].xy` the
/// resolution, for the aspect), the hue also falls with the distance from the screen's centre (one
/// turn per screen height), so the bands run outwards, towards the player, like the clouds streaming
/// past the ship.
fn tint_block(reg: &str, clock: &str, swirl: Option<&str>) -> Vec<String> {
    let mut v = vec![
        "// EDCAS star light: x526 y526 z526 the star's colour at luminance 1, w526 the strength (0 = the game's light), x527 how far into the witch-space rainbow on z515's clock".to_string(),
        format!("ld_indexable(texture1d)(float,float,float,float) {reg}.xyzw, l(526, 0, 0, 0), t120.xyzw"),
        format!("ld_indexable(texture1d)(float,float,float,float) {clock}.xyzw, l(515, 0, 0, 0), t120.xyzw"),
        format!("mul {clock}.x, {clock}.z, l({RAINBOW_HZ:.6})"),
        format!("frc {clock}.x, {clock}.x"),
    ];
    if let Some(uv) = swirl {
        v.extend([
            format!("add {clock}.yz, {uv}.xxyx, l(0.000000, -0.500000, -0.500000, 0.000000)"),
            format!("div {clock}.w, cb2[1].x, cb2[1].y"),
            format!("mul {clock}.y, {clock}.y, {clock}.w"),
            format!("dp2 {clock}.w, {clock}.yzyy, {clock}.yzyy"),
            format!("sqrt {clock}.w, {clock}.w"),
            format!("mad {clock}.x, {clock}.w, l(-1.000000), {clock}.x"),
        ]);
    }
    v.extend([
        format!("mul {clock}.x, {clock}.x, l(6.283185)"),
        format!("add {clock}.xyz, {clock}.xxxx, l(0.000000, 2.094395, 4.188790, 0.000000)"),
        format!("sincos null, {clock}.xyz, {clock}.xyzx"),
        format!("mad {clock}.xyz, {clock}.xyzx, l(0.100000, 0.100000, 0.100000, 0.000000), l(0.300000, 0.300000, 0.300000, 0.000000)"),
        format!("max {clock}.w, {reg}.w, l(0.000000)"),
        format!("add {reg}.xyz, {reg}.xyzx, l(-1.000000, -1.000000, -1.000000, 0.000000)"),
        format!("mad {reg}.xyz, {reg}.xyzx, {clock}.wwww, l(1.000000, 1.000000, 1.000000, 0.000000)"),
        format!("ld_indexable(texture1d)(float,float,float,float) {clock}.w, l(527, 0, 0, 0), t120.xxxx"),
        format!("mov_sat {clock}.w, {clock}.w"),
        format!("add {clock}.xyz, {clock}.xyzx, -{reg}.xyzx"),
        format!("mad {reg}.xyz, {clock}.wwww, {clock}.xyzx, {reg}.xyzx"),
    ]);
    v
}

/// How fast the witch-space rainbow turns, in turns a second (a jump lasts 10 to 15 s).
const RAINBOW_HZ: f64 = 0.3;

/// A copy of one of Elite's shaders that takes its directional lights from a raw buffer at `slot`
/// (the cockpit glass: 48-byte records, colour at byte 0, direction at byte 16) with every
/// record's colour multiplied by the star's tint. A record is read as two loads in a row, the
/// colour's first: each `ld_raw` from the slot that another load from it follows within two
/// lines is a colour load, and the tinting multiply goes right after it (inside its loop, where
/// the register is live). The tint itself is worked out once after the declarations.
pub fn star_light_list(asm: &str, slot: &str) -> Result<String, String> {
    let mut lines = unpatched_lines(asm)?;
    let is_load = |l: &str| l.trim_start().starts_with("ld_raw_indexable(raw_buffer)") && l.trim_end().ends_with(&format!(", {slot}.xyzx"));
    let loads: Vec<usize> = lines.iter().enumerate().filter(|(_, l)| is_load(l)).map(|(i, _)| i).collect();
    let colours: Vec<(usize, String)> = loads
        .iter()
        .filter(|&&i| loads.iter().any(|&j| j > i && j <= i + 2))
        .filter_map(|&i| {
            let rest = lines[i].trim_start().strip_prefix("ld_raw_indexable(raw_buffer)(mixed,mixed,mixed,mixed) ")?;
            let dest = rest.split(',').next()?.strip_suffix(".xyz")?;
            is_register(dest).then(|| (i, dest.to_string()))
        })
        .collect();
    let (Some((temps_line, n)), false) = (dcl_temps(&lines), colours.is_empty()) else {
        return Err(format!("this doesn't look like the light-list shader EDCAS knows (no paired loads from {slot}, or no dcl_temps); the game may have changed it"));
    };
    let places = TintPlaces::of(&lines)?;
    let (reg, clock) = (format!("r{n}"), format!("r{}", n + 1));
    // From the last line backwards, so the earlier indexes stay right (the loads all come after
    // the declarations, so the places stay right too).
    for (i, dest) in colours.iter().rev() {
        let indent: String = lines[*i].chars().take_while(|c| *c == ' ').collect();
        lines.insert(i + 1, format!("{indent}mul {dest}.xyz, {dest}.xyzx, {reg}.xyzx"));
    }
    Ok(places.insert(lines, temps_line, n, tint_block(&reg, &clock, None)))
}

/// The lines of a game shader's assembly (`\r\n` read as `\n`) that EDCAS has not patched yet:
/// one that reads IniParams (`t120`) already is refused.
fn unpatched_lines(asm: &str) -> Result<Vec<String>, String> {
    if asm.contains("t120") {
        return Err("this shader already reads t120 (IniParams); is it patched already?".into());
    }
    Ok(asm_lines(asm))
}

/// The assembly as lines, `\r\n` read as `\n`.
fn asm_lines(asm: &str) -> Vec<String> {
    asm.replace("\r\n", "\n").split('\n').map(str::to_string).collect()
}

/// `dcl_temps N`: its line and N.
fn dcl_temps(lines: &[String]) -> Option<(usize, u32)> {
    lines.iter().enumerate().find_map(|(i, l)| {
        let n = l.strip_prefix("dcl_temps ")?;
        n.bytes().all(|b| b.is_ascii_digit()).then(|| n.parse().ok().map(|n| (i, n)))?
    })
}

/// `mov o1.xyzw, rC.xyzw`, the move of the ground colour into its G-buffer target: `rC`.
fn colour_move(line: &str) -> Option<&str> {
    line.strip_prefix("mov o1.xyzw, ")?.strip_suffix(".xyzw").filter(|r| is_register(r))
}

/// The declaration of IniParams, the texture 3DMigoto binds at `t120`.
const T120_DECL: &str = "dcl_resource_texture1d (float,float,float,float) t120";

/// `dp3 rX.y, rN.xyzx, cb1[277].xyzx`: the normal's encoding: its line and N, the normal before it
/// is encoded.
fn encode_of(lines: &[String]) -> Option<(usize, String)> {
    lines.iter().enumerate().find_map(|(i, l)| {
        let rest = l.strip_prefix("dp3 ")?;
        let parts: Vec<&str> = rest.split(", ").collect();
        let [dest, normal, light] = parts.as_slice() else { return None };
        (dest.strip_suffix(".y").is_some_and(is_register) && *light == "cb1[277].xyzx").then_some(())?;
        normal.strip_suffix(".xyzx").filter(|r| is_register(r)).map(|r| (i, r.to_string()))
    })
}

/// Whether the line writes the rgb of the register `src`.
fn changes_colour(l: &str, src: &str) -> bool {
    l.split_once(' ').is_some_and(|(op, rest)| {
        !op.is_empty() && op.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') && rest.strip_prefix(src).and_then(|r| r.strip_prefix('.')).is_some_and(|r| r.starts_with(['x', 'y', 'z']))
    })
}

/// The ground shader's `tint` with the ground detail on top: layers of colour and relief from
/// the textures of `ground_detail`, picked by the ground type (`x514`). `asm` is a tinted ground
/// shader ([`tint`]'s result); the block from [`GROUND_DETAIL`] goes in just before the normal
/// is encoded (`dp3 rN.y, rM.xyzx, cb1[277].xyzx`). Port of `ground_tint.detail`, whose
/// docstring says how it works.
pub fn detail(asm: &str) -> Result<String, String> {
    const MARKER: &str = "// EDCAS planet ground tint";
    const NOT_TINTED: &str = "detail goes on a tinted ground shader (run tint first)";
    let mut lines = asm_lines(asm);

    let colour = lines.iter().enumerate().find_map(|(i, l)| colour_move(l).map(|r| (i, r.to_string())));
    let marker = lines.iter().position(|l| l.starts_with(MARKER));
    let (Some((temps_line, n)), Some((colour_line, src)), Some(_), Some((encode_line, normal))) = (dcl_temps(&lines), colour, marker, encode_of(&lines)) else {
        return Err(NOT_TINTED.into());
    };
    // From the encoding to the move into o1 nothing may change the colour's rgb.
    if lines[encode_line..colour_line.max(encode_line)].iter().any(|l| changes_colour(l, &src)) {
        return Err(format!("the ground colour {src} changes after the normal is encoded; the game may have changed the shader"));
    }

    let mut filled = GROUND_DETAIL.to_string();
    for (i, name) in DETAIL_NAMES.chars().enumerate() {
        filled = filled.replace(&format!("{{{name}}}"), &format!("r{}", n + i as u32));
    }
    let filled = filled.replace("{src}", &src).replace("{normal}", &normal);
    let (declarations, block): (Vec<&str>, Vec<&str>) = filled.lines().partition(|l| l.starts_with("dcl_"));
    check_operands(&block)?;

    let Some(t120_line) = lines.iter().position(|l| l == T120_DECL) else { return Err(NOT_TINTED.into()) };
    // The later lines first, so the earlier ones' indexes stay right.
    lines.splice(encode_line..encode_line, block.iter().map(|l| l.to_string()));
    lines[temps_line] = format!("dcl_temps {}", n + DETAIL_TEMPS);
    lines.splice(t120_line..t120_line, declarations.iter().map(|l| l.to_string()));
    Ok(lines.join("\n"))
}

/// Ice's index of refraction 1.31 gives F0 = ((n - 1) / (n + 1))^2 = 0.018; the game's own F0
/// (0.04 on rock) is blended towards it.
const ICE_F0: f32 = 0.018;
/// The roughness of polished ice when `y520` is 0 (Elite's perceptual one, `README-ELITE.md`,
/// "Deferred lighting").
const ICE_ROUGHNESS: f32 = 0.5;
/// The sparkle (`ground_tint.py`, `SPARKLE_*`, whose comment says how it works).
const SPARKLE_CELL: f64 = 0.02;
const SPARKLE_TILT: f64 = 0.7;
const SPARKLE_CONE: f64 = 0.09;
const SPARKLE_ROUGHNESS: f64 = 0.25;
const SPARKLE_F0: f64 = 0.02;
const SPARKLE_LEVEL_MAX: f64 = 20.0;
const SPARKLE_DOT_FOOTPRINTS: f64 = 3.0;
const SPARKLE_DOT_MIN: f64 = 0.3;
const HASH_LEVEL: u32 = 0x1656_67b1;
const HASH_MULTIPLIERS: [u32; 4] = [0x9e37_79b1, 0x85eb_ca6b, 0xc2b2_ae35, 0x27d4_eb2f];
const HASH_SALTS: [u32; 3] = [0x0000_0000, 0x68e3_1da4, 0xb529_7a4d];
const HASH_ROUNDS: [u32; 2] = [0x7feb_352d, 0x846c_a68b];

/// The sparkle's assembly (port of `ground_tint.sparkle_lines`): `a.x` is the sparkle's weight, `b.x` the
/// roughness so far, `normal` the world normal; `d` to `i` are scratch registers, `d.y` leaves the
/// sum of the glint weights (the y511 = 7 view).
fn sparkle_lines(a: &str, b: &str, d: &str, e: &str, f: &str, g: &str, h: &str, i: &str, normal: &str) -> Vec<String> {
    let k = HASH_MULTIPLIERS.iter().map(|v| format!("0x{v:08x}")).collect::<Vec<_>>().join(", ");
    let salts = HASH_SALTS.iter().map(|v| format!("0x{v:08x}")).chain(["0x00000000".to_string()]).collect::<Vec<_>>().join(", ");
    let mix: Vec<String> = HASH_ROUNDS.iter().map(|v| format!("0x{v:08x}, 0x{v:08x}, 0x{v:08x}, 0")).collect();
    let scale = format!("{:.9E}", 2.0 / 4_294_967_296.0);
    let tilt = format!("{SPARKLE_TILT:.6}");

    // One level's cells: d.x its level, d.z its cells per metre; its glints go into the normal and
    // the roughness by their weight (1 - h.y for the finer level, h.y for the coarser).
    let cell_pass = |second: bool| -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        if second {
            v.push(format!("  add {d}.x, {d}.x, l(1.000000)"));
        }
        v.extend([
            format!("  exp {d}.z, -{d}.x"),
            format!("  mul {d}.z, {d}.z, l({:.6})", 1.0 / SPARKLE_CELL),
            format!("  mul {e}.xy, v1.xyxx, cb2[0].xxxx"),
            format!("  mul {e}.xy, {e}.xyxx, {d}.zzzz"),
            format!("  round_ni {f}.xy, {e}.xyxx"),
            format!("  add {e}.xy, {e}.xyxx, -{f}.xyxx"),
            format!("  add {e}.xy, {e}.xyxx, l(-0.500000, -0.500000, 0.000000, 0.000000)"),
            format!("  dp2 {i}.w, {e}.xyxx, {e}.xyxx"),
            format!("  mul {g}.w, {h}.x, {d}.z"),
            format!("  mul {g}.w, {g}.w, l({SPARKLE_DOT_FOOTPRINTS:.6})"),
            format!("  max {g}.w, {g}.w, l({SPARKLE_DOT_MIN:.6})"),
            format!("  mul {g}.w, {g}.w, {g}.w"),
            // The dot's weight: a dome (1 - d² / r²)², 1 at the cell's centre, 0 at the disc's edge (a hard
            // inside-or-not made 2 to 3 pixel squares with the anti-aliasing off, 2026-11-09).
            format!("  div {i}.w, {i}.w, {g}.w"),
            format!("  add {i}.w, -{i}.w, l(1.000000)"),
            format!("  max {i}.w, {i}.w, l(0.000000)"),
            format!("  mul {i}.w, {i}.w, {i}.w"),
            format!("  ftoi {e}.xy, {f}.xyxx"),
            format!("  ftoi {e}.zw, cb2[1].xxxy"),
            format!("  imul null, {e}.xyzw, {e}.xyzw, l({k})"),
            format!("  xor {e}.x, {e}.x, {e}.y"),
            format!("  xor {e}.x, {e}.x, {e}.z"),
            format!("  xor {e}.x, {e}.x, {e}.w"),
            format!("  ftoi {g}.w, {d}.x"),
            format!("  imul null, {g}.w, {g}.w, l(0x{HASH_LEVEL:08x})"),
            format!("  xor {e}.x, {e}.x, {g}.w"),
            format!("  iadd {f}.xyz, {e}.xxxx, l({salts})"),
            format!("  ushr {g}.xyz, {f}.xyzx, l(16, 16, 16, 0)"),
            format!("  xor {f}.xyz, {f}.xyzx, {g}.xyzx"),
            format!("  imul null, {f}.xyz, {f}.xyzx, l({})", mix[0]),
            format!("  ushr {g}.xyz, {f}.xyzx, l(15, 15, 15, 0)"),
            format!("  xor {f}.xyz, {f}.xyzx, {g}.xyzx"),
            format!("  imul null, {f}.xyz, {f}.xyzx, l({})", mix[1]),
            format!("  ushr {g}.xyz, {f}.xyzx, l(16, 16, 16, 0)"),
            format!("  xor {f}.xyz, {f}.xyzx, {g}.xyzx"),
            format!("  utof {f}.xyz, {f}.xyzx"),
            format!("  mad {f}.xyz, {f}.xyzx, l({scale}, {scale}, {scale}, 0.000000), l(-1.000000, -1.000000, -1.000000, 0.000000)"),
            format!("  dp3 {e}.x, {normal}.xyzx, {normal}.xyzx"),
            format!("  rsq {e}.x, {e}.x"),
            format!("  mul {g}.xyz, {normal}.xyzx, {e}.xxxx"),
            format!("  mad {f}.xyz, {f}.xyzx, l({tilt}, {tilt}, {tilt}, 0.000000), {g}.xyzx"),
            format!("  dp3 {e}.x, {f}.xyzx, {f}.xyzx"),
            format!("  rsq {e}.x, {e}.x"),
            format!("  mul {f}.xyz, {f}.xyzx, {e}.xxxx"),
            format!("  dp3 {e}.x, v3.xyzx, v3.xyzx"),
            format!("  rsq {e}.x, {e}.x"),
            format!("  mul {i}.xyz, v3.xyzx, -{e}.xxxx"),
            format!("  add {i}.xyz, {i}.xyzx, -cb1[4].xyzx"),
            format!("  dp3 {e}.x, {i}.xyzx, {i}.xyzx"),
            format!("  rsq {e}.x, {e}.x"),
            format!("  mul {i}.xyz, {i}.xyzx, {e}.xxxx"),
            format!("  dp3 {e}.x, {f}.xyzx, {i}.xyzx"),
            format!("  mul {e}.y, {h}.z, {h}.z"),
            format!("  mad {e}.y, {e}.y, l({:.9}), l(1.000000)", -SPARKLE_CONE * SPARKLE_CONE / 2.0),
            format!("  ge {e}.x, {e}.x, {e}.y"),
            format!("  and {e}.x, {e}.x, {d}.w"),
            format!("  mul {e}.x, {e}.x, {i}.w"),
            if second { format!("  mov {e}.y, {h}.y") } else { format!("  add {e}.y, -{h}.y, l(1.000000)") },
            format!("  mul {e}.x, {e}.x, {e}.y"),
            format!("  add {d}.y, {d}.y, {e}.x"),
            format!("  add {f}.xyz, {i}.xyzx, -{g}.xyzx"),
            format!("  mad {f}.xyz, {f}.xyzx, {e}.xxxx, {g}.xyzx"),
            format!("  dp3 {e}.y, {f}.xyzx, {f}.xyzx"),
            format!("  rsq {e}.y, {e}.y"),
            format!("  mul {normal}.xyz, {f}.xyzx, {e}.yyyy"),
            format!("  add {e}.y, -{b}.x, l({SPARKLE_ROUGHNESS:.6})"),
            format!("  mad {b}.x, {e}.x, {e}.y, {b}.x"),
        ]);
        v
    };

    let mut lines = vec![
        "// EDCAS sparkle: z520 (0 = 1, under 0 = off) widens or narrows the cone of facets that glint".to_string(),
        format!("ld_indexable(texture1d)(float,float,float,float) {h}.xyzw, l(520, 0, 0, 0), t120.xyzw"),
        format!("eq {h}.w, {h}.z, l(0.000000)"),
        format!("movc {h}.z, {h}.w, l(1.000000), {h}.z"),
        format!("lt {h}.w, {h}.z, l(0.000000)"),
        format!("deriv_rtx_fine {d}.xyz, v3.xyzx"),
        format!("deriv_rty_fine {e}.xyz, v3.xyzx"),
        format!("dp3 {d}.w, {d}.xyzx, {d}.xyzx"),
        format!("dp3 {e}.w, {e}.xyzx, {e}.xyzx"),
        format!("max {d}.w, {d}.w, {e}.w"),
        format!("sqrt {h}.x, {d}.w"),
        format!("movc {d}.w, {h}.w, l(0.000000), {a}.x"),
        format!("mov {d}.y, l(0.000000)"),
        format!("lt {e}.x, l(0.000000), {d}.w"),
        format!("if_nz {e}.x"),
        format!("  mul {e}.x, {h}.x, l({:.6})", 1.0 / SPARKLE_CELL),
        format!("  max {e}.x, {e}.x, l(1.000000)"),
        format!("  log {e}.x, {e}.x"),
        format!("  min {e}.x, {e}.x, l({SPARKLE_LEVEL_MAX:.6})"),
        format!("  round_ni {d}.x, {e}.x"),
        format!("  add {h}.y, {e}.x, -{d}.x"),
    ];
    lines.extend(cell_pass(false));
    lines.extend(cell_pass(true));
    lines.push("endif".to_string());
    lines
}

/// The ground shader with the ice gloss: on the ground types of ice and frost (2 and 3, `x514`)
/// the G-buffer's roughness `o0.x` and Fresnel F0 `o2.x` are blended from the game's towards a
/// low roughness and ice's F0, so Elite's own deferred lighting draws the sun's and the probes'
/// specular light. IniParams row 20: `x` strength (0 = 1, under 0 = off), `y` the roughness of
/// polished ice (0 = [`ICE_ROUGHNESS`]). `asm` is [`detail`]'s result; the block goes just before
/// the normal is encoded. Port of `ground_tint.gloss`, whose docstring says more.
pub fn gloss(asm: &str) -> Result<String, String> {
    const UNKNOWN: &str = "gloss goes on a ground shader with the tint and the detail (it needs the roughness 'max o0.x, rN.w, l(0.001)' and the normal's encoding); the game may have changed it";
    let mut lines = asm_lines(asm);

    // `max o0.x, rM.w, l(0.001000)`: M holds the game's roughness.
    let rough = lines.iter().find_map(|l| {
        let r = l.strip_prefix("max o0.x, ")?.strip_suffix(".w, l(0.001000)")?;
        is_register(r).then(|| r.to_string())
    });
    // `mov o2.xyzw, rK.xyzw` or `.xywz`: K.xy is the F0 (50fd337fd22daec6, 695ededf10e98696); the
    // other shader writes o2.xy itself.
    let copy = lines.iter().find_map(|l| {
        let r = l.strip_prefix("mov o2.xyzw, ")?;
        let r = r.strip_suffix(".xyzw").or_else(|| r.strip_suffix(".xywz"))?;
        is_register(r).then(|| r.to_string())
    });
    // The normal's encoding (its register is the world normal, which the sparkle changes).
    let (Some((temps_line, n)), Some(rough), Some((encode_line, normal)), true) = (dcl_temps(&lines), rough, encode_of(&lines), asm.contains("t120")) else {
        return Err(UNKNOWN.into());
    };
    // The ground colour (after the tint), which the debug views paint.
    let Some(colour) = lines.iter().find_map(|l| colour_move(l).map(str::to_string)) else {
        return Err("gloss: the move of the ground colour into o1 is missing".into());
    };
    let r = |m: u32| format!("r{}", n + m);
    let (a, b, c, d, e, f, g, h, i) = (r(0), r(1), r(2), r(3), r(4), r(5), r(6), r(7), r(8));
    // The snow crust (the detail's mask, the register its comment line names) is matte: the broad gloss (the sheen lobe)
    // only where the ice is polished, its weight in b.z; the sparkle's glints (a.x: strength and ice types) are not times 1 - mask.
    let crust = lines.iter().find_map(|l| l.strip_prefix("// EDCAS crust mask: ")?.strip_suffix(".x").filter(|r| is_register(r)).map(str::to_string));
    let crust_lines: Vec<String> = crust.map(|r| vec![format!("add {b}.y, -{r}.x, l(1.000000)"), format!("mul {b}.z, {a}.x, {b}.y")]).unwrap_or_else(|| vec![format!("mov {b}.z, {a}.x")]);
    let (game, dest) = match copy {
        Some(r) => (r.clone(), format!("{r}.xy")),
        None => {
            // The shader writes o2.xy itself, in the branches of its own gloss path (a `mad` and a
            // `mov`): the game's F0 goes to c instead, and the block blends from it into o2.xy.
            let mut count = 0;
            for l in lines.iter_mut() {
                let body = l.trim_start();
                let indent = l.len() - body.len();
                if let Some(op) = ["mad", "mov"].into_iter().find(|op| body.strip_prefix(*op).is_some_and(|r| r.starts_with(" o2.xy, "))) {
                    *l = format!("{}{op} {c}.xy, {}", &l[..indent], &body[op.len() + " o2.xy, ".len()..]);
                    count += 1;
                }
            }
            if count == 0 {
                return Err("gloss: no write of o2.xy to redirect; the game may have changed the shader".into());
            }
            (c.clone(), "o2.xy".to_string())
        }
    };
    let ice = format!("{ICE_F0:.6}");
    let block = [
        "// EDCAS ground gloss: x520 strength (0 = 1, under 0 = off), y520 roughness of polished ice (0 = 0.5); ground types 2, 3 only".to_string(),
        format!("ld_indexable(texture1d)(float,float,float,float) {a}.xyzw, l(520, 0, 0, 0), t120.xyzw"),
        format!("ld_indexable(texture1d)(float,float,float,float) {b}.xyzw, l(514, 0, 0, 0), t120.xyzw"),
        format!("ge {a}.z, {b}.x, l(1.500000)"),
        format!("lt {a}.w, {b}.x, l(3.500000)"),
        format!("and {a}.z, {a}.z, {a}.w"),
        format!("eq {b}.y, {a}.x, l(0.000000)"),
        format!("movc {a}.x, {b}.y, l(1.000000), {a}.x"),
        format!("max {a}.x, {a}.x, l(0.000000)"),
        format!("movc {a}.x, {a}.z, {a}.x, l(0.000000)"),
        "@crust".to_string(),
        format!("eq {b}.y, {a}.y, l(0.000000)"),
        format!("movc {a}.y, {b}.y, l({ICE_ROUGHNESS:.6}), {a}.y"),
        format!("add {b}.x, {a}.y, -{rough}.w"),
        format!("mad {b}.x, {b}.z, {b}.x, {rough}.w"),
        "@sparkle".to_string(),
        format!("mov {a}.x, {b}.z"),
        format!("max o0.x, {b}.x, l(0.001000)"),
        // The glints' F0 (SPARKLE_F0) in place of the game's where a glint is (d.y, the sum of their weights).
        format!("mad_sat {e}.x, {d}.y, l(1.000000), l(0.000000)"),
        format!("add {e}.y, l({SPARKLE_F0:.6}), -{game}.x"),
        format!("mad {game}.xy, {e}.xxxx, {e}.yyyy, {game}.xyxx"),
        // F0: the game's own (rock's term included) towards ice's, by the strength; 0 = the game's.
        format!("add {b}.yz, l(0.000000, {ice}, {ice}, 0.000000), -{game}.xxyx"),
        format!("mad {dest}, {a}.xxxx, {b}.yzyy, {game}.xyxx"),
        // Debug views (y511 = 7: white where a glint's weight is above 0, y511 = 8: the sparkle's weight as
        // grey), painted into the ground colour.
        format!("ld_indexable(texture1d)(float,float,float,float) {e}.xyzw, l(511, 0, 0, 0), t120.xyzw"),
        format!("eq {f}.x, {e}.y, l(7.000000)"),
        format!("lt {f}.y, l(0.000000), {d}.y"),
        format!("and {f}.x, {f}.x, {f}.y"),
        format!("movc {colour}.xyz, {f}.xxxx, l(1.000000, 1.000000, 1.000000, 0.000000), {colour}.xyzx"),
        format!("eq {f}.x, {e}.y, l(8.000000)"),
        format!("movc {colour}.xyz, {f}.xxxx, {d}.wwww, {colour}.xyzx"),
    ];
    let at = block.iter().position(|l| l == "@crust").expect("the crust marker");
    let mut block = block.to_vec();
    block.splice(at..=at, crust_lines);
    let at = block.iter().position(|l| l == "@sparkle").expect("the sparkle marker");
    block.splice(at..=at, sparkle_lines(&a, &b, &d, &e, &f, &g, &h, &i, &normal));
    let refs: Vec<&str> = block.iter().map(String::as_str).collect();
    check_operands(&refs)?;
    lines.splice(encode_line..encode_line, block);
    lines[temps_line] = format!("dcl_temps {}", n + 9);
    Ok(lines.join("\n"))
}

/// The ground shader with the terrain mask: it also writes its distance from the camera
/// (metres, the length of the camera-relative position `v3`) to `o4`, which 3DMigoto binds next
/// to Elite's four G-buffer outputs. The dust takes a pixel for ground only where that distance
/// is the depth buffer's, which also holds the ships and the SRV. `asm` is [`detail`]'s result.
/// Port of `ground_tint.mask`, whose docstring says more.
pub fn mask(asm: &str) -> Result<String, String> {
    const UNKNOWN: &str = "the terrain mask goes on a tinted ground shader with outputs o0 to o3 and the position v3; the game may have changed it";
    let mut lines = asm_lines(asm);

    let outputs: Vec<usize> = lines.iter().enumerate().filter(|(_, l)| l.starts_with("dcl_output ")).map(|(i, _)| i).collect();
    let names: Vec<&str> = outputs.iter().filter_map(|&i| lines[i]["dcl_output ".len()..].split('.').next()).collect();
    let position = lines.iter().any(|l| l.starts_with("dcl_input_ps linear v3.xyz"));
    // The move of the ground colour into o1 (after the tint's marker: the tint is in).
    let alpha = lines.iter().position(|l| l.starts_with("// EDCAS planet ground tint")).and_then(|m| {
        lines[m..].iter().position(|l| colour_move(l).is_some()).map(|i| m + i)
    });
    let (Some((temps_line, n)), Some(alpha_line), true, ["o0", "o1", "o2", "o3"]) = (dcl_temps(&lines), alpha, position, names.as_slice()) else {
        return Err(UNKNOWN.into());
    };
    let r = format!("r{n}");

    // The later lines first, so the earlier ones' indexes stay right.
    let write = [
        "// EDCAS terrain mask: this ground's distance from the camera (m), for the dust".to_string(),
        format!("dp3 {r}.x, v3.xyzx, v3.xyzx"),
        format!("sqrt o4.x, {r}.x"),
    ];
    lines.splice(alpha_line + 1..alpha_line + 1, write);
    lines[temps_line] = format!("dcl_temps {}", n + 1);
    lines.insert(outputs[outputs.len() - 1] + 1, "dcl_output o4.x".to_string());
    Ok(lines.join("\n"))
}

/// The ground shader as EDCAS installs it: tint, detail, gloss, terrain mask. Port of `ground_tint.patch`.
pub fn patch(asm: &str) -> Result<String, String> {
    tint(asm).and_then(|t| detail(&t)).and_then(|d| gloss(&d)).and_then(|g| mask(&g))
}

/// Whether `s` is a temporary register, `r` and digits.
fn is_register(s: &str) -> bool {
    s.strip_prefix('r').is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Every operand of the generated assembly is a register or a literal: a name that was not
/// filled in (or a value formatted in its place) fails here, not in the game, where 3DMigoto
/// falls back to Elite's shader without a word on screen (2026-10-29). As `check_operands` in
/// `ground_tint.py`.
fn check_operands(lines: &[&str]) -> Result<(), String> {
    let operand_ok = |o: &str| {
        let o = o.strip_prefix('-').unwrap_or(o);
        let o = o.strip_prefix('|').unwrap_or(o);
        let o = o.strip_prefix('-').unwrap_or(o);
        let o = o.strip_suffix('|').unwrap_or(o);
        let (reg, swizzle) = match o.split_once('.') {
            Some((r, s)) => (r, Some(s)),
            None => (o, None),
        };
        let digits = |d: &str| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit());
        let register = reg == "null" || ['r', 'v', 'o', 't', 's'].iter().any(|p| reg.strip_prefix(*p).is_some_and(digits))
            || reg.strip_prefix("cb").and_then(|r| r.strip_suffix(']')).and_then(|r| r.split_once('[')).is_some_and(|(b, i)| digits(b) && digits(i));
        register && swizzle.is_none_or(|s| (1..=4).contains(&s.len()) && s.bytes().all(|b| matches!(b, b'x' | b'y' | b'z' | b'w')))
    };
    for line in lines.iter().map(|l| l.trim()).filter(|l| !l.starts_with("//") && l.contains(' ')) {
        let (_, operands) = line.split_once(' ').unwrap_or((line, ""));
        // Literals `l(...)` are one operand each.
        let mut flat = String::new();
        let mut rest = operands;
        while let Some(at) = rest.find("l(") {
            let end = rest[at..].find(')').map_or(rest.len(), |e| at + e + 1);
            flat.push_str(&rest[..at]);
            flat.push('l');
            rest = &rest[end..];
        }
        flat.push_str(rest);
        for operand in flat.split(',').map(str::trim) {
            if operand != "l" && !operand_ok(operand) {
                return Err(format!("bad operand {operand:?} in: {line}"));
            }
        }
    }
    Ok(())
}

/// Where 3DMigoto writes the shaders it exports.
fn exported_path(dir: &Path, hash: &str) -> PathBuf {
    dir.join("ShaderCache").join(format!("{hash}-ps.txt"))
}

/// How the star light's copies of the game's shaders stand: how many of [`Self::TOTAL`] are in
/// `ShaderFixes`, and what went wrong with the exports that are there.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct StarLightShaders {
    pub made: usize,
    pub failed: Vec<String>,
}

impl StarLightShaders {
    pub const TOTAL: usize = SUN_SHADERS.len() + ATMOSPHERE_SHADERS.len() + FORWARD_SHADERS.len() + LIGHT_LIST_SHADERS.len();

    pub fn done(&self) -> bool {
        self.made == Self::TOTAL
    }
}

/// What [`finish_game_shaders`] (and so [`install`] and [`update_shaders`]) made: the ground tint
/// and the star light's copies.
#[derive(Clone, Debug, PartialEq)]
pub struct Patched {
    pub ground_tint: GroundTint,
    pub star_light: StarLightShaders,
}

/// Which copies of the game's own shaders the settings ask for. 3DMigoto can only get the game's
/// shaders by exporting every shader the game makes (`export_shaders`), at each game start anew
/// and whenever the game makes one while it runs: a disassembly each, so a hitch each. The
/// export is therefore on only while a copy that is asked for is still to be made (2026-11-10:
/// it stayed on until all of the star light's 28 were there, whatever was switched on, and
/// testers with every effect off had drops).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wanted {
    /// The tinted ground shaders: the ground tint, the ground detail, and the dust's terrain mask.
    pub ground: bool,
    /// The star light's copies of the sun, sky, ring, forward and glass shaders.
    pub star_light: bool,
}

impl Wanted {
    #[cfg(test)]
    pub const ALL: Wanted = Wanted { ground: true, star_light: true };

    pub fn of(g: &crate::settings::game_graphics::GameGraphicsSettings) -> Self {
        Wanted { ground: g.ground_tint || g.ground_detail || g.planet_dust, star_light: g.star_light }
    }
}

/// How the ground tint stands after [`install`] or [`finish_game_shaders`].
#[derive(Clone, Debug, PartialEq)]
pub enum GroundTint {
    /// The tinted ground shaders are in `ShaderFixes`; the game picks them up when it starts.
    Done,
    /// The game's ground shaders aren't exported yet: `export_shaders` is on, and landing on
    /// a planet once writes them.
    NeedsLanding,
    /// The exported shaders aren't the ones EDCAS knows.
    Failed(String),
}

/// Makes EDCAS's copies of the game's shaders from the ones 3DMigoto exported: the tinted ground
/// shaders when their three exports are all there, and each of the star light's
/// [`StarLightShaders::TOTAL`] as its export turns up (the game exports a shader the first time it
/// draws with it: the sun pass of open space at the first run, the atmosphere's over a planet
/// with air, the rings' inside a ring). Sets `export_shaders` in `d3dx.ini` to match: on while a copy `want` asks
/// for is still to be made, off otherwise. Copies already in `ShaderFixes` stay when the exports are gone.
pub fn finish_game_shaders(dir: &Path, want: Wanted) -> Result<Patched, String> {
    let ini_path = dir.join("d3dx.ini");
    if !ini_path.is_file() {
        return Err(format!("{} is missing; install the shader mods first", ini_path.display()));
    }
    let exports: Vec<PathBuf> = GROUND_SHADERS.iter().map(|h| exported_path(dir, h)).collect();
    let ground_tint = if exports.iter().all(|p| p.is_file()) {
        make_tinted(dir, &exports)
    } else if ground_tint_done(dir) {
        GroundTint::Done
    } else {
        GroundTint::NeedsLanding
    };
    let star_light = make_star_light(dir);
    let export = (want.ground && ground_tint == GroundTint::NeedsLanding) || (want.star_light && !star_light.done());
    set_export(dir, export)?;
    Ok(Patched { ground_tint, star_light })
}

/// Whether the game has to export its shaders for `want`: a copy asked for is not in `ShaderFixes`.
fn export_needed(dir: &Path, want: Wanted) -> bool {
    (want.ground && !ground_tint_done(dir)) || (want.star_light && star_light_made(dir) < StarLightShaders::TOTAL)
}

/// Writes `export_shaders` in `d3dx.ini`, only when it says something else. Returns whether it changed.
fn set_export(dir: &Path, on: bool) -> Result<bool, String> {
    let ini_path = dir.join("d3dx.ini");
    let value = if on { "1" } else { "0" };
    if read_ini(&ini_path).is_some_and(|ini| get_option(&ini, "Rendering", "export_shaders").as_deref() == Some(value)) {
        return Ok(false);
    }
    edit_ini(&ini_path, |ini| set_option(&ini, "Rendering", "export_shaders", value))?;
    Ok(true)
}

/// The settings whose export [`sync_export`] last looked at, with the game folder.
static EXPORT_SYNCED: std::sync::Mutex<Option<(PathBuf, Wanted)>> = std::sync::Mutex::new(None);

/// Brings `export_shaders` in line with the settings after they changed: off as soon as nothing
/// switched on needs a game shader any more, on again when something does. Called at every change
/// of the settings; does nothing unless what is wanted changed. On a thread, as [`maintain`];
/// `d3dx.ini` is read at the game's start, so a change counts from the next one.
pub fn sync_export(settings: &crate::settings::Settings) {
    let Some(dir) = crate::ground_tint::game_dir(settings) else { return };
    let want = Wanted::of(&settings.game_graphics);
    {
        let mut synced = EXPORT_SYNCED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if synced.as_ref() == Some(&(dir.clone(), want)) {
            return;
        }
        *synced = Some((dir.clone(), want));
    }
    thread::spawn(move || {
        let _guard = JOB_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !matches!(status(&dir, VERSION), Status::Installed { .. }) {
            return;
        }
        let on = export_needed(&dir, want);
        match set_export(&dir, on) {
            Ok(true) => info!("Shader mods: the game's shader export is {} from its next start ({want:?})", if on { "on" } else { "off" }),
            Ok(false) => {}
            Err(e) => warn!("Shader mods: could not set the game's shader export: {e}"),
        }
    });
}

/// Whether an export is waiting to become one of the star light's copies: what [`maintain`]
/// finishes by itself when the game is closed.
fn star_light_pending(dir: &Path) -> bool {
    star_light_shaders().any(|(h, _)| !dir.join(FIXES).join(format!("{h}-ps.txt")).is_file() && exported_path(dir, h).is_file())
}

/// Writes the star light's copy of every exported game shader: anew whenever the export is there
/// (the patch changes between client versions; a copy kept as it was would run the old one,
/// 2026-11-09), kept as it is when the export is gone.
fn make_star_light(dir: &Path) -> StarLightShaders {
    let fixes = dir.join(FIXES);
    let mut state = StarLightShaders::default();
    for (hash, source) in star_light_shaders() {
        let out = fixes.join(format!("{hash}-ps.txt"));
        let export = exported_path(dir, hash);
        if !export.is_file() {
            if out.is_file() {
                state.made += 1;
            }
            continue;
        }
        let made = fs::read_to_string(&export).map_err(|e| io_error("read", &export, &e)).and_then(|asm| source.patch(&asm));
        match made.and_then(|text| fs::write(&out, text).map_err(|e| io_error("write", &out, &e))) {
            Ok(()) => state.made += 1,
            Err(e) => state.failed.push(format!("{hash}-ps.txt: {e}")),
        }
    }
    state
}

/// Writes the tinted copy of every export into `ShaderFixes`; nothing is written unless all work.
fn make_tinted(dir: &Path, exports: &[PathBuf]) -> GroundTint {
    let mut out = Vec::new();
    for (hash, path) in GROUND_SHADERS.iter().zip(exports) {
        let asm = match fs::read_to_string(path) {
            Ok(a) => a,
            Err(e) => return GroundTint::Failed(format!("can't read {}: {e}", path.display())),
        };
        match patch(&asm) {
            Ok(t) => out.push((hash, t)),
            Err(e) => return GroundTint::Failed(format!("{hash}-ps.txt: {e}")),
        }
    }
    for (hash, text) in out {
        let path = dir.join(FIXES).join(format!("{hash}-ps.txt"));
        if let Err(e) = fs::write(&path, text) {
            return GroundTint::Failed(io_error("write", &path, &e));
        }
    }
    GroundTint::Done
}

// ------------------------------------------------------------------ d3dx.ini

/// Sets `key = value` in `[section]`, replacing the key's line there (an active one before a
/// commented-out one) or adding it under the header; adds the section when it is missing.
/// Works on text with `\n` line ends.
pub fn set_option(ini: &str, section: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = ini.split('\n').map(str::to_string).collect();
    let Some(head) = lines.iter().position(|l| is_header(l, section)) else {
        return format!("{}\n\n[{section}]\n{key} = {value}\n", ini.trim_end_matches('\n'));
    };
    let end = lines[head + 1..].iter().position(|l| l.starts_with('[')).map_or(lines.len(), |i| head + 1 + i);
    let body = &lines[head + 1..end];
    let at = body.iter().position(|l| is_assignment(l, key, false)).or_else(|| body.iter().position(|l| is_assignment(l, key, true)));
    let new = format!("{key} = {value}");
    match at {
        Some(i) => lines[head + 1 + i] = new,
        None => lines.insert(head + 1, new),
    }
    lines.join("\n")
}

/// The value of `key` in `[section]` when an active (not commented-out) line sets it.
pub fn get_option(ini: &str, section: &str, key: &str) -> Option<String> {
    let mut lines = ini.split('\n');
    lines.find(|l| is_header(l, section))?;
    lines
        .take_while(|l| !l.starts_with('['))
        .find(|l| is_assignment(l, key, false))
        .and_then(|l| l.split_once('=').map(|(_, v)| v.trim().to_string()))
}

/// Comments out the lines that set `key` (`;key = …`).
pub fn comment_out(ini: &str, key: &str) -> String {
    ini.split('\n').map(|l| if is_assignment(l, key, false) { format!(";{l}") } else { l.to_string() }).collect::<Vec<_>>().join("\n")
}

/// Makes `d3dx.ini` include `ShaderFixes\edcas.ini`, once.
pub fn add_include(ini: &str) -> String {
    if ini.split('\n').any(is_include) {
        return ini.to_string();
    }
    let mut lines: Vec<String> = ini.split('\n').map(str::to_string).collect();
    match lines.iter().position(|l| is_header(l, "Include")) {
        Some(head) => {
            lines.insert(head + 1, format!("include = {INCLUDE}"));
            lines.join("\n")
        }
        None => format!("{}\n\n[Include]\ninclude = {INCLUDE}\n", ini.trim_end_matches('\n')),
    }
}

/// Takes the include of `ShaderFixes\edcas.ini` out again.
pub fn remove_include(ini: &str) -> String {
    ini.split('\n').filter(|l| !is_include(l)).collect::<Vec<_>>().join("\n")
}

fn is_header(line: &str, section: &str) -> bool {
    line.trim_end().eq_ignore_ascii_case(&format!("[{section}]"))
}

/// Whether `line` sets `key` (case-insensitive); with `commented`, also as `;key = …`.
fn is_assignment(line: &str, key: &str, commented: bool) -> bool {
    let line = if commented { line.strip_prefix(';').unwrap_or(line) } else { line };
    let line = line.trim_start_matches([' ', '\t']);
    line.len() >= key.len()
        && line.is_char_boundary(key.len())
        && line[..key.len()].eq_ignore_ascii_case(key)
        && line[key.len()..].trim_start_matches([' ', '\t']).starts_with('=')
}

fn is_include(line: &str) -> bool {
    let t = line.trim();
    t.len() > 7 && t.is_char_boundary(7) && t[..7].eq_ignore_ascii_case("include") && {
        let rest = t[7..].trim_start();
        rest.strip_prefix('=').is_some_and(|v| v.trim().eq_ignore_ascii_case(INCLUDE))
    }
}

/// Reads an ini as text with `\n` line ends (byte for byte, so non-UTF-8 comments survive),
/// lets `edit` change it and writes it back with the line ends it had.
fn edit_ini(path: &Path, edit: impl FnOnce(String) -> String) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| io_error("read", path, &e))?;
    let crlf = bytes.windows(2).any(|w| w == b"\r\n");
    let text: String = bytes.iter().map(|&b| b as char).collect();
    let edited = edit(text.replace("\r\n", "\n"));
    let edited = if crlf { edited.replace('\n', "\r\n") } else { edited };
    // Every char is one the file had, or ASCII we added.
    let out: Vec<u8> = edited.chars().map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?')).collect();
    if out != bytes {
        fs::write(path, out).map_err(|e| io_error("write", path, &e))?;
    }
    Ok(())
}

// ------------------------------------------------------------------ download

/// The verified download.
pub struct Bundle {
    zip: Vec<u8>,
}

/// Downloads the 3DMigoto zip and its checksum list from `base_url` (the edcas server) and
/// checks the one against the other.
pub fn download(base_url: &str) -> Result<Bundle, String> {
    let base = base_url.trim().trim_end_matches('/');
    let base = if base.is_empty() { DEFAULT_BASE_URL } else { base };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .user_agent(format!("edcas-client/{VERSION}"))
        .build()
        .map_err(|e| format!("can't start the download: {e}"))?;
    let get = |name: &str| -> Result<Vec<u8>, String> {
        let url = format!("{base}/download/{name}");
        let resp = client.get(&url).send().map_err(|e| format!("can't download {url}: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("can't download {url}: the server answered {}", resp.status()));
        }
        resp.bytes().map(|b| b.to_vec()).map_err(|e| format!("can't download {url}: {e}"))
    };
    let sums = String::from_utf8_lossy(&get(SUMS_NAME)?).into_owned();
    let zip = get(ZIP_NAME)?;
    verify_checksum(&zip, &sums, ZIP_NAME)?;
    Ok(Bundle { zip })
}

/// Checks `data` against the line for `name` in a `sha256sum` list (`<hex>  <name>`).
pub fn verify_checksum(data: &[u8], sums: &str, name: &str) -> Result<(), String> {
    let wanted = sums
        .lines()
        .filter_map(|l| l.split_once(char::is_whitespace))
        .find(|(_, file)| file.trim().trim_start_matches('*') == name)
        .map(|(hash, _)| hash.trim().to_ascii_lowercase())
        .ok_or_else(|| format!("{SUMS_NAME} on the server doesn't list {name}"))?;
    let got: String = Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect();
    if got != wanted {
        return Err(format!("the download of {name} is damaged: its SHA-256 is {got}, the server lists {wanted}"));
    }
    Ok(())
}

// ------------------------------------------------------------------ install

/// Installs or updates the shader mods in `dir` from the (verified) `bundle`; `progress`
/// hears each step. The game must not run on Windows, which locks `d3d11.dll`.
///
/// 1. `d3d11.dll` first, so a locked file stops everything before anything else changed. A
///    `d3d11.dll` that isn't EDCAS's is kept once as `d3d11.dll.official`.
/// 2. The zip's other files, without replacing any that exist (EDHM and others use
///    3DMigoto too, and `d3dx.ini` is theirs as much), except `d3dcompiler_47.dll` and
///    EDCAS's own `EDCAS-README.txt` and `source/`.
/// 3. EDCAS's shaders and `edcas.ini` into `ShaderFixes`, `edcas_params.txt` only if
///    missing, and the ground detail's textures ([`ground_detail`]), made here.
/// 4. `d3dx.ini` edited: `params_file`, `params_fade`, the include, `hunting = 0`, no
///    `analyse_frame` (F8, which crashes Elite on Proton).
/// 5. The copies of the game's shaders ([`finish_game_shaders`]).
/// 6. `edcas_version.txt`, last: an interrupted install shows as an update.
pub fn install(dir: &Path, bundle: &Bundle, version: &str, want: Wanted, mut progress: impl FnMut(&str)) -> Result<Patched, String> {
    // Every step goes to the log as well (2026-11-08): a failed install is diagnosed from
    // the log, and the step it stopped in is the first thing to read there.
    let mut progress = |step: &str| {
        info!("Shader mods install: {step}");
        progress(step);
    };
    check_game(dir)?;
    progress("Unpacking the download");
    let files = read_zip(&bundle.zip)?;
    let dll = files.iter().find(|(p, _)| p == "d3d11.dll").map(|(_, d)| d).ok_or("the download has no d3d11.dll")?;

    progress("Installing d3d11.dll");
    install_dll(dir, dll)?;

    progress("Installing 3DMigoto");
    for (name, data) in files.iter().filter(|(p, _)| p != "d3d11.dll") {
        let dest = dir.join(name);
        let ours = name == "d3dcompiler_47.dll" || name == "EDCAS-README.txt" || name.starts_with("source/");
        if dest.exists() && !ours {
            continue;
        }
        if name == "d3dcompiler_47.dll" {
            write_locked(&dest, data)?;
        } else {
            write_file(&dest, data)?;
        }
    }

    update_shaders(dir, version, want, &mut progress)
}

/// The part of an install that comes from this client alone, no download: EDCAS's shaders and
/// ini, the ground textures, the `d3dx.ini` options, the tinted ground shaders and the version
/// file. 3DMigoto must be there already (`d3dx.ini`). This is what an update is, so
/// [`maintain`] runs it by itself when the installed version is another one (2026-11-08: every
/// test build had needed "replace the exe, press Update", and a forgotten press ran old shaders
/// under a new client).
pub fn update_shaders(dir: &Path, version: &str, want: Wanted, progress: &mut impl FnMut(&str)) -> Result<Patched, String> {
    progress("Writing the EDCAS shaders");
    let fixes = dir.join(FIXES);
    fs::create_dir_all(&fixes).map_err(|e| io_error("create", &fixes, &e))?;
    remove_retired_shaders(&fixes)?;
    for (name, text) in SKY_SHADERS.iter().chain([&(INI_FILE, EDCAS_INI)]) {
        write_file(&fixes.join(name), text.as_bytes())?;
    }
    let params = fixes.join(PARAMS_FILE);
    if !params.exists() {
        write_file(&params, b"; written by EDCAS: planet ground tint\nx500 = 0\n")?;
    }
    write_ground_textures(&fixes, progress)?;

    progress("Setting up d3dx.ini");
    setup_ini(dir)?;

    progress("Setting up the ground tint and the star light");
    let patched = finish_game_shaders(dir, want)?;

    write_file(&fixes.join(VERSION_FILE), format!("{version}\n").as_bytes())?;
    info!("Shader mods {version} installed in {} (ground tint: {:?}, star light: {:?})", dir.display(), patched.ground_tint, patched.star_light);
    Ok(patched)
}

/// What EDCAS needs in `d3dx.ini`: the include of its ini, the params file and its fade, no
/// hunting. 3DMigoto must be there already (an install writes the release's `d3dx.ini` first).
fn setup_ini(dir: &Path) -> Result<(), String> {
    let ini_path = dir.join("d3dx.ini");
    if !ini_path.is_file() {
        return Err(format!("{} is missing: the download has no d3dx.ini and the game folder none", ini_path.display()));
    }
    edit_ini(&ini_path, |ini| {
        // 3DMigoto fades changed values per frame over params_fade seconds.
        let ini = set_option(&ini, "System", "params_file", &format!("{FIXES}\\{PARAMS_FILE}"));
        let ini = set_option(&ini, "System", "params_fade", PARAMS_FADE);
        let ini = add_include(&ini);
        let ini = set_option(&ini, "Hunting", "hunting", "0");
        comment_out(&ini, "analyse_frame")
    })
}

/// Whether the mods in `dir` were installed by another client version that recorded itself:
/// what [`maintain`] updates. A folder without a version file (set up by hand, with
/// `install.py`) is left alone.
pub fn update_due(dir: &Path) -> bool {
    matches!(status(dir, VERSION), Status::UpdateAvailable { installed: Some(_) })
}

/// One job at a time in the game folder: the GUI's ([`start`]) and the unattended one
/// ([`maintain`]) never write there together.
static JOB_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// What EDCAS does in the game folder by itself, on a thread, whenever the game is known to be
/// closed (the client's start, a journal Shutdown): undoes what another installer did to the
/// folder ([`repair`]) and brings the shaders to this client's version
/// ([`update_shaders`]) when an older client installed them. Skipped while a job of the GUI
/// runs. Everything goes to the log; Settings shows the version it finds.
pub fn maintain(settings: &crate::settings::Settings) {
    let Some(dir) = crate::ground_tint::game_dir(settings) else { return };
    let want = Wanted::of(&settings.game_graphics);
    thread::spawn(move || {
        let Ok(_guard) = JOB_LOCK.try_lock() else {
            info!("Shader mods: a job is running, the check of the game folder waits for the next game shutdown");
            return;
        };
        match repair(&dir) {
            Ok(Some(what)) => warn!("Shader mods: {what}"),
            Ok(None) => {}
            Err(e) => warn!("Shader mods: another installer changed the game folder and EDCAS could not repair it yet: {e}"),
        }
        if !update_due(&dir) {
            if matches!(status(&dir, VERSION), Status::Installed { .. }) {
                if star_light_pending(&dir) {
                    match finish_game_shaders(&dir, want) {
                        Ok(p) => info!("Shader mods: star light shader copies made from the game's exports: {} of {}{}", p.star_light.made, StarLightShaders::TOTAL, failures(&p.star_light)),
                        Err(e) => warn!("Shader mods: making the star light's shader copies failed: {e}"),
                    }
                    crate::wake::wake();
                } else {
                    // An install of before 0.43.117 left the export on until every copy was there.
                    let on = export_needed(&dir, want);
                    match set_export(&dir, on) {
                        Ok(true) => info!("Shader mods: the game's shader export is {} from its next start ({want:?})", if on { "on" } else { "off" }),
                        Ok(false) => {}
                        Err(e) => warn!("Shader mods: could not set the game's shader export: {e}"),
                    }
                }
            }
            return;
        }
        let installed = fs::read_to_string(dir.join(FIXES).join(VERSION_FILE)).unwrap_or_default().trim().to_string();
        info!("Shader mods: {installed} installed, this client is {VERSION}: updating the shaders by itself");
        match update_shaders(&dir, VERSION, want, &mut |step| info!("Shader mods update: {step}")) {
            Ok(report) => info!("Shader mods: updated to {VERSION} by itself (ground tint: {:?}, star light: {:?})", report.ground_tint, report.star_light),
            Err(e) => warn!("Shader mods: the update to {VERSION} failed, press Update in Settings > Game Graphics: {e}"),
        }
        crate::wake::wake();
    });
}

/// Makes the ground detail textures (46 MB, seconds) and writes them into `ShaderFixes`; the
/// single textures from before the ground types (2026-10-29) go.
fn write_ground_textures(fixes: &Path, progress: &mut impl FnMut(&str)) -> Result<(), String> {
    ground_detail::write_all(fixes, TEXTURE_SIZE, TEXTURE_SIZE, SNOW_TEXTURE_SIZE, |s| progress(s))?;
    for old in OLD_TEXTURES {
        let _ = fs::remove_file(fixes.join(old));
    }
    Ok(())
}

/// Deletes EDCAS's shaders that are no longer in [`SKY_SHADERS`] (the landing dust's ship probe,
/// 2026-10-29) and the [`STALE_SHADERS`]: nothing of ours runs them, but they would stay in
/// `ShaderFixes` for good (the stale ones even run, 3DMigoto loads them by hash).
fn remove_retired_shaders(fixes: &Path) -> Result<(), String> {
    let Ok(read) = fs::read_dir(fixes) else { return Ok(()) };
    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let retired = name.starts_with("edcas") && name.ends_with(".hlsl") && !SKY_SHADERS.iter().any(|(n, _)| *n == name);
        if retired || STALE_SHADERS.contains(&name.as_str()) {
            fs::remove_file(entry.path()).map_err(|e| io_error("delete", &entry.path(), &e))?;
        }
    }
    Ok(())
}

fn check_game(dir: &Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Err(format!("{} doesn't exist: set the Game Folder above", dir.display()));
    }
    if !dir.join(GAME_EXE).is_file() {
        return Err(format!(
            "{} is not Elite's game folder (no {GAME_EXE}): set the Game Folder above to …/Products/elite-dangerous-odyssey-64",
            dir.display()
        ));
    }
    Ok(())
}

/// The zip's files as (path inside, bytes); paths with `/`, none leading out of the folder.
fn read_zip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    const LIMIT: u64 = 256 * 1024 * 1024;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("the download isn't a zip file: {e}"))?;
    let (mut files, mut total) = (Vec::new(), 0u64);
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("the download's zip is damaged: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.enclosed_name().ok_or_else(|| format!("the download's zip has an unsafe path: {}", entry.name()))?;
        let name = name.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/");
        total += entry.size();
        if total > LIMIT {
            return Err("the download's zip is far bigger than expected".into());
        }
        let mut data = Vec::with_capacity(usize::try_from(entry.size()).unwrap_or(0));
        entry.read_to_end(&mut data).map_err(|e| format!("the download's zip is damaged ({name}): {e}"))?;
        files.push((name, data));
    }
    Ok(files)
}

/// Which 3DMigoto the game folder runs, for the log and the diagnostics: whether `d3d11.dll`
/// is EDCAS's build and the version the last game start's `d3d11_log.txt` reports. EDHM
/// (the HUD mod) ships its own 3DMigoto and its installer puts that `d3d11.dll` back; with it
/// the game runs EDCAS's shaders, but `edcas_params.txt` is never read (the params-file
/// patch is EDCAS's), so the per-body values stay at `edcas.ini`'s defaults (a Windows
/// tester, 2026-11-08: "v 1.3.16" in the log, no `params_file=` line).
pub(crate) fn dll_report(dir: &Path) -> String {
    let dll = match fs::read(dir.join("d3d11.dll")) {
        Err(_) => "no d3d11.dll".to_string(),
        Ok(b) if is_edcas_dll(&b) => format!("d3d11.dll is EDCAS's build ({} bytes)", b.len()),
        Ok(b) => format!("d3d11.dll is another 3DMigoto's ({} bytes): edcas_params.txt is not read, the values stay at the ini's defaults", b.len()),
    };
    let log = fs::read_to_string(dir.join("d3d11_log.txt")).unwrap_or_default();
    let version = log
        .lines()
        .find_map(|l| l.split_once("starting init - v ").map(|(_, v)| v.split(" - ").next().unwrap_or(v).trim().to_string()))
        .map(|v| format!("; the last game start logged 3DMigoto v {v}"))
        .unwrap_or_default();
    format!("{dll}{version}")
}

/// Whether the game folder's `d3d11.dll` is another 3DMigoto's (EDHM's installer puts its
/// own back, see [`dll_report`]).
fn foreign_dll(dir: &Path) -> bool {
    fs::read(dir.join("d3d11.dll")).is_ok_and(|b| !is_edcas_dll(&b))
}

/// What another installer (EDHM's, a 3DMigoto release's) has undone in a game folder where
/// EDCAS's mods are installed, one line each, for Settings and the log; empty when the folder
/// is as EDCAS left it (or the mods aren't installed). EDHM_UI's install and repair write
/// their own `d3d11.dll` and `d3dx.ini`: with their dll the shaders run but `edcas_params.txt`
/// is never read; without the include nothing of EDCAS's runs at all.
pub fn damage(dir: &Path) -> Vec<&'static str> {
    if status(dir, VERSION) == Status::NotInstalled {
        return Vec::new();
    }
    let mut found = Vec::new();
    if foreign_dll(dir) {
        found.push("another 3DMigoto's d3d11.dll is in place, so EDCAS's per-planet values don't reach the game");
    }
    if let Some(ini) = read_ini(&dir.join("d3dx.ini")) {
        if !ini.split('\n').any(is_include) {
            found.push("d3dx.ini no longer includes ShaderFixes\\edcas.ini, so none of EDCAS's effects run");
        }
        if get_option(&ini, "System", "params_file").is_none() {
            found.push("d3dx.ini has no params_file line, so EDCAS's per-planet values don't reach the game");
        }
    }
    found
}

/// Undoes what [`damage`] found: EDCAS's `d3d11.dll` goes back in (the other one is kept as
/// `d3d11.dll.official` if none is kept yet) and `d3dx.ini` gets its lines again. `Ok(None)`
/// when there was nothing to do; the message says what was repaired. A foreign dll with no
/// copy to restore from (an install before 0.43.82) and a dll the running game locks are
/// errors: the caller tries again at the next chance, or the player presses Update.
pub fn repair(dir: &Path) -> Result<Option<String>, String> {
    let found = damage(dir);
    if found.is_empty() {
        return Ok(None);
    }
    if foreign_dll(dir) {
        let ours = fs::read(dir.join(EDCAS_DLL_COPY)).ok().filter(|b| is_edcas_dll(b)).ok_or("no copy of EDCAS's d3d11.dll to put back: press Update")?;
        install_dll(dir, &ours)?;
    }
    setup_ini(dir)?;
    Ok(Some(format!(
        "another installer (EDHM's) had changed the game folder: {}. Repaired: EDCAS's d3d11.dll and its d3dx.ini lines are back, the other dll is kept as d3d11.dll.official",
        found.join("; ")
    )))
}

/// An ini as text with `\n` line ends, byte for byte (see [`edit_ini`]); `None` when it can't be read.
fn read_ini(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(bytes.iter().map(|&b| b as char).collect::<String>().replace("\r\n", "\n"))
}

/// Whether a `d3d11.dll` is EDCAS's build: the params-file patch logs `params_file=`.
fn is_edcas_dll(bytes: &[u8]) -> bool {
    const MARK: &[u8] = b"params_file=";
    bytes.windows(MARK.len()).any(|w| w == MARK)
}

/// EDCAS's `d3d11.dll` kept beside the live one, so [`repair`] can put it back after
/// another installer (EDHM's) has overwritten it, without a download.
pub(crate) const EDCAS_DLL_COPY: &str = "d3d11.dll.edcas";

/// Puts EDCAS's `d3d11.dll` in place. Another 3DMigoto's (the release's, EDHM's) is kept
/// once as `d3d11.dll.official`, also when it was switched off as `d3d11.dll.off`.
fn install_dll(dir: &Path, new: &[u8]) -> Result<(), String> {
    let (current, off, official) = (dir.join("d3d11.dll"), dir.join("d3d11.dll.off"), dir.join("d3d11.dll.official"));
    if !official.exists() {
        for candidate in [&current, &off] {
            if fs::read(candidate).is_ok_and(|b| !is_edcas_dll(&b)) {
                fs::copy(candidate, &official).map_err(|e| io_error("copy to", &official, &e))?;
                break;
            }
        }
    }
    write_locked(&current, new)?;
    write_file(&dir.join(EDCAS_DLL_COPY), new)?;
    // The patched DLL is the active one now; a switched-off copy of it (or of the kept
    // original) is of no use. Anything else stays.
    if let Ok(b) = fs::read(&off) {
        if is_edcas_dll(&b) || fs::read(&official).is_ok_and(|o| o == b) {
            let _ = fs::remove_file(&off);
        }
    }
    Ok(())
}

/// Writes a DLL the game may have loaded (unless the same bytes are there already). On
/// Windows a running game locks it, which is reported as such.
fn write_locked(path: &Path, data: &[u8]) -> Result<(), String> {
    if fs::read(path).is_ok_and(|b| b == data) {
        return Ok(());
    }
    let tmp = path.with_extension("dll.edcas-new");
    let result = fs::write(&tmp, data).and_then(|()| fs::rename(&tmp, path));
    if let Err(e) = result {
        let _ = fs::remove_file(&tmp);
        return Err(locked_or(path, &e));
    }
    Ok(())
}

fn write_file(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| io_error("create", parent, &e))?;
    }
    let mut f = fs::File::create(path).map_err(|e| io_error("write", path, &e))?;
    f.write_all(data).map_err(|e| io_error("write", path, &e))
}

fn io_error(what: &str, path: &Path, e: &std::io::Error) -> String {
    format!("can't {what} {}: {e}", path.display())
}

/// The error for a DLL that can't be replaced: on Windows that means the game runs.
fn locked_or(path: &Path, e: &std::io::Error) -> String {
    let locked = cfg!(windows) && (e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(32));
    if locked {
        "close Elite Dangerous first: Windows keeps d3d11.dll locked while the game runs".into()
    } else {
        io_error("replace", path, e)
    }
}

// ------------------------------------------------------------------ remove

/// Takes EDCAS's shader mods out of `dir`, leaving the rest of 3DMigoto (and its settings)
/// alone:
///
/// - the include of `edcas.ini` goes from `d3dx.ini`, and `export_shaders` is switched off
///   if the ground tint was waiting for a landing;
/// - `ShaderFixes` loses `edcas.ini`, `edcas*.hlsl`, the ground textures (`edcas_ground_*.dds`),
///   `edcas_version.txt` and the tinted ground shaders. `edcas_params.txt` stays: it is the player's file too (the hand-tuned
///   glow scale `x505` lives in it), nothing reads it without the include, and a reinstall
///   finds it again;
/// - `d3d11.dll.official` goes back to `d3d11.dll`; without one, EDCAS's own `d3d11.dll` is
///   deleted (there was no 3DMigoto before). A `d3d11.dll` that isn't EDCAS's is not touched.
pub fn remove(dir: &Path) -> Result<String, String> {
    if !dir.is_dir() {
        return Err(format!("{} doesn't exist: set the Game Folder above", dir.display()));
    }
    let ini_path = dir.join("d3dx.ini");
    if ini_path.is_file() {
        edit_ini(&ini_path, |ini| {
            let ini = remove_include(&ini);
            if get_option(&ini, "Rendering", "export_shaders").as_deref() == Some("1") {
                set_option(&ini, "Rendering", "export_shaders", "0")
            } else {
                ini
            }
        })?;
    }

    // edcas.ini first: it is what marks the mods as installed (and EDCAS's params writer
    // stops with it).
    let fixes = dir.join(FIXES);
    let mut doomed = vec![fixes.join(INI_FILE), fixes.join(VERSION_FILE)];
    doomed.extend(GROUND_SHADERS.iter().copied().chain(star_light_shaders().map(|(h, _)| h)).map(|h| fixes.join(format!("{h}-ps.txt"))));
    doomed.extend(STALE_SHADERS.iter().map(|n| fixes.join(n)));
    // Every embedded shader by name: the game-shader replacements are not named edcas*.hlsl.
    doomed.extend(SKY_SHADERS.iter().map(|(n, _)| fixes.join(n)));
    doomed.extend(ground_detail::file_names().iter().map(|n| fixes.join(n)));
    doomed.extend(OLD_TEXTURES.iter().map(|n| fixes.join(n)));
    if let Ok(read) = fs::read_dir(&fixes) {
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name.starts_with("edcas") && name.ends_with(".hlsl") {
                doomed.push(entry.path());
            }
        }
    }
    let mut removed = 0;
    for path in doomed {
        match fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error("delete", &path, &e)),
        }
    }

    let (current, official) = (dir.join("d3d11.dll"), dir.join("d3d11.dll.official"));
    let _ = fs::remove_file(dir.join(EDCAS_DLL_COPY));
    let ours = fs::read(&current).map_or(true, |b| is_edcas_dll(&b));
    let dll = if !ours {
        "d3d11.dll isn't EDCAS's and stays"
    } else if let Ok(original) = fs::read(&official) {
        write_locked(&current, &original)?;
        let _ = fs::remove_file(&official);
        "the original d3d11.dll is back"
    } else if current.is_file() {
        fs::remove_file(&current).map_err(|e| locked_or(&current, &e))?;
        "EDCAS's d3d11.dll is deleted"
    } else {
        "no d3d11.dll to restore"
    };
    info!("Shader mods removed from {} ({removed} files; {dll})", dir.display());
    Ok(format!("Removed {removed} EDCAS file(s); {dll}"))
}

// ------------------------------------------------------------------ background job

/// What the Shader Mods row can do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Job {
    /// Install or update: download, unpack, write, edit `d3dx.ini`.
    Install,
    /// Only the ground tint: the exports are there after landing on a planet.
    FinishGameShaders,
    /// Undo another installer's changes to the folder ([`repair`]), no download.
    Repair,
    Remove,
}

/// What a running job tells the GUI.
pub enum Event {
    /// What it is doing now.
    Step(String),
    /// The end, with the message to show (an error is the `Err`).
    Done(Result<String, String>),
}

/// Runs `job` on a thread of its own in the game folder `dir`; the server to download from
/// is `base_url`.
pub fn start(job: Job, dir: PathBuf, base_url: String, want: Wanted) -> mpsc::Receiver<Event> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let say = |e: Event| {
            let _ = tx.send(e);
            crate::wake::wake();
        };
        info!("Shader mods: {job:?} starts in {} (server {base_url})", dir.display());
        let _guard = JOB_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let result = run(job, &dir, &base_url, want, |s| say(Event::Step(s.to_string())));
        match &result {
            Ok(report) => info!("Shader mods ({job:?}) done: {}", report.replace('\n', " ")),
            Err(e) => warn!("Shader mods ({job:?}) failed: {e}"),
        }
        say(Event::Done(result));
    });
    rx
}

fn run(job: Job, dir: &Path, base_url: &str, want: Wanted, mut progress: impl FnMut(&str)) -> Result<String, String> {
    match job {
        Job::Install => {
            check_game(dir)?;
            progress("Downloading the 3DMigoto files");
            info!("Shader mods install: downloading the 3DMigoto files from {base_url}");
            let bundle = download(base_url)?;
            Ok(installed_message(&install(dir, &bundle, VERSION, want, progress)?, want))
        }
        Job::FinishGameShaders => {
            check_game(dir)?;
            Ok(installed_message(&finish_game_shaders(dir, want)?, want))
        }
        Job::Repair => {
            check_game(dir)?;
            repair(dir).map(|r| r.unwrap_or_else(|| "Nothing to repair: the game folder is as EDCAS left it.".to_string()))
        }
        Job::Remove => remove(dir),
    }
}

fn installed_message(patched: &Patched, want: Wanted) -> String {
    let ground = match &patched.ground_tint {
        GroundTint::NeedsLanding if !want.ground => "Installed. Clouds, fog and frost work now.\nThe ground tint, the ground detail and the dust are off, so the game is not asked for its ground shaders; \
                                                     switch one on, land on a planet once, then press Finish Game Shaders here."
            .into(),
        GroundTint::Done => "Installed. Start the game: clouds, fog, frost and the ground tint and detail are ready.".to_string(),
        GroundTint::NeedsLanding => "Installed. Clouds, fog and frost work now.\nGround tint: start the game, land on a planet once, then press \
                                     Finish Game Shaders here and restart the game."
            .into(),
        GroundTint::Failed(why) => format!("Installed, but the ground tint failed: {why}.\nClouds, fog and frost work."),
    };
    format!("{ground}\n{}", star_light_message(&patched.star_light, want.star_light))
}

/// One line on the star light's shader copies.
pub fn star_light_message(star_light: &StarLightShaders, wanted: bool) -> String {
    let total = StarLightShaders::TOTAL;
    let base = if star_light.done() {
        "Star light: ready, every game shader it needs is copied.".to_string()
    } else if !wanted {
        format!("Star light: {} of {total} game shaders copied. Star Light Colour is off, so the game is not asked for the rest.", star_light.made)
    } else {
        format!(
            "Star light: {} of {total} game shaders copied; the game exports the rest as it runs (space, a planet with an atmosphere, rings, settlements), \
             EDCAS copies them at its next start, or press Finish Game Shaders.",
            star_light.made
        )
    };
    format!("{base}{}", failures(star_light))
}

fn failures(star_light: &StarLightShaders) -> String {
    if star_light.failed.is_empty() {
        String::new()
    } else {
        format!(" Failed: {}", star_light.failed.join("; "))
    }
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;

    /// A small made-up pixel shader in 3DMigoto's asm shape: one colour output `o1`, and the
    /// normal's encoding (`dp3 r2.y, r0.xyzx, cb1[277].xyzx`) that the detail goes in front of.
    const GROUND: &str = include_str!("../testdata/ground_fake_ps.txt");

    const TINTED: &str = "ps_5_0\n\
dcl_globalFlags refactoringAllowed\n\
dcl_sampler s0, mode_default\n\
dcl_resource_texture2d (float,float,float,float) t0\n\
dcl_resource_texture2d (float,float,float,float) t1\n\
dcl_resource_texture1d (float,float,float,float) t120\n\
dcl_input_ps linear v0.xy\n\
dcl_input_ps linear v3.xyzw\n\
dcl_output o0.xyzw\n\
dcl_output o1.xyzw\n\
dcl_output o2.xyzw\n\
dcl_output o3.xyzw\n\
dcl_temps 6\n\
sample_indexable(texture2d)(float,float,float,float) r0.xyzw, v0.xyxx, t0.xyzw, s0\n\
mul r1.xyz, r0.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)\n\
mov r1.w, l(1.000000)\n\
if_nz r0.x\n\
\x20 mad r3.xy, r0.wwww, r1.xyxx, l(0.040000, 0.040000, 0.000000, 0.000000)\n\
else\n\
\x20 mov r3.xy, l(0.040000,0.040000,0,0)\n\
endif\n\
max o0.x, r1.w, l(0.001000)\n\
// EDCAS planet ground tint: x500 = strength, y500 z500 w500 = tint colour\n\
ld_indexable(texture1d)(float,float,float,float) r4.xyzw, l(500, 0, 0, 0), t120.xyzw\n\
dp3 r5.w, r1.xyzx, l(0.299000, 0.587000, 0.114000, 0.000000)\n\
mul r5.xyz, r5.wwww, r4.yzwy\n\
min r5.xyz, r5.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)\n\
add r5.xyz, r5.xyzx, -r1.xyzx\n\
mad r1.xyz, r4.xxxx, r5.xyzx, r1.xyzx\n\
dp3 r2.y, r0.xyzx, cb1[277].xyzx\n\
mov o0.xyzw, r0.xyzw\n\
mov o2.xyzw, r3.xywz\n\
mov o1.xyzw, r1.xyzw\n\
ret\n";

    #[test]
    fn tint_adds_the_ini_params_blend_before_the_colour_output() {
        assert_eq!(tint(GROUND).unwrap(), TINTED);
    }

    #[test]
    fn tint_refuses_what_it_does_not_know() {
        let no_colour = GROUND.replace("mov o1.xyzw, r1.xyzw\n", "");
        assert!(tint(&no_colour).unwrap_err().contains("doesn't look like the ground shader"));
        let twice = GROUND.replace("ret\n", "mov o1.xyzw, r0.xyzw\nret\n");
        assert!(tint(&twice).unwrap_err().contains("doesn't look like the ground shader"));
        let no_temps = GROUND.replace("dcl_temps 4\n", "");
        assert!(tint(&no_temps).unwrap_err().contains("doesn't look like the ground shader"));
        let already = GROUND.replace("ret\n", "ld_indexable(texture1d)(float,float,float,float) r2.xyzw, l(500, 0, 0, 0), t120.xyzw\nret\n");
        assert!(tint(&already).unwrap_err().contains("already reads t120"));
        let no_resource = GROUND.replace("dcl_resource_texture2d (float,float,float,float) t0\n", "").replace("dcl_resource_texture2d (float,float,float,float) t1\n", "");
        assert!(tint(&no_resource).unwrap_err().contains("dcl_resource_"));
    }

    #[test]
    fn tint_reads_windows_line_ends() {
        assert_eq!(tint(&GROUND.replace('\n', "\r\n")).unwrap(), TINTED);
    }

    // ---- the ground detail

    /// The script's result for the made-up shader (`python3 ground_tint.py
    /// crates/client/testdata/ground_fake_ps.txt crates/client/testdata/ground_fake_patched.txt`).
    const PATCHED: &str = include_str!("../testdata/ground_fake_patched.txt");

    #[test]
    fn detail_makes_what_the_script_makes() {
        assert_eq!(patch(GROUND).unwrap(), PATCHED, "ground_tint.py's output for the made-up shader differs: regenerate testdata/ground_fake_patched.txt");
    }

    #[test]
    fn gloss_blends_the_roughness_and_f0_of_ice_before_the_normal_is_encoded() {
        let detailed = detail(&tint(GROUND).unwrap()).unwrap();
        let out = gloss(&detailed).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        let at = |needle: &str| lines.iter().position(|l| *l == needle).unwrap_or_else(|| panic!("{needle} missing"));
        // Nine more scratch registers after the detail's (r30 to r37 here; the detail's r6 to r18), counted in dcl_temps.
        assert!(out.contains("dcl_temps 38\n"));
        // Row 20 (the gloss) and row 14 (the ground type: 2 and 3 only).
        assert!(out.contains("ld_indexable(texture1d)(float,float,float,float) r29.xyzw, l(520, 0, 0, 0), t120.xyzw\n"));
        assert!(out.contains("ld_indexable(texture1d)(float,float,float,float) r30.xyzw, l(514, 0, 0, 0), t120.xyzw\nge r29.z, r30.x, l(1.500000)\nlt r29.w, r30.x, l(3.500000)\n"));
        // Roughness: the game's (r1.w) towards y520, written over o0.x. F0: the game's own (r3.xy, which
        // the shader copies to o2) towards ice's, in place; at strength 0 both are what the game wrote.
        let blend = at("mad r30.x, r30.z, r30.x, r1.w");
        assert_eq!(lines[blend - 1], "add r30.x, r29.y, -r1.w");
        // The sparkle (r32 to r37): the facet's normal and the glint roughness go into the normal (r0 here) and r30.x.
        let end = lines[blend..].iter().position(|l| *l == "endif").map(|i| blend + i).expect("the sparkle's endif");
        assert_eq!(lines[blend + 1], "// EDCAS sparkle: z520 (0 = 1, under 0 = off) widens or narrows the cone of facets that glint");
        assert_eq!(lines[blend + 2], "ld_indexable(texture1d)(float,float,float,float) r36.xyzw, l(520, 0, 0, 0), t120.xyzw");
        assert!(out.contains("deriv_rtx_fine r32.xyz, v3.xyzx\nderiv_rty_fine r33.xyz, v3.xyzx\n"));
        // No fade: the footprint (r36.x) is the level's, from which the cells' size follows, two levels' worth.
        assert!(out.contains("max r32.w, r32.w, r33.w\nsqrt r36.x, r32.w\nmovc r32.w, r36.w, l(0.000000), r29.x\nmov r32.y, l(0.000000)\nlt r33.x, l(0.000000), r32.w\nif_nz r33.x\n"));
        assert!(out.contains("  mul r33.x, r36.x, l(50.000000)\n  max r33.x, r33.x, l(1.000000)\n  log r33.x, r33.x\n  min r33.x, r33.x, l(20.000000)\n  round_ni r32.x, r33.x\n  add r36.y, r33.x, -r32.x\n  exp r32.z, -r32.x\n  mul r32.z, r32.z, l(50.000000)\n  mul r33.xy, v1.xyxx, cb2[0].xxxx\n  mul r33.xy, r33.xyxx, r32.zzzz\n"));
        assert!(out.contains("  ftoi r33.zw, cb2[1].xxxy\n  imul null, r33.xyzw, r33.xyzw, l(0x9e3779b1, 0x85ebca6b, 0xc2b2ae35, 0x27d4eb2f)\n"));
        // The level goes into the hash; the second pass is the level above.
        assert_eq!(out.matches("  ftoi r35.w, r32.x\n  imul null, r35.w, r35.w, l(0x165667b1)\n  xor r33.x, r33.x, r35.w\n").count(), 2);
        assert!(out.contains("  add r32.x, r32.x, l(1.000000)\n  exp r32.z, -r32.x\n"));
        // The dot: the pixel's place in its cell against the larger of 1.5 footprints and 0.15 cell.
        assert!(out.contains("  round_ni r34.xy, r33.xyxx\n  add r33.xy, r33.xyxx, -r34.xyxx\n  add r33.xy, r33.xyxx, l(-0.500000, -0.500000, 0.000000, 0.000000)\n  dp2 r37.w, r33.xyxx, r33.xyxx\n  mul r35.w, r36.x, r32.z\n  mul r35.w, r35.w, l(3.000000)\n  max r35.w, r35.w, l(0.300000)\n  mul r35.w, r35.w, r35.w\n  div r37.w, r37.w, r35.w\n  add r37.w, -r37.w, l(1.000000)\n  max r37.w, r37.w, l(0.000000)\n  mul r37.w, r37.w, r37.w\n  ftoi r33.xy, r34.xyxx\n"));
        // Each level's weight: the finer one's 1 - the fraction (r36.y), the coarser one's the fraction; summed in r32.y.
        assert!(out.contains("  and r33.x, r33.x, r32.w\n  mul r33.x, r33.x, r37.w\n  add r33.y, -r36.y, l(1.000000)\n  mul r33.x, r33.x, r33.y\n  add r32.y, r32.y, r33.x\n"));
        assert!(out.contains("  mul r33.x, r33.x, r37.w\n  mov r33.y, r36.y\n  mul r33.x, r33.x, r33.y\n  add r32.y, r32.y, r33.x\n"));
        assert!(out.contains("  add r37.xyz, r37.xyzx, -cb1[4].xyzx\n"));
        assert!(out.contains("  add r34.xyz, r37.xyzx, -r35.xyzx\n  mad r34.xyz, r34.xyzx, r33.xxxx, r35.xyzx\n"));
        assert!(out.contains("  mul r0.xyz, r34.xyzx, r33.yyyy\n  add r33.y, -r30.x, l(0.250000)\n  mad r30.x, r33.x, r33.y, r30.x\nendif\n"));
        // The broad gloss' weight (b.z = r30.z, times 1 - mask) goes back into a.x after the sparkle, whose glints used a.x before it.
        assert_eq!(lines[end + 1], "mov r29.x, r30.z");
        assert_eq!(lines[end + 2], "max o0.x, r30.x, l(0.001000)");
        // The glints' F0 (0.02) replaces the game's where a glint is (r32.y, the sum of the weights), before the broad blend.
        assert_eq!(lines[end + 3], "mad_sat r33.x, r32.y, l(1.000000), l(0.000000)");
        assert_eq!(lines[end + 4], "add r33.y, l(0.020000), -r3.x");
        assert_eq!(lines[end + 5], "mad r3.xy, r33.xxxx, r33.yyyy, r3.xyxx");
        assert_eq!(lines[end + 6], "add r30.yz, l(0.000000, 0.018000, 0.018000, 0.000000), -r3.xxyx");
        assert_eq!(lines[end + 7], "mad r3.xy, r29.xxxx, r30.yzyy, r3.xyxx");
        // The game's own branch stays as it is.
        assert_eq!(lines[at("if_nz r0.x") + 1], "  mad r3.xy, r0.wwww, r1.xyxx, l(0.040000, 0.040000, 0.000000, 0.000000)");
        // The snow crust (the detail's r18) is matte: the broad gloss' strength (r30.z) is times one minus the mask, before the blend; the sparkle's (r29.x) is not.
        assert!(out.contains("movc r29.x, r29.z, r29.x, l(0.000000)\nadd r30.y, -r18.x, l(1.000000)\nmul r30.z, r29.x, r30.y\neq r30.y, r29.y, l(0.000000)\n"));
        assert!(out.contains("add r30.x, r29.y, -r1.w\nmad r30.x, r30.z, r30.x, r1.w\n"), "{out}");
        // After the detail, before the encoding.
        let detail_at = lines.iter().position(|l| l.starts_with("// EDCAS ground detail")).expect("the detail's comment");
        assert!(detail_at < at("// EDCAS ground gloss: x520 strength (0 = 1, under 0 = off), y520 roughness of polished ice (0 = 0.5); ground types 2, 3 only"));
        // The debug views (y511 = 7 and 8) paint the ground colour the tint blends (r1), before the encoding.
        assert_eq!(lines[end + 8], "ld_indexable(texture1d)(float,float,float,float) r33.xyzw, l(511, 0, 0, 0), t120.xyzw");
        assert_eq!(lines[end + 12], "movc r1.xyz, r34.xxxx, l(1.000000, 1.000000, 1.000000, 0.000000), r1.xyzx");
        assert_eq!(lines[end + 14], "movc r1.xyz, r34.xxxx, r32.wwww, r1.xyzx");
        assert_eq!(lines[end + 15], "dp3 r2.y, r0.xyzx, cb1[277].xyzx");
        // The shader that writes o2.xy itself (7ce82fabb300f52e): both writes go to a spare register,
        // the block blends from it into o2.xy.
        let own = detailed
            .replace("mov o2.xyzw, r3.xywz\n", "")
            .replace("  mad r3.xy, r0.wwww", "  mad o2.xy, r0.wwww")
            .replace("  mov r3.xy, l(0.04", "  mov o2.xy, l(0.04");
        let out = gloss(&own).unwrap();
        assert!(out.contains("if_nz r0.x\n  mad r31.xy, r0.wwww, r1.xyxx, l(0.040000, 0.040000, 0.000000, 0.000000)\nelse\n  mov r31.xy, l(0.04"), "{out}");
        assert!(!out.contains("o2.xy, r0.wwww") && !out.contains("mov o2.xy, l("));
        assert!(out.contains("add r30.yz, l(0.000000, 0.018000, 0.018000, 0.000000), -r31.xxyx\nmad o2.xy, r29.xxxx, r30.yzyy, r31.xyxx\n"));
        assert!(out.contains("dcl_temps 38\n"));
        // No write of o2.xy to redirect and no copy: refused.
        assert!(gloss(&own.replace("  mad o2.xy, r0.wwww", "  mad r3.xy, r0.wwww").replace("  mov o2.xy, l(0.04", "  mov r3.xy, l(0.04")).unwrap_err().contains("no write of o2.xy"));
    }

    #[test]
    fn gloss_refuses_what_it_does_not_know() {
        let detailed = detail(&tint(GROUND).unwrap()).unwrap();
        assert!(gloss(&tint(GROUND).unwrap()).is_ok(), "it needs only the tint");
        assert!(gloss(GROUND).unwrap_err().contains("gloss goes on"), "untinted");
        assert!(gloss(&detailed.replace("max o0.x, r1.w, l(0.001000)\n", "")).unwrap_err().contains("gloss goes on"), "no roughness");
        assert!(gloss(&detailed.replace("dp3 r2.y, r0.xyzx, cb1[277].xyzx\n", "")).unwrap_err().contains("gloss goes on"), "no encoding");
    }

    #[test]
    fn the_mask_writes_the_grounds_distance_to_o4_after_the_colour() {
        let out = mask(&detail(&tint(GROUND).unwrap()).unwrap()).unwrap();
        let at = |needle: &str| out.find(needle).unwrap_or_else(|| panic!("{needle} missing"));
        assert!(at("dcl_output o3.xyzw\ndcl_output o4.x\n") < at("dcl_temps 30\n"));
        assert!(at("mov o1.xyzw, r1.xyzw\n// EDCAS terrain mask") < at("dp3 r29.x, v3.xyzx, v3.xyzx\nsqrt o4.x, r29.x\nret"));
    }

    #[test]
    fn the_mask_refuses_what_it_does_not_know() {
        let detailed = detail(&tint(GROUND).unwrap()).unwrap();
        assert!(mask(&tint(GROUND).unwrap()).is_ok(), "the tint alone is enough");
        assert!(mask(GROUND).unwrap_err().contains("terrain mask"), "untinted");
        assert!(mask(&detailed.replace("dcl_input_ps linear v3.xyzw\n", "")).unwrap_err().contains("terrain mask"), "no position");
        assert!(mask(&detailed.replace("dcl_output o3.xyzw\n", "")).unwrap_err().contains("terrain mask"), "three outputs");
        assert!(mask(&mask(&detailed).unwrap()).unwrap_err().contains("terrain mask"), "twice");
    }

    #[test]
    fn detail_goes_before_the_normal_is_encoded_with_its_textures_and_registers() {
        let out = detail(&tint(GROUND).unwrap()).unwrap();
        let at = |needle: &str| out.find(needle).unwrap_or_else(|| panic!("{needle} missing"));
        assert!(at("// EDCAS ground detail") < at("dp3 r2.y, r0.xyzx, cb1[277].xyzx"));
        assert!(at("// EDCAS planet ground tint") < at("// EDCAS ground detail") && at("// EDCAS ground detail") < at("dp3 r2.y, r0.xyzx, cb1[277].xyzx"));
        // Scratch registers from r6 (the tint took r4, r5; r18 holds the crust's mask; r19 to r27 are the hex-tiling's, r28 the parallax's), counted in dcl_temps; no placeholder left.
        assert!(out.contains("dcl_temps 29\n"));
        assert!(!out.contains('{') && !out.contains('}'));
        for t in ["t110", "t111", "t112", "t113", "t114", "t115"] {
            let declaration = format!("dcl_resource_texture2darray (float,float,float,float) {t}\n");
            assert!(at(&declaration) < at("dcl_resource_texture1d (float,float,float,float) t120"), "{t}");
        }
        // The ground type is IniParams row 14, read into the slice register.
        assert!(out.contains("l(514, 0, 0, 0), t120.xyzw\nmov r17.z, r15.x\n"), "{out}");
        // The snow crust's mask register `{m}` (r18 here), found by `gloss` by the comment line, with the ice colours' strength in its .y; the
        // stones' mask t116 is not read any more (the drifts are noise, ground_tint.py `CRUST_DRIFTS`).
        assert!(out.contains("// EDCAS crust mask: r18.x\nmov r18.x, l(0.000000)\n"));
        assert!(!out.contains("t116.xxxx"));
        // Hex-tiling (ground_tint.py `hex_setup`): y514 in m.w, each rotating group's three sines and cosines
        // (the 8, 2 and 0.5 m layers and two stones' layers), three reads of every stone texture, the y511 = 9 view
        // of the cells.
        assert!(out.contains("mov r18.w, r15.y\n"));
        assert!(out.contains("lt r27.x, r18.w, l(0.000000)\nmovc r23.xyz, r27.xxxx,"));
        assert_eq!(out.matches("sincos r24.x, r23.x, r13.x\n").count(), 7);
        assert_eq!(out.matches("t115.xyzw, s0").count(), 6);
        assert_eq!(out.matches("t112.xyxx, s0").count(), 6);
        assert!(out.contains("eq r15.w, r8.y, l(9.000000)\nmovc r1.xyz, r15.wwww, r18.zzzz, r1.xyzx\n"));
        assert!(out.contains("movc r18.y, r7.x, r15.w, l(0.000000)\n"));
        // The crust's fields are value noise of the point on the planet (three octaves, two of drifts and the snow relief's mounds; the pillows are off, ground_tint.py
        // `SNOW_PILLOWS` is empty; `crust_octave`), no tiled sample of the colour layer: the key constants once an octave, no
        // t110 read but the 32 m layer's, the 8 and 2 m and 0.5 m ones' (three each, hex-tiled) and the three planes'.
        assert_eq!(out.matches("imul null, r23.xyzw, r23.xyzw, l(0x8da6b343,").count(), 6);
        assert_eq!(out.matches("t110.xyzw, s0").count(), 1 + 3 + 3);
        // Far away the cut fades into the plain cover: 1 / (700 - 150) per metre from 150 m, w511 (r8.w here) scaling the distance.
        assert!(out.contains("mad_sat r7.z, r7.z, l(0.001818182), l(-0.272727)\n"), "{out}");
        // The snow is a density and a lace (ground_tint.py `crust_snow`, no threshold of the score): saturate((w + d - T) K) in b.x
        // (r7), the lace giving way to d by the view distance, and none where the cover (j.x = r15.x) is under 0.
        assert!(out.contains("rcp r7.y, r7.y\nmin r7.y, r7.y, l(6.000000)\nadd r7.x, r7.x, l(-0.983333)\nmad_sat r7.x, r7.x, r7.y, l(0.500000)\n"), "{out}");
        assert!(out.contains("lt r7.w, r15.x, l(0.000000)\nmovc r18.x, r7.w, l(0.000000), r7.x\n"), "{out}");
        // The snow's own relief (ground_tint.py `snow_relief`): the scales' fades in b (r7), the height's screen derivatives
        // (b.x = r7.x) join the relief's i (r14), the ice's scaled by v.y (r26.y) first; off the ice (m.y = r18.y = 0) exactly nothing.
        assert!(out.contains("mul r6.y, r18.x, r18.y\nmin r6.y, r6.y, l(1.000000)\n"), "{out}");
        assert!(out.contains("deriv_rtx_fine r7.y, r15.x\nderiv_rty_fine r7.z, r15.x\nadd r7.y, r7.y, r14.z\nadd r7.z, r7.z, r14.w\nmul r7.yz, r7.xyzx, r15.wwww\nmul r14.xy, r14.xyxx, r26.yyyy\nadd r14.xy, r14.xyxx, r7.yzyy\n"), "{out}");
        // The lace from the snow height map (ground_tint.py `snow_lace`: k.y = r16.y smoothstepped between the levels into a.z = r6.z, the strongest of that and the
        // cracks in j.y = r15.y; no noise and no flakes), and the map's relief scaled by it (floors smooth, fragments rough) and by 1 - 0.7 thick (thick = r7.w, from the density in r6.y on entry to `snow_relief`: softer bend, no pillows, softer bend, blue hollows in v.w = r26.w).
        assert!(out.contains("mad_sat r6.z, r16.y, l(32.786885), l(0.377049)\nmad r15.z, r6.z, l(-2.000000), l(3.000000)\nmul r6.z, r6.z, r6.z\nmul r6.z, r6.z, r15.z\nmax r6.z, r6.z, r15.y\n"), "{out}");
        assert!(out.contains("mad r7.x, r6.z, l(0.700000), l(0.300000)\nmad r7.y, r7.w, l(-0.700000), l(1.000000)\nmul r7.x, r7.x, r7.y\nmul r16.y, r16.y, r7.x\nmul r14.zw, r14.xyzw, r7.xxxx\n"), "{out}");
        // The snow's height map, hex-tiled (ground_tint.py `snow_texture`): three vertices' reads of the slopes (t116) and the height (t117), slice 0 (q.z = r17.z
        // is 0 while they are taken and the ground type again after).
        // 3 taps of the slopes and of the height (the 3 hex vertices; the height's x the fractal, y the churn), 2 more of the churn a vertex for its own slopes (forward differences
        // along the vertex's axes, used where the fractal is out of the shading) and the parallax march's 72 + 8 steps of 3 taps each of the BC5 height (explicit mip: sample_l).
        assert_eq!((out.matches("t116.xyxx, s0").count(), out.matches("t117.xxxy, s0").count(), out.matches("t117.xxyx, s0").count(), out.matches("t117.xyzw, s0").count()), (3, 6, 3, 240));
        assert!(out.contains("sample_d_indexable(texture2darray)(float,float,float,float) r12.zw, r17.xyzx, t117.xxxy, s0, r11.xyxx, r11.zwzz\n"), "{out}");
        assert!(out.contains("div r12.x, r25.x, r7.z\nmad r7.xy, r13.xyxx, r12.xxxx, r7.xyxx\n"), "{out}");
        // Under the density 0.7 (POM_FRACTAL_D) the slopes go to the churn's, i.zw - (1 - wF) (i.zw - b.xy), exact from d = 1; the hollows lose the fractal's height the same way (`snow_relief`).
        assert!(out.contains("mad_sat r12.w, r12.w, l(-3.333333), l(3.333333)\nmul r7.xy, r7.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000)\nadd r13.xy, r14.zwzz, -r7.xyxx\nmad r14.zw, -r13.xxxy, r12.wwww, r14.xxzw\n"), "{out}");
        assert!(out.contains("mad_sat r7.x, r6.y, l(-3.333333), l(3.333333)\nmad r16.y, -r16.y, r7.x, r16.y\n"), "{out}");
        // The pebbles' relief strength (k.z = r16.z) cut by the snow density on the ice types only (ground_tint.py `PEBBLE_MIXED`; the mask in f.z = r11.z).
        assert!(out.contains("and r15.w, r15.w, r11.z\nmad r16.z, -r16.z, r15.w, r16.z\n"), "{out}");
        // The mixed zones' ice colour (ground_tint.py `ICE_MIXED`): the polished colour (e = r10) times 1 + the density's ramp (k.w = r16.w, from `snow_relief`) times (tint - 1).
        assert!(out.contains("mov r16.w, r6.y\n"), "{out}");
        assert!(out.contains("\nmul r10.xyz, r10.xyzx, r7.xyzx\nadd r9.xyz, r9.xyzx, -r10.xyzx\n"), "{out}");
        assert_eq!(out.matches("sample_l_indexable(texture2darray)(float,float,float,float) r13.").count(), 240);
        // The parallax (ground_tint.py `pom`): the switch z514 (0 = on, under 0 = off) and the ice types gate it; the shift is z.xy = r28.xy, which every layer's coordinate takes (after
        // the derivatives of the unshifted one), the largest field octave sits in z.w = r28.w.
        assert!(out.contains("ld_indexable(texture1d)(float,float,float,float) r15.xyzw, l(514, 0, 0, 0), t120.xyzw\neq r13.x, r15.z, l(0.000000)\nmovc r12.z, r13.x, l(1.000000), r15.z\n"), "{out}");
        assert!(out.contains("movc r28.xy, r13.xxxx, r28.xyxx, l(0.000000, 0.000000, 0.000000, 0.000000)\n"), "{out}");
        assert!(out.contains("mad r6.zw, r28.xxxy, r7.xxxx, r6.xyzw\n"), "{out}");
        // The march's height (`pom_tap`): (wF F + m (C + the sheet, e.y)) times the weight (e.x), wF the fractal's weight by the density: the mask m the lace's height ramp of F plus the density (g.w), smoothstepped; the lace, the hollows and the relief read slice 0 alone.
        assert!(out.contains("mad r25.xy, r25.xyxx, l(0.160000, 0.160000, 0.000000, 0.000000), l(-0.080000, -0.080000, 0.000000, 0.000000)\nmad_sat r25.z, r25.x, l(32.786885), l(0.377049)\n"), "{out}");
        assert!(out.contains("add r25.z, r25.z, r12.w\nmad_sat r25.z, r25.z, l(6.000000), l(-5.400000)\n"), "{out}");
        assert!(out.contains("mad_sat r25.w, r12.w, l(3.333333), l(-2.333333)\nmul r25.x, r25.x, r25.w\nadd r25.y, r25.y, r10.y\nmad r25.x, r25.z, r25.y, r25.x\nmul r25.x, r25.x, r10.x\n"), "{out}");
        // The density d (g.w = r12.w) gates the weight (0 at 0.05, 1 at 0.15: pure ice has no relief) and weighs the fractal in the height (0 under 0.7, 1 at 1): the ice is flat where snow and ice interleave.
        assert!(out.contains("mad_sat r27.z, r12.w, l(10.000000), l(-0.500000)\nmul r27.y, r27.y, r27.z\nmov r10.x, r27.y\n"), "{out}");
        // The self-shadow darkens the albedo off z.z (0 off the ice), y511 = 10 paints the march's depth.
        assert!(out.contains("mad r9.xyz, r7.wwww, l(0.090000, 0.210000, 0.370000, 0.000000), l(-0.450000, -0.450000, -0.450000, 0.000000)\nmad r9.xyz, r9.xyzx, r28.zzzz, l(1.000000, 1.000000, 1.000000, 0.000000)\nmul r1.xyz, r1.xyzx, r9.xyzx\n"), "{out}");
        assert!(out.contains("eq r15.w, r8.y, l(10.000000)\nmovc r1.xyz, r15.wwww, r28.zzzz, r1.xyzx\n"), "{out}");
        assert!(out.contains("mov r17.z, l(0.000000)\nmov r15.xyz, l(0.000000, 0.000000, 0.000000, 0.000000)\n"), "{out}");
        // Nothing of the ice's cracks shows through full snow: the 2 m and 8 m colour layers and the stones' shade scale by v.y.
        assert!(out.contains("mul r10.yz, r10.xyzx, r26.yyyy\n") && out.contains("mul r16.x, r16.x, r26.y\nmad r6.x, r16.x, l(0.900000), r6.x\nadd r6.x, r6.x, r26.x\n"), "{out}");
        // The ground footprint (ground_tint.py `ground_footprint`) in v.z (r26.z): the screen steps of v3, w511 (r8.w) applied; the
        // fades of the lace and of the snow relief's scales go by it, not by the view distance.
        assert!(out.contains("deriv_rtx_fine r7.xyz, r1.xyzx") || out.contains("deriv_rtx_fine r7.xyz, v3.xyzx\nderiv_rty_fine r9.xyz, v3.xyzx\n"), "{out}");
        assert!(out.contains("div r26.z, r7.w, r9.w\n"), "{out}");
        // The snow's soft white cap, by the mask on the ice types.
        assert!(out.contains("min r9.xyz, r1.xyzx, l(0.920000, 0.920000, 0.920000, 0.000000)\n"), "{out}");
        // The colour register is the tinted shader's: the detail multiplies it.
        assert!(out.contains("mul r1.xyz, r1.xyzx, r6.xxxx\n"));
    }

    #[test]
    fn detail_refuses_what_it_does_not_know() {
        let tinted = tint(GROUND).unwrap();
        let untinted = detail(GROUND).unwrap_err();
        assert!(untinted.contains("tinted"), "{untinted}");
        let no_encode = tinted.replace("dp3 r2.y, r0.xyzx, cb1[277].xyzx\n", "");
        assert!(detail(&no_encode).unwrap_err().contains("tinted"));
        let colour_changes = tinted.replace("mov o0.xyzw, r0.xyzw\n", "mov o0.xyzw, r0.xyzw\nmul r1.xyz, r1.xyzx, r0.xxxx\n");
        assert!(detail(&colour_changes).unwrap_err().contains("changes after the normal is encoded"));
        // A change of the alpha is no matter.
        assert!(detail(&tinted.replace("mov o0.xyzw, r0.xyzw\n", "mov o0.xyzw, r0.xyzw\nmul r1.w, r1.w, r0.x\n")).is_ok());
    }

    #[test]
    fn check_operands_catches_a_name_that_was_not_filled_in() {
        assert!(check_operands(&["mad r1.xyz, -r2.xyzx, |r3.x|, l(1.000000, 2, 3, 4)", "// a comment {x}", "ld_indexable(texture1d)(float,float,float,float) r1.xyzw, l(511, 0, 0, 0), t120.xyzw", "div r1.z, cb2[0].x, r3.y"]).is_ok());
        assert!(check_operands(&["mul {a}.xy, r1.xyxx, r2.xxxx"]).unwrap_err().contains("{a}"));
        assert!(check_operands(&["mul r1.xy, r1.xyxx, 0.500000"]).is_err());
        assert!(check_operands(&["mul r1.xy, r1.xyxxx, r2.x"]).is_err());
    }

    #[test]
    fn the_detail_template_is_the_scripts() {
        let Some(dir) = scripts_dir() else { return };
        assert_eq!(GROUND_DETAIL, fs::read_to_string(dir.join("edcas_ground_detail.asm")).unwrap(), "cp scripts/ed-shader/edcas_ground_detail.asm crates/client/shader_mods/");
        // And the script still makes it (where python3 is).
        if let Ok(out) = std::process::Command::new("python3").arg(dir.join("ground_tint.py")).arg("--template").output() {
            if out.status.success() {
                assert_eq!(GROUND_DETAIL, String::from_utf8_lossy(&out.stdout), "ground_tint.py --template makes something else: run it into scripts/ed-shader/edcas_ground_detail.asm and crates/client/shader_mods/");
            }
        }
    }

    // ---- the embedded files follow scripts/ed-shader

    fn scripts_dir() -> Option<PathBuf> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ed-shader");
        dir.is_dir().then_some(dir)
    }

    #[test]
    fn the_embedded_shaders_are_the_ones_install_py_installs() {
        let Some(dir) = scripts_dir() else { return };
        let py = fs::read_to_string(dir.join("install.py")).unwrap();
        let tuple = py.split("SKY_SHADERS = (").nth(1).and_then(|s| s.split(')').next()).expect("SKY_SHADERS in install.py");
        let listed: Vec<&str> = tuple.split('\'').skip(1).step_by(2).collect();
        let embedded: Vec<&str> = SKY_SHADERS.iter().map(|(n, _)| *n).collect();
        assert_eq!(embedded, listed, "SKY_SHADERS in shader_mods.rs and install.py differ");

        let tint_py = fs::read_to_string(dir.join("ground_tint.py")).unwrap();
        let hashes: Vec<&str> = tint_py.split("GROUND_SHADERS = (").nth(1).and_then(|s| s.split(')').next()).unwrap().split('\'').skip(1).step_by(2).collect();
        assert_eq!(GROUND_SHADERS.to_vec(), hashes, "GROUND_SHADERS in shader_mods.rs and ground_tint.py differ");

        let star_py = fs::read_to_string(dir.join("star_light.py")).unwrap();
        let tuple = |name: &str| -> Vec<String> {
            star_py.split(&format!("{name} = (")).nth(1).and_then(|s| s.split(')').next()).unwrap().split('\'').skip(1).step_by(2).map(str::to_string).collect()
        };
        assert_eq!(SUN_SHADERS.to_vec(), tuple("SUN_SHADERS"), "SUN_SHADERS in shader_mods.rs and star_light.py differ");
        assert_eq!(ATMOSPHERE_SHADERS.to_vec(), tuple("ATMOSPHERE_SHADERS"), "ATMOSPHERE_SHADERS in shader_mods.rs and star_light.py differ");
        assert_eq!(FORWARD_SHADERS.to_vec(), tuple("FORWARD_SHADERS"), "FORWARD_SHADERS in shader_mods.rs and star_light.py differ");
        assert_eq!(LIGHT_LIST_SHADERS.to_vec(), tuple("LIGHT_LIST_SHADERS"), "LIGHT_LIST_SHADERS in shader_mods.rs and star_light.py differ");
    }

    #[test]
    fn the_embedded_files_equal_scripts_ed_shader() {
        let Some(dir) = scripts_dir() else { return };
        for (name, text) in SKY_SHADERS.iter().chain([&(INI_FILE, EDCAS_INI)]) {
            let original = fs::read_to_string(dir.join(name)).unwrap();
            assert_eq!(
                *text, original,
                "crates/client/shader_mods/{name} differs from scripts/ed-shader/{name}: cp scripts/ed-shader/{name} crates/client/shader_mods/"
            );
        }
    }

    // ---- d3dx.ini

    #[test]
    fn set_option_replaces_adds_and_uncomments() {
        let ini = "[System]\nfoo = 1\n; params_file = old\n\n[Hunting]\n; hunting = 1\n\n[Rendering]\nexport_shaders=1\n";
        // A commented-out line is the key's home when there is no active one.
        let out = set_option(ini, "System", "params_file", "ShaderFixes\\p.txt");
        assert!(out.starts_with("[System]\nfoo = 1\nparams_file = ShaderFixes\\p.txt\n\n[Hunting]"), "{out}");
        let out = set_option(&out, "System", "params_file", "new");
        assert_eq!(out.matches("params_file").count(), 1, "{out}");
        // An active line wins over a commented one before it.
        let out = set_option("[A]\n; k = 1\nk = 2\n", "A", "k", "3");
        assert_eq!(out, "[A]\n; k = 1\nk = 3\n");
        // No line at all: added under the header.
        let out = set_option("[A]\nz500 = 1\n", "A", "k", "3");
        assert_eq!(out, "[A]\nk = 3\nz500 = 1\n");
        let out = set_option(ini, "Hunting", "hunting", "0");
        assert!(out.contains("[Hunting]\nhunting = 0\n\n[Rendering]"), "{out}");
        let out = set_option(ini, "Rendering", "export_shaders", "0");
        assert!(out.ends_with("[Rendering]\nexport_shaders = 0\n"), "{out}");
        let out = set_option(ini, "Missing", "a", "b");
        assert!(out.ends_with("export_shaders=1\n\n[Missing]\na = b\n"), "{out}");
        // Only inside the section, and keys that merely start the same stay.
        let out = set_option("[A]\nhunting_x = 1\n[B]\nhunting = 5\n", "A", "hunting", "0");
        assert_eq!(out, "[A]\nhunting = 0\nhunting_x = 1\n[B]\nhunting = 5\n");
        // Section and key names ignore case.
        assert_eq!(set_option("[hunting]\nHUNTING=2\n", "Hunting", "hunting", "0"), "[hunting]\nhunting = 0\n");
    }

    #[test]
    fn get_option_reads_active_lines_of_the_section_only() {
        let ini = "[A]\nx500 = 1\n[B]\n;x = 2\ny500 = 3\n";
        assert_eq!(get_option(ini, "A", "x500").as_deref(), Some("1"));
        assert_eq!(get_option(ini, "B", "x500"), None);
        assert_eq!(get_option(ini, "B", "y500").as_deref(), Some("3"));
        assert_eq!(get_option(ini, "C", "y500"), None);
    }

    #[test]
    fn include_is_added_once_and_removed() {
        let ini = "[Include]\n; nothing\n\n[Other]\na = 1\n";
        let once = add_include(ini);
        assert_eq!(once, "[Include]\ninclude = ShaderFixes\\edcas.ini\n; nothing\n\n[Other]\na = 1\n");
        assert_eq!(add_include(&once), once);
        assert_eq!(remove_include(&once), ini);
        let without = add_include("[Other]\na = 1\n");
        assert!(without.ends_with("[Include]\ninclude = ShaderFixes\\edcas.ini\n"), "{without}");
        // Another include and a commented one are not ours.
        let ini = "[Include]\ninclude = ShaderFixes\\other.ini\n;include = ShaderFixes\\edcas.ini\ninclude_recursive = x\n";
        assert_eq!(add_include(ini).matches("edcas.ini").count(), 2);
        assert_eq!(remove_include("[Include]\nInclude=shaderfixes\\EDCAS.ini\n"), "[Include]\n");
    }

    #[test]
    fn comment_out_only_touches_active_lines_of_that_key() {
        let out = comment_out("analyse_frame = 1\n;analyse_frame = 2\nanalyse_frame_x = 3\n", "analyse_frame");
        assert_eq!(out, ";analyse_frame = 1\n;analyse_frame = 2\nanalyse_frame_x = 3\n");
    }

    #[test]
    fn edit_ini_keeps_line_ends_and_foreign_bytes() {
        let tmp = temp("edit-ini");
        let path = tmp.join("d3dx.ini");
        fs::write(&path, b"[Hunting]\r\n; caf\xe9 \xff\r\nhunting = 1\r\n").unwrap();
        edit_ini(&path, |i| set_option(&i, "Hunting", "hunting", "0")).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"[Hunting]\r\n; caf\xe9 \xff\r\nhunting = 0\r\n");
        edit_ini(&path, |i| i).unwrap();
        fs::write(&path, b"[A]\nx500 = 1\n").unwrap();
        edit_ini(&path, |i| set_option(&i, "A", "x500", "2")).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"[A]\nx500 = 2\n");
    }

    // ---- checksum

    #[test]
    fn checksum_is_checked_against_the_list() {
        let data = b"hello";
        // sha256("hello")
        let hash = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        let sums = format!("{}  other.zip\n{hash}  edcas.zip\n", "0".repeat(64));
        verify_checksum(data, &sums, "edcas.zip").unwrap();
        verify_checksum(data, &format!("{}  edcas.zip\n", hash.to_uppercase()), "edcas.zip").unwrap();
        verify_checksum(data, &format!("{hash} *edcas.zip\n"), "edcas.zip").unwrap();
        assert!(verify_checksum(b"HELLO", &sums, "edcas.zip").unwrap_err().contains("damaged"));
        assert!(verify_checksum(data, &sums, "missing.zip").unwrap_err().contains("doesn't list"));
        assert!(verify_checksum(data, "", "edcas.zip").unwrap_err().contains("doesn't list"));
    }

    // ---- install and remove on a made-up game folder

    /// A folder under the system temp dir, deleted when the test drops it (also on a failed
    /// assertion: a panic unwinds through the drop).
    struct Temp(PathBuf);

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Temp {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl AsRef<Path> for Temp {
        fn as_ref(&self) -> &Path {
            &self.0
        }
    }

    fn temp(name: &str) -> Temp {
        let dir = std::env::temp_dir().join(format!("edcas-shader-mods-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Temp(dir)
    }

    const EDCAS_DLL: &[u8] = b"MZ patched 3dmigoto  params_file=%S  ";
    const STOCK_DLL: &[u8] = b"MZ stock 3dmigoto";
    const D3DX: &str = "; the release's d3dx.ini\r\n[Logging]\r\n[System]\r\n[Hunting]\r\nhunting = 1\r\n[Rendering]\r\n;export_shaders = 1\r\n[Include]\r\n; none\r\n[KeyAnalyse]\r\nanalyse_frame = 1\r\n";

    /// The zip CI builds: the release's files with EDCAS's d3d11.dll, at the root.
    fn bundle(dll: &[u8]) -> Bundle {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in [
            ("d3d11.dll", dll),
            ("d3dcompiler_47.dll", b"compiler v2".as_slice()),
            ("d3dx.ini", D3DX.as_bytes()),
            ("ShaderFixes/", b"".as_slice()),
            ("ShaderFixes/default.hlsl", b"// release shader".as_slice()),
            ("EDCAS-README.txt", b"readme".as_slice()),
            ("source/params-file.patch", b"patch".as_slice()),
        ] {
            if name.ends_with('/') {
                zip.add_directory(name, opts).unwrap();
            } else {
                zip.start_file(name, opts).unwrap();
                zip.write_all(data).unwrap();
            }
        }
        Bundle { zip: zip.finish().unwrap().into_inner() }
    }

    fn game(name: &str) -> Temp {
        let dir = temp(name);
        fs::write(dir.join(GAME_EXE), b"exe").unwrap();
        dir
    }

    fn read(dir: &Path, rel: &str) -> String {
        fs::read_to_string(dir.join(rel)).unwrap()
    }

    #[test]
    fn another_installers_changes_are_found_and_repaired_when_the_mods_are_installed() {
        let dir = game("edhm");
        // Nothing installed: a foreign dll is none of EDCAS's business.
        fs::write(dir.join("d3d11.dll"), STOCK_DLL).unwrap();
        assert!(damage(&dir).is_empty());
        assert_eq!(repair(&dir).unwrap(), None);

        install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap();
        assert_eq!(fs::read(dir.join("d3d11.dll")).unwrap(), EDCAS_DLL);
        assert_eq!(fs::read(dir.join(EDCAS_DLL_COPY)).unwrap(), EDCAS_DLL, "the install keeps a copy to restore from");
        assert_eq!(fs::read(dir.join("d3d11.dll.official")).unwrap(), STOCK_DLL);
        assert!(damage(&dir).is_empty());
        assert_eq!(repair(&dir).unwrap(), None, "the folder is as we left it: nothing to do");

        // EDHM's installer runs: its own dll over ours, its own d3dx.ini without our lines.
        fs::write(dir.join("d3d11.dll"), b"MZ edhm 3dmigoto 1.3.16").unwrap();
        fs::write(dir.join("d3dx.ini"), "; EDHM's d3dx.ini\r\n[System]\r\n[Include]\r\ninclude = ShaderFixes\\EDHM.ini\r\n").unwrap();
        let found = damage(&dir);
        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found[0].contains("d3d11.dll") && found[1].contains("includes") && found[2].contains("params_file"), "{found:?}");
        assert!(repair(&dir).unwrap().unwrap().contains("Repaired"));
        assert_eq!(fs::read(dir.join("d3d11.dll")).unwrap(), EDCAS_DLL);
        assert_eq!(fs::read(dir.join("d3d11.dll.official")).unwrap(), STOCK_DLL, "the first kept original stays the one to restore on Remove");
        let ini = read(&dir, "d3dx.ini");
        assert!(ini.contains("include = ShaderFixes\\EDHM.ini\r\n") && ini.contains("include = ShaderFixes\\edcas.ini\r\n"), "{ini}");
        assert_eq!(get_option(&ini.replace("\r\n", "\n"), "System", "params_file").as_deref(), Some("ShaderFixes\\edcas_params.txt"));
        assert!(damage(&dir).is_empty());
        assert_eq!(repair(&dir).unwrap(), None);

        // Only the dll replaced, and the copy to restore from gone (an install before 0.43.82).
        fs::write(dir.join("d3d11.dll"), b"MZ edhm 3dmigoto 1.3.16").unwrap();
        fs::remove_file(dir.join(EDCAS_DLL_COPY)).unwrap();
        assert_eq!(damage(&dir).len(), 1);
        assert!(repair(&dir).unwrap_err().contains("press Update"));
        install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap();
        assert!(damage(&dir).is_empty());

        // Remove takes the copy away with the rest.
        remove(&dir).unwrap();
        assert!(!dir.join(EDCAS_DLL_COPY).exists());
        assert_eq!(fs::read(dir.join("d3d11.dll")).unwrap(), STOCK_DLL);
    }

    #[test]
    fn an_older_clients_install_is_brought_to_this_version_without_a_download() {
        let dir = game("older");
        export_ground_shaders(&dir);
        install(&dir, &bundle(EDCAS_DLL), "0.9.0", Wanted::ALL, |_| {}).unwrap();
        // The old client's shaders: one file with other text, one that the new version retired.
        fs::write(dir.join(FIXES).join("edcas_clouds.hlsl"), "// old clouds").unwrap();
        fs::write(dir.join(FIXES).join("edcas_ship_probe.hlsl"), "// retired").unwrap();
        assert_eq!(status(&dir, "1.0.0"), Status::UpdateAvailable { installed: Some("0.9.0".to_string()) });

        let mut steps = Vec::new();
        let report = update_shaders(&dir, "1.0.0", Wanted::ALL, &mut |s| steps.push(s.to_string())).unwrap();
        assert_eq!(report.ground_tint, GroundTint::Done);
        assert_eq!(status(&dir, "1.0.0"), Status::Installed { ground_tint: true, star_light: 0 });
        assert_eq!(read(&dir, "ShaderFixes/edcas_clouds.hlsl"), SKY_SHADERS.iter().find(|(n, _)| *n == "edcas_clouds.hlsl").unwrap().1);
        assert!(!dir.join(FIXES).join("edcas_ship_probe.hlsl").exists(), "retired shaders go");
        assert_eq!(read(&dir, "ShaderFixes/edcas.ini"), EDCAS_INI);
        for h in GROUND_SHADERS {
            assert_eq!(read(&dir, &format!("ShaderFixes/{h}-ps.txt")), PATCHED);
        }
        assert_eq!(fs::read(dir.join("d3d11.dll")).unwrap(), EDCAS_DLL, "3DMigoto's files are not the update's business");
        assert!(steps.iter().any(|s| s == "Writing the EDCAS shaders") && !steps.iter().any(|s| s.contains("ownload")), "{steps:?}");
    }

    #[test]
    fn the_unattended_update_is_due_only_for_a_recorded_older_version() {
        let dir = game("due");
        assert!(!update_due(&dir), "nothing installed");
        install(&dir, &bundle(EDCAS_DLL), VERSION, Wanted::ALL, |_| {}).unwrap();
        assert!(!update_due(&dir), "this version");
        fs::write(dir.join(FIXES).join(VERSION_FILE), "0.9.0\n").unwrap();
        assert!(update_due(&dir), "an older client's");
        fs::remove_file(dir.join(FIXES).join(VERSION_FILE)).unwrap();
        assert!(!update_due(&dir), "set up by hand: left alone");
    }

    /// Exports like the game's, for all three ground shaders.
    fn export_ground_shaders(dir: &Path) {
        fs::create_dir_all(dir.join("ShaderCache")).unwrap();
        for h in GROUND_SHADERS {
            fs::write(exported_path(dir, h), GROUND).unwrap();
        }
    }

    /// A small made-up lighting shader in the shape of the sun pass: the sun's colour read twice.
    const SUN: &str = "ps_5_0\n\
dcl_globalFlags refactoringAllowed\n\
dcl_constantbuffer CB2[47], immediateIndexed\n\
dcl_resource_texture2d (float,float,float,float) t0\n\
dcl_resource_texture2d (float,float,float,float) t1\n\
dcl_input_ps linear v0.xy\n\
dcl_output o0.xyz\n\
dcl_temps 3\n\
sample_indexable(texture2d)(float,float,float,float) r0.xyzw, v0.xyxx, t0.xyzw, s0\n\
mul r1.xyz, r0.xyzx, cb2[40].xyzx\n\
if_nz r0.w\n\
  mad r2.xyz, r1.wwww, cb2[40].xxyz, r1.xyzx\n\
endif\n\
mul o0.xyz, r2.xyzx, cb2[0].xxxx\n\
ret\n";

    /// The sun fixture patched: the declarations, the tint block, the tinted reads.
    fn sun_patched() -> String {
        format!(
            "ps_5_0\n\
dcl_globalFlags refactoringAllowed\n\
dcl_constantbuffer CB2[47], immediateIndexed\n\
dcl_resource_texture2d (float,float,float,float) t0\n\
dcl_resource_texture2d (float,float,float,float) t1\n\
dcl_resource_texture1d (float,float,float,float) t120\n\
dcl_input_ps linear v0.xy\n\
dcl_output o0.xyz\n\
dcl_temps 5\n\
{}\n\
mul r3.xyz, r3.xyzx, cb2[40].xyzx\n\
sample_indexable(texture2d)(float,float,float,float) r0.xyzw, v0.xyxx, t0.xyzw, s0\n\
mul r1.xyz, r0.xyzx, r3.xyzx\n\
if_nz r0.w\n\
  mad r2.xyz, r1.wwww, r3.xxyz, r1.xyzx\n\
endif\n\
mul o0.xyz, r2.xyzx, cb2[0].xxxx\n\
ret\n",
            tint_block("r3", "r4", Some("v0")).join("\n")
        )
    }

    /// Exports for every star light shader: the fake sun pass, with the sky pass's register
    /// for the atmosphere's.
    fn export_star_light_shaders(dir: &Path) {
        fs::create_dir_all(dir.join("ShaderCache")).unwrap();
        for (h, source) in star_light_shaders() {
            let text = match source {
                SunSource::Row(row) => SUN.replace("cb2[40]", row),
                SunSource::List(_) => GLASS.to_string(),
            };
            fs::write(exported_path(dir, h), text).unwrap();
        }
    }

    /// A small made-up glass shader: the sun's record read outside a loop, every record's inside one.
    const GLASS: &str = "ps_5_0\n\
dcl_globalFlags refactoringAllowed\n\
dcl_constantbuffer CB1[246], immediateIndexed\n\
dcl_resource_raw t1\n\
dcl_resource_texture2d (float,float,float,float) t3\n\
dcl_input_ps linear v0.xy\n\
dcl_output o0.xyz\n\
dcl_temps 6\n\
ld_raw_indexable(raw_buffer)(mixed,mixed,mixed,mixed) r0.xyz, l(0), t1.xyzx\n\
ld_raw_indexable(raw_buffer)(mixed,mixed,mixed,mixed) r1.xyz, l(16), t1.xyzx\n\
mov r2.xyz, r0.xyzx\n\
mov r3.w, l(1)\n\
loop\n\
  uge r4.w, r3.w, cb1[245].y\n\
  breakc_nz r4.w\n\
  imul null, r4.w, r3.w, l(48)\n\
  ld_raw_indexable(raw_buffer)(mixed,mixed,mixed,mixed) r4.xyz, r4.w, t1.xyzx\n\
  imad r4.w, l(48), r3.w, l(16)\n\
  ld_raw_indexable(raw_buffer)(mixed,mixed,mixed,mixed) r5.xyz, r4.w, t1.xyzx\n\
  mad r2.xyz, r4.xyzx, r5.xyzx, r2.xyzx\n\
  iadd r3.w, r3.w, l(1)\n\
endloop\n\
mov o0.xyz, r2.xyzx\n\
ret\n";

    #[test]
    fn star_light_list_tints_the_colour_load_of_every_record_and_leaves_the_directions() {
        let out = star_light_list(GLASS, "t1").unwrap();
        let at = |needle: &str| out.find(needle).unwrap_or_else(|| panic!("{needle} missing in {out}"));
        assert!(at("dcl_resource_raw t1\ndcl_resource_texture2d (float,float,float,float) t3\ndcl_resource_texture1d (float,float,float,float) t120\n") < at("dcl_temps 8\n"));
        assert!(at("dcl_temps 8\n// EDCAS star light") < at("ld_indexable(texture1d)(float,float,float,float) r6.xyzw, l(526, 0, 0, 0), t120.xyzw\nld_indexable(texture1d)(float,float,float,float) r7.xyzw, l(515, 0, 0, 0), t120.xyzw\n"));
        assert!(out.contains("r0.xyz, l(0), t1.xyzx\nmul r0.xyz, r0.xyzx, r6.xyzx\nld_raw_indexable(raw_buffer)(mixed,mixed,mixed,mixed) r1.xyz, l(16), t1.xyzx\nmov r2.xyz"), "{out}");
        // (The fixture's loop body lost its indentation in the string literal; the real shader's is kept.)
        assert!(out.contains("r4.xyz, r4.w, t1.xyzx\nmul r4.xyz, r4.xyzx, r6.xyzx\nimad r4.w"), "{out}");
        assert_eq!(out.matches(", r6.xyzx\n").count(), 3, "two colour loads tinted (and the blend's own line): {out}");
        assert!(!out.contains("mul r1.") && !out.contains("mul r5."), "the directions stay: {out}");
        assert!(star_light_list(&out, "t1").unwrap_err().contains("already reads t120"));
        assert!(star_light_list(SUN, "t1").unwrap_err().contains("no paired loads"));
    }

    #[test]
    fn star_light_tints_both_reads_of_the_suns_colour_once_after_the_declarations() {
        let patched = sun_patched();
        assert_eq!(star_light(SUN, "cb2[40]").unwrap(), patched);
        let sky = star_light(&SUN.replace("cb2[40]", "cb1[8]"), "cb1[8]").unwrap();
        assert!(sky.contains("mul r3.xyz, r3.xyzx, cb1[8].xyzx\n") && !sky.contains("cb1[8].xxyz"), "{sky}");
        // The generated lines pass the operand check the ground detail's do.
        let ours: Vec<&str> = patched.lines().filter(|l| l.contains("r3.") || l.contains("r4.")).filter(|l| !l.starts_with("sincos")).collect();
        check_operands(&ours).unwrap();
        // With the screen position declared (the sun pass has it) the hue runs in bands out from the centre.
        assert!(patched.contains("frc r4.x, r4.x\nadd r4.yz, v0.xxyx, l(0.000000, -0.500000, -0.500000, 0.000000)\ndiv r4.w, cb2[1].x, cb2[1].y\nmul r4.y, r4.y, r4.w\ndp2 r4.w, r4.yzyy, r4.yzyy\nsqrt r4.w, r4.w\nmad r4.x, r4.w, l(-1.000000), r4.x\nmul r4.x, r4.x, l(6.283185)\n"), "{patched}");
        let no_uv = SUN.replace("dcl_input_ps linear v0.xy\n", "dcl_input_ps linear v0.xyz\n");
        assert!(star_light(&no_uv, "cb2[40]").unwrap().contains("frc r4.x, r4.x\nmul r4.x, r4.x, l(6.283185)\n"), "no screen position, no bands");
        assert!(!sky.contains("dp2 r4.w"), "the bands are the sun pass's alone");
        // The rainbow: the clock row, three phases a third of a turn apart, the blend by x527.
        assert!(patched.contains("l(515, 0, 0, 0), t120.xyzw\nmul r4.x, r4.z, l(0.300000)\nfrc r4.x, r4.x\n") && patched.contains("sincos null, r4.xyz, r4.xyzx\n") && patched.contains("ld_indexable(texture1d)(float,float,float,float) r4.w, l(527, 0, 0, 0), t120.xxxx\nmov_sat r4.w, r4.w\n"), "{patched}");
    }

    /// `star_light.<function>(asm, arg)` of `scripts/ed-shader/star_light.py`, where python3 is.
    fn star_light_py(function: &str, asm: &str, arg: &str) -> Option<String> {
        let dir = scripts_dir()?;
        let script = "import sys; sys.path.insert(0, sys.argv[1]); import star_light; sys.stdout.write(getattr(star_light, sys.argv[2])(sys.stdin.read(), sys.argv[3]))";
        let mut child = std::process::Command::new("python3")
            .args(["-c", script, dir.to_str()?, function, arg])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .ok()?;
        child.stdin.take()?.write_all(asm.as_bytes()).ok()?;
        let out = child.wait_with_output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }

    #[test]
    fn star_light_py_patches_as_the_port_does() {
        let sky = SUN.replace("cb2[40]", "cb1[8]");
        for (asm, source) in [(SUN, "cb2[40]"), (sky.as_str(), "cb1[8]")] {
            let Some(py) = star_light_py("star_light", asm, source) else { return };
            assert_eq!(star_light(asm, source).unwrap(), py, "star_light.py and shader_mods::star_light differ on the made-up shader ({source})");
        }
        if let Some(py) = star_light_py("star_light_list", GLASS, "t1") {
            assert_eq!(star_light_list(GLASS, "t1").unwrap(), py, "star_light.py and shader_mods::star_light_list differ on the made-up glass shader");
        }
    }

    #[test]
    fn star_light_takes_any_number_of_reads_and_refuses_a_shader_without_one() {
        let one_read = SUN.replacen("mad r2.xyz, r1.wwww, cb2[40].xxyz, r1.xyzx", "mov r2.xyz, r1.xyzx", 1);
        let out = star_light(&one_read, "cb2[40]").unwrap();
        assert_eq!(out.matches("cb2[40].").count(), 1, "only the tint reads it now: {out}");
        let four = SUN.replacen("endif\n", "  dp3 r2.w, r0.xyzx, cb2[40].xyzx\n  add r2.w, r2.w, cb2[40].x\nendif\n", 1);
        let out = star_light(&four, "cb2[40]").unwrap();
        assert!(out.contains("  dp3 r2.w, r0.xyzx, r3.xyzx\n  add r2.w, r2.w, r3.x\n") && out.matches("cb2[40].").count() == 1, "{out}");
        assert!(star_light(&sun_patched(), "cb2[40]").unwrap_err().contains("already reads t120"));
        assert!(star_light(SUN, "cb1[8]").unwrap_err().contains("no read of cb1[8]"));
    }

    /// The real shaders, when the game's exports are at hand (`ed-captures/shaders-cache-2026-11-09`,
    /// git-ignored): every one patches, and the copies go to `target/star-light/` for
    /// `scripts/ed-shader/renderdoc/dxbc2spv.py` to assemble and the replay to run.
    #[test]
    fn star_light_patches_the_games_own_shaders_when_their_dumps_are_at_hand() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dumps = [repo.join("ed-captures/shaders-cache-2026-11-09"), repo.join("ed-captures/shaders-cache-2026-11-09-ring")];
        if !dumps[0].is_dir() {
            eprintln!("skipped: {} is not here", dumps[0].display());
            return;
        }
        let out = repo.join("target/star-light");
        fs::create_dir_all(&out).unwrap();
        for (hash, source) in star_light_shaders() {
            let file = format!("{hash}-ps.txt");
            let asm = dumps.iter().map(|d| d.join(&file)).find(|p| p.is_file()).map(|p| fs::read_to_string(p).unwrap()).unwrap_or_else(|| panic!("{file}: no dump"));
            let patched = source.patch(&asm).unwrap_or_else(|e| panic!("{hash}: {e}"));
            if let SunSource::Row(row) = source {
                assert_eq!(patched.matches(&format!("{row}.")).count(), 1, "{hash}: only the tint reads {row} now");
            }
            fs::write(out.join(format!("{hash}-ps.txt")), patched).unwrap();
        }
    }

    #[test]
    fn install_into_a_fresh_folder_waits_for_a_landing() {
        let dir = game("fresh");
        assert_eq!(status(&dir, "1.0.0"), Status::NotInstalled);
        // Textures from before the ground types are of no use any more.
        fs::create_dir_all(dir.join("ShaderFixes")).unwrap();
        fs::write(dir.join("ShaderFixes/edcas_ground_detail.dds"), "old").unwrap();
        // Nor shaders taken out of SKY_SHADERS.
        fs::write(dir.join("ShaderFixes/edcas_ship_ref.hlsl"), "old").unwrap();
        // The landing dust redraw's shaders (removed 2026-11-03) would still run by their hash.
        for name in STALE_SHADERS {
            fs::write(dir.join("ShaderFixes").join(name), "old").unwrap();
        }
        let mut steps = Vec::new();
        let report = install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |s| steps.push(s.to_string())).unwrap();
        assert_eq!(report.ground_tint, GroundTint::NeedsLanding);
        assert!(steps.len() >= 4, "{steps:?}");

        assert_eq!(fs::read(dir.join("d3d11.dll")).unwrap(), EDCAS_DLL);
        assert!(!dir.join("d3d11.dll.official").exists(), "there was no other d3d11.dll to keep");
        assert_eq!(read(&dir, "d3dcompiler_47.dll"), "compiler v2");
        assert_eq!(read(&dir, "ShaderFixes/default.hlsl"), "// release shader");
        assert_eq!(read(&dir, "EDCAS-README.txt"), "readme");
        assert_eq!(read(&dir, "source/params-file.patch"), "patch");
        assert_eq!(read(&dir, "ShaderFixes/edcas.ini"), EDCAS_INI);
        for name in ground_detail::file_names() {
            assert!(dir.join("ShaderFixes").join(&name).metadata().unwrap().len() > 148, "{name}");
        }
        assert!(!dir.join("ShaderFixes/edcas_ground_detail.dds").exists());
        assert!(!dir.join("ShaderFixes/edcas_ship_ref.hlsl").exists());
        for name in STALE_SHADERS {
            assert!(!dir.join("ShaderFixes").join(name).exists(), "{name}");
        }
        assert!(steps.iter().any(|s| s.contains("lava")), "{steps:?}");
        for (name, text) in SKY_SHADERS {
            assert_eq!(read(&dir, &format!("ShaderFixes/{name}")), text);
        }
        assert_eq!(read(&dir, "ShaderFixes/edcas_version.txt"), "1.0.0\n");
        assert!(read(&dir, "ShaderFixes/edcas_params.txt").contains("x500 = 0"));

        let ini = read(&dir, "d3dx.ini");
        assert!(ini.contains("\r\n"), "the release's CRLF line ends stay");
        let ini = ini.replace("\r\n", "\n");
        assert_eq!(get_option(&ini, "System", "params_file").as_deref(), Some("ShaderFixes\\edcas_params.txt"));
        assert_eq!(get_option(&ini, "System", "params_fade").as_deref(), Some("4"));
        assert_eq!(get_option(&ini, "Hunting", "hunting").as_deref(), Some("0"));
        assert_eq!(get_option(&ini, "Rendering", "export_shaders").as_deref(), Some("1"), "the export is on until the shaders are there");
        assert_eq!(ini.matches("include = ShaderFixes\\edcas.ini").count(), 1);
        assert!(ini.contains(";analyse_frame = 1"));

        // Installed, ground tint to come: the exports aren't there.
        assert_eq!(status(&dir, "1.0.0"), Status::Installed { ground_tint: false, star_light: 0 });
        assert_eq!(finish_game_shaders(&dir, Wanted::ALL).unwrap().ground_tint, GroundTint::NeedsLanding);

        // The player lands; the exports are there. The game ran in space before: the open-space
        // sun pass is exported too, the atmosphere's shaders not yet.
        export_ground_shaders(&dir);
        fs::write(exported_path(&dir, SUN_SHADERS[0]), SUN).unwrap();
        let p = finish_game_shaders(&dir, Wanted::ALL).unwrap();
        assert_eq!(p.ground_tint, GroundTint::Done);
        assert_eq!(p.star_light, StarLightShaders { made: 1, failed: vec![] });
        for h in GROUND_SHADERS {
            assert_eq!(read(&dir, &format!("ShaderFixes/{h}-ps.txt")), PATCHED);
        }
        assert_eq!(read(&dir, &format!("ShaderFixes/{}-ps.txt", SUN_SHADERS[0])), sun_patched());
        let ini = read(&dir, "d3dx.ini").replace("\r\n", "\n");
        assert_eq!(get_option(&ini, "Rendering", "export_shaders").as_deref(), Some("1"), "four star light copies are still to come");
        assert_eq!(status(&dir, "1.0.0"), Status::Installed { ground_tint: true, star_light: 1 });
        assert!(star_light_message(&p.star_light, true).contains(&format!("1 of {}", StarLightShaders::TOTAL)));
        // Over a planet with air the rest is exported; EDCAS finishes by itself at its next start.
        export_star_light_shaders(&dir);
        assert!(star_light_pending(&dir));
        let p = finish_game_shaders(&dir, Wanted::ALL).unwrap();
        assert!(p.star_light.done(), "{:?}", p.star_light);
        assert!(!star_light_pending(&dir));
        let ini = read(&dir, "d3dx.ini").replace("\r\n", "\n");
        assert_eq!(get_option(&ini, "Rendering", "export_shaders").as_deref(), Some("0"));
        assert_eq!(status(&dir, "1.0.0"), Status::Installed { ground_tint: true, star_light: StarLightShaders::TOTAL });
        // A copy is made anew from its export (an older client's copy would keep the old patch).
        fs::write(dir.join(FIXES).join(format!("{}-ps.txt", SUN_SHADERS[0])), "old copy").unwrap();
        assert!(finish_game_shaders(&dir, Wanted::ALL).unwrap().star_light.done());
        assert_eq!(read(&dir, &format!("ShaderFixes/{}-ps.txt", SUN_SHADERS[0])), sun_patched());
        // The copies stay when the game's cache is cleared and the user updates.
        fs::remove_dir_all(dir.join("ShaderCache")).unwrap();
        assert_eq!(finish_game_shaders(&dir, Wanted::ALL).unwrap().ground_tint, GroundTint::Done);
        // A different client version has an update to offer.
        assert_eq!(status(&dir, "1.1.0"), Status::UpdateAvailable { installed: Some("1.0.0".into()) });
        let report = install(&dir, &bundle(EDCAS_DLL), "1.1.0", Wanted::ALL, |_| {}).unwrap();
        assert_eq!(report.ground_tint, GroundTint::Done);
        assert_eq!(status(&dir, "1.1.0"), Status::Installed { ground_tint: true, star_light: StarLightShaders::TOTAL });
    }

    #[test]
    fn install_with_the_exports_present_makes_the_tint_at_once() {
        let dir = game("exports");
        export_ground_shaders(&dir);
        let report = install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap();
        assert_eq!(report.ground_tint, GroundTint::Done);
        assert_eq!(status(&dir, "1.0.0"), Status::Installed { ground_tint: true, star_light: 0 });
    }

    #[test]
    fn a_changed_ground_shader_is_reported_not_half_installed() {
        let dir = game("changed");
        export_ground_shaders(&dir);
        fs::write(exported_path(&dir, GROUND_SHADERS[2]), "ps_5_0\nret\n").unwrap();
        let report = install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap();
        let GroundTint::Failed(why) = report.ground_tint else { panic!("{:?}", report.ground_tint) };
        assert!(why.contains(GROUND_SHADERS[2]) && why.contains("game may have changed it"), "{why}");
        assert!(!dir.join(FIXES).join(format!("{}-ps.txt", GROUND_SHADERS[0])).exists());
        assert_eq!(status(&dir, "1.0.0"), Status::Installed { ground_tint: false, star_light: 0 });
    }

    #[test]
    fn install_keeps_other_mods_files_and_the_original_dll() {
        let dir = game("existing");
        fs::write(dir.join("d3d11.dll"), STOCK_DLL).unwrap();
        fs::write(dir.join("d3dx.ini"), "[Hunting]\nhunting = 1\n[Include]\ninclude = EDHM\\edhm.ini\n[Custom]\nmine = 1\n").unwrap();
        fs::create_dir_all(dir.join("ShaderFixes")).unwrap();
        fs::write(dir.join("ShaderFixes/default.hlsl"), "// mine").unwrap();
        fs::write(dir.join("d3dcompiler_47.dll"), b"compiler v1").unwrap();

        install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap();
        assert_eq!(fs::read(dir.join("d3d11.dll")).unwrap(), EDCAS_DLL);
        assert_eq!(fs::read(dir.join("d3d11.dll.official")).unwrap(), STOCK_DLL);
        assert_eq!(read(&dir, "ShaderFixes/default.hlsl"), "// mine", "an existing file isn't overwritten");
        assert_eq!(read(&dir, "d3dcompiler_47.dll"), "compiler v2");
        let ini = read(&dir, "d3dx.ini");
        assert!(!ini.contains("the release's d3dx.ini"), "the existing d3dx.ini is edited, not replaced");
        assert!(ini.contains("[Custom]\nmine = 1") && ini.contains("include = EDHM\\edhm.ini") && ini.contains("include = ShaderFixes\\edcas.ini"), "{ini}");

        // A second run changes nothing and doesn't mistake EDCAS's DLL for the original.
        let before: Vec<String> = ["d3dx.ini", "ShaderFixes/edcas.ini", "ShaderFixes/edcas_params.txt"].iter().map(|f| read(&dir, f)).collect();
        install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap();
        let after: Vec<String> = ["d3dx.ini", "ShaderFixes/edcas.ini", "ShaderFixes/edcas_params.txt"].iter().map(|f| read(&dir, f)).collect();
        assert_eq!(before, after);
        assert_eq!(fs::read(dir.join("d3d11.dll.official")).unwrap(), STOCK_DLL);
        assert_eq!(read(&dir, "d3dx.ini").matches("include = ShaderFixes\\edcas.ini").count(), 1);

        // A newer EDCAS DLL replaces ours and the original stays the original.
        let newer = [EDCAS_DLL, b" v2"].concat();
        install(&dir, &bundle(&newer), "1.1.0", Wanted::ALL, |_| {}).unwrap();
        assert_eq!(fs::read(dir.join("d3d11.dll")).unwrap(), newer);
        assert_eq!(fs::read(dir.join("d3d11.dll.official")).unwrap(), STOCK_DLL);
    }

    #[test]
    fn install_keeps_the_original_dll_switched_off_and_the_players_params() {
        let dir = game("off");
        fs::write(dir.join("d3d11.dll.off"), STOCK_DLL).unwrap();
        fs::create_dir_all(dir.join("ShaderFixes")).unwrap();
        fs::write(dir.join("ShaderFixes/edcas_params.txt"), "x505 = 0.5\n").unwrap();
        install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap();
        assert_eq!(fs::read(dir.join("d3d11.dll.official")).unwrap(), STOCK_DLL);
        assert!(!dir.join("d3d11.dll.off").exists(), "the patched DLL is the active one now");
        assert_eq!(read(&dir, "ShaderFixes/edcas_params.txt"), "x505 = 0.5\n");
    }

    #[test]
    fn install_refuses_what_is_not_elites_folder() {
        let dir = temp("not-elite");
        let err = install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap_err();
        assert!(err.contains(GAME_EXE), "{err}");
        assert!(fs::read_dir(&dir).unwrap().next().is_none(), "nothing was written");
        assert!(install(&dir.join("nope"), &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap_err().contains("doesn't exist"));
        // A zip without the DLL, and something that is no zip.
        let dir = game("bad-zip");
        assert!(install(&dir, &Bundle { zip: b"not a zip".to_vec() }, "1.0.0", Wanted::ALL, |_| {}).unwrap_err().contains("isn't a zip"));
    }

    #[test]
    fn a_zip_path_leading_out_of_the_folder_is_refused() {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("../evil.txt", opts).unwrap();
        zip.write_all(b"x500").unwrap();
        let err = read_zip(&zip.finish().unwrap().into_inner()).unwrap_err();
        assert!(err.contains("unsafe path"), "{err}");
    }

    #[test]
    fn remove_takes_edcas_out_and_puts_the_original_dll_back() {
        let dir = game("remove");
        fs::write(dir.join("d3d11.dll"), STOCK_DLL).unwrap();
        fs::write(dir.join("d3dx.ini"), "[Include]\ninclude = EDHM\\edhm.ini\n").unwrap();
        install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap();
        export_ground_shaders(&dir);
        export_star_light_shaders(&dir);
        finish_game_shaders(&dir, Wanted::ALL).unwrap();
        fs::write(dir.join("ShaderFixes/edcas_params.txt"), "x505 = 0.5\n").unwrap();
        fs::write(dir.join("ShaderFixes/other-ps.txt"), "keep").unwrap();
        for name in STALE_SHADERS {
            fs::write(dir.join("ShaderFixes").join(name), "old").unwrap();
        }

        remove(&dir).unwrap();
        assert_eq!(status(&dir, "1.0.0"), Status::NotInstalled);
        let mut left: Vec<String> = fs::read_dir(dir.join("ShaderFixes")).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, ["default.hlsl", "edcas_params.txt", "other-ps.txt"]);
        let ini = read(&dir, "d3dx.ini");
        assert!(!ini.contains("edcas.ini") && ini.contains("include = EDHM\\edhm.ini"), "{ini}");
        assert_eq!(fs::read(dir.join("d3d11.dll")).unwrap(), STOCK_DLL);
        assert!(!dir.join("d3d11.dll.official").exists());
        assert_eq!(read(&dir, "d3dcompiler_47.dll"), "compiler v2", "the rest of 3DMigoto stays");
        // Removing again is harmless.
        remove(&dir).unwrap();
        assert_eq!(fs::read(dir.join("d3d11.dll")).unwrap(), STOCK_DLL);
    }

    #[test]
    fn remove_without_an_original_deletes_edcas_dll_and_switches_the_export_off() {
        let dir = game("remove-fresh");
        install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap();
        assert_eq!(get_option(&read(&dir, "d3dx.ini").replace("\r\n", "\n"), "Rendering", "export_shaders").as_deref(), Some("1"));
        remove(&dir).unwrap();
        assert!(!dir.join("d3d11.dll").exists());
        assert_eq!(get_option(&read(&dir, "d3dx.ini").replace("\r\n", "\n"), "Rendering", "export_shaders").as_deref(), Some("0"));
        // A d3d11.dll the player put there since isn't ours to delete.
        install(&dir, &bundle(EDCAS_DLL), "1.0.0", Wanted::ALL, |_| {}).unwrap();
        fs::write(dir.join("d3d11.dll"), STOCK_DLL).unwrap();
        let msg = remove(&dir).unwrap();
        assert!(msg.contains("stays"), "{msg}");
        assert_eq!(fs::read(dir.join("d3d11.dll")).unwrap(), STOCK_DLL);
    }

    #[test]
    fn the_export_is_on_only_while_something_switched_on_needs_a_game_shader() {
        let export = |dir: &Path| get_option(&read(dir, "d3dx.ini").replace("\r\n", "\n"), "Rendering", "export_shaders");
        let nothing = Wanted { ground: false, star_light: false };
        let dir = game("export-wanted");
        // Every effect that needs a game shader is off: the game is never asked to export.
        let p = install(&dir, &bundle(EDCAS_DLL), "1.0.0", nothing, |_| {}).unwrap();
        assert_eq!(p.ground_tint, GroundTint::NeedsLanding);
        assert_eq!(export(&dir).as_deref(), Some("0"), "nothing wanted");
        assert!(installed_message(&p, nothing).contains("not asked"), "{}", installed_message(&p, nothing));
        assert!(!export_needed(&dir, nothing));
        // The star light alone, then the ground alone: on for each while its copies are missing.
        let star = Wanted { ground: false, star_light: true };
        let ground = Wanted { ground: true, star_light: false };
        assert!(export_needed(&dir, star) && export_needed(&dir, ground));
        assert!(set_export(&dir, true).unwrap(), "changed");
        assert!(!set_export(&dir, true).unwrap(), "left alone when it says so already");
        assert_eq!(export(&dir).as_deref(), Some("1"));
        // The ground's exports arrive: the ground needs no more, the star light still does.
        export_ground_shaders(&dir);
        assert_eq!(finish_game_shaders(&dir, ground).unwrap().ground_tint, GroundTint::Done);
        assert_eq!(export(&dir).as_deref(), Some("0"), "the ground is done and the star light is off");
        assert!(!export_needed(&dir, ground) && export_needed(&dir, star));
        assert_eq!(finish_game_shaders(&dir, star).unwrap().star_light.made, 0);
        assert_eq!(export(&dir).as_deref(), Some("1"), "the star light switched on: its copies are missing");
        // All there: off for good, whatever is switched on.
        export_star_light_shaders(&dir);
        assert!(finish_game_shaders(&dir, Wanted::ALL).unwrap().star_light.done());
        assert_eq!(export(&dir).as_deref(), Some("0"));
        assert!(!export_needed(&dir, Wanted::ALL));
        // What the settings ask for: the dust needs the ground shaders too (its terrain mask).
        let mut g = crate::settings::Settings::default().game_graphics;
        (g.ground_tint, g.ground_detail, g.planet_dust, g.star_light) = (false, false, false, false);
        assert_eq!(Wanted::of(&g), nothing);
        g.planet_dust = true;
        assert_eq!(Wanted::of(&g), ground);
        (g.planet_dust, g.star_light) = (false, true);
        assert_eq!(Wanted::of(&g), star);
        assert!(star_light_message(&StarLightShaders { made: 3, failed: vec![] }, false).contains("is off"));
    }

    #[test]
    fn an_install_by_install_py_shows_as_an_update() {
        let dir = game("legacy");
        fs::create_dir_all(dir.join("ShaderFixes")).unwrap();
        fs::write(dir.join("ShaderFixes/edcas.ini"), "old").unwrap();
        assert_eq!(status(&dir, "1.0.0"), Status::UpdateAvailable { installed: None });
        fs::write(dir.join("ShaderFixes/edcas_version.txt"), "\n").unwrap();
        assert_eq!(status(&dir, "1.0.0"), Status::UpdateAvailable { installed: None });
    }

    #[test]
    fn the_patched_dll_is_told_from_the_release() {
        assert!(is_edcas_dll(EDCAS_DLL));
        assert!(!is_edcas_dll(STOCK_DLL));
        assert!(!is_edcas_dll(b""));
    }
}
