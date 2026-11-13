//! The diagnostics zip of Settings → Game Graphics → Shader Mods → Save Diagnostics.
//!
//! A tester whose shader mods misbehave sends one zip instead of being asked for a file at a
//! time: the client's log, the game folder's `d3dx.ini`, 3DMigoto's own log, what `ShaderFixes`
//! holds (with hashes, so a stale or half-written install shows), `Status.json`, Elite's
//! graphics option files, the settings (secrets redacted) and a `system.txt` that says where
//! everything was looked for. Every part is best effort: a missing file becomes a line in
//! `system.txt`, never an error. Journal files stay out, they hold the commander's name and
//! travels.

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracing::{info, warn};

use crate::ground_tint::{self, GroundTint};
use crate::journal_reader::JournalData;
use crate::settings::{self, Settings};
use crate::shader_mods;

/// How much of each client log goes in (the end of it is where the trouble is).
const LOG_TAIL: u64 = 2 * 1024 * 1024;
/// How much of 3DMigoto's `d3d11_log.txt` goes in.
const D3D11_LOG_TAIL: u64 = 1024 * 1024;
/// The most `ShaderFixes` file content that is hashed for the listing: the ground textures
/// are a few MB each, a folder gone wrong with huge files should not hold the button up.
const HASH_BUDGET: u64 = 200 * 1024 * 1024;
/// Elite's option files are small XML; anything bigger is not one.
const OPTION_FILE_MAX: u64 = 256 * 1024;

/// What only the running app knows (the journal's state), read on the GUI thread and handed
/// to the thread that writes the zip.
#[derive(Clone, Debug, Default)]
pub struct Live {
    pub body_name: String,
    pub cloud_status: String,
}

impl Live {
    pub fn of(settings: &Settings, journal: &JournalData) -> Self {
        Live { body_name: journal.body_name_here().to_string(), cloud_status: GroundTint::cloud_status(settings, journal) }
    }
}

/// Everything [`write`] looks at, gathered up front so a test can point it at a temp dir.
pub struct Inputs {
    pub version: String,
    pub build: String,
    pub now: DateTime<Utc>,
    pub journal_dir_setting: String,
    pub game_folder_setting: String,
    /// What `ground_tint::game_dir` resolved the two settings to.
    pub game_dir: Option<PathBuf>,
    /// The settings as JSON, before redaction.
    pub settings_json: Value,
    pub live: Live,
    pub log_current: Option<PathBuf>,
    pub log_previous: PathBuf,
    /// Folders that may hold Elite's `Options/Graphics` files, most likely first.
    pub options_dirs: Vec<PathBuf>,
    /// Where the zip goes.
    pub out_dir: PathBuf,
}

impl Inputs {
    pub fn of(settings: &Settings, live: Live) -> Self {
        let journal = settings.journal_reader.journal_directory.trim().to_string();
        let log_dir = settings::log_dir();
        Inputs {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build: crate::build_info::BUILD.to_string(),
            now: Utc::now(),
            journal_dir_setting: journal.clone(),
            game_folder_setting: settings.game_graphics.game_folder.clone(),
            game_dir: ground_tint::game_dir(settings),
            settings_json: serde_json::to_value(settings).unwrap_or(Value::Null),
            live,
            log_current: settings::log_file().map(Path::to_path_buf),
            log_previous: log_dir.join("log.previous.log"),
            options_dirs: options_dirs(&journal),
            out_dir: log_dir,
        }
    }
}

/// Where Elite keeps its graphics option files. Under Proton the journal folder is
/// `<prefix>/drive_c/users/<user>/Saved Games/Frontier Developments/Elite Dangerous` and the
/// options sit in the same user's `AppData/Local`; on Windows that is `%LOCALAPPDATA%`.
pub fn options_dirs(journal_dir: &str) -> Vec<PathBuf> {
    let tail = |base: PathBuf| base.join("Frontier Developments").join("Elite Dangerous").join("Options").join("Graphics");
    let mut dirs = Vec::new();
    if let Some(user) = Path::new(journal_dir).ancestors().find(|p| p.file_name().is_some_and(|n| n == "Saved Games")).and_then(Path::parent) {
        dirs.push(tail(user.join("AppData").join("Local")));
    }
    if cfg!(windows) {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let dir = tail(PathBuf::from(local));
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    dirs
}

/// Writes the zip for the app's current state (see [`write`]) on a thread of its own, as
/// hashing the textures takes a moment; the end comes through the receiver.
pub fn start(settings: &Settings, live: Live) -> mpsc::Receiver<Result<PathBuf, String>> {
    let (tx, rx) = mpsc::channel();
    let inputs = Inputs::of(settings, live);
    thread::spawn(move || {
        let result = write(&inputs);
        match &result {
            Ok(path) => info!("Diagnostics saved to {}", path.display()),
            Err(e) => warn!("Diagnostics failed: {e}"),
        }
        let _ = tx.send(result);
        crate::wake::wake();
    });
    rx
}

/// Writes `edcas-diagnostics-<version>-<YYYYMMDD-HHMMSS>.zip` into `inputs.out_dir`.
pub fn write(inputs: &Inputs) -> Result<PathBuf, String> {
    let mut notes: Vec<String> = Vec::new();
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();

    entries.push(("settings.json".into(), settings_text(&inputs.settings_json).into_bytes()));

    for (entry, path, max) in [
        ("client-log.log", inputs.log_current.clone().unwrap_or_else(|| inputs.log_previous.with_file_name("log.log")), LOG_TAIL),
        ("client-log.previous.log", inputs.log_previous.clone(), LOG_TAIL),
    ] {
        add_tail(&mut entries, &mut notes, entry, &path, max);
    }

    let mut ini_text = None;
    match &inputs.game_dir {
        Some(dir) if dir.is_dir() => {
            add_tail(&mut entries, &mut notes, "d3dx.ini", &dir.join("d3dx.ini"), u64::MAX);
            ini_text = entries.last().filter(|(n, _)| n == "d3dx.ini").map(|(_, b)| String::from_utf8_lossy(b).replace("\r\n", "\n"));
            add_tail(&mut entries, &mut notes, "d3d11_log.txt", &dir.join("d3d11_log.txt"), D3D11_LOG_TAIL);
            let fixes = dir.join(shader_mods::FIXES);
            for name in [shader_mods::INI_FILE, shader_mods::PARAMS_FILE, shader_mods::VERSION_FILE] {
                add_tail(&mut entries, &mut notes, &format!("{}/{name}", shader_mods::FIXES), &fixes.join(name), u64::MAX);
            }
            entries.push(("ShaderFixes-listing.txt".into(), listing(&fixes, &mut notes).into_bytes()));
        }
        Some(dir) => notes.push(format!("game folder {} does not exist: no game files in this zip", dir.display())),
        None => notes.push("no game folder known (not set, and not found from the journal folder): no game files in this zip".into()),
    }

    let journal = Path::new(&inputs.journal_dir_setting);
    if inputs.journal_dir_setting.is_empty() {
        notes.push("no journal folder set: no Status.json".into());
    } else {
        add_tail(&mut entries, &mut notes, "Status.json", &journal.join("Status.json"), u64::MAX);
    }

    let mut found_options = false;
    for dir in &inputs.options_dirs {
        found_options |= add_options(&mut entries, &mut notes, dir);
    }
    if !found_options {
        notes.push(format!(
            "no Elite graphics option files found (looked in: {})",
            if inputs.options_dirs.is_empty() { "nowhere, the journal folder is not under 'Saved Games' and this is not Windows".into() } else { join_paths(&inputs.options_dirs) }
        ));
    }

    let system = system_text(inputs, ini_text.as_deref(), &notes);
    let name = format!("edcas-diagnostics-{}-{}.zip", inputs.version, inputs.now.format("%Y%m%d-%H%M%S"));
    fs::create_dir_all(&inputs.out_dir).map_err(|e| format!("can't create {}: {e}", inputs.out_dir.display()))?;
    let path = inputs.out_dir.join(name);
    let file = File::create(&path).map_err(|e| format!("can't create {}: {e}", path.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let all = std::iter::once(("system.txt".to_string(), system.into_bytes())).chain(entries);
    for (entry, data) in all {
        zip.start_file(entry.as_str(), opts).map_err(|e| format!("can't write {entry} into the zip: {e}"))?;
        zip.write_all(&data).map_err(|e| format!("can't write {entry} into the zip: {e}"))?;
    }
    zip.finish().map_err(|e| format!("can't finish {}: {e}", path.display()))?;
    Ok(path)
}

fn join_paths(paths: &[PathBuf]) -> String {
    paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("; ")
}

// ------------------------------------------------------------------ files

/// The last `max` bytes of `path` as the entry `name`; a missing or unreadable file is a note.
fn add_tail(entries: &mut Vec<(String, Vec<u8>)>, notes: &mut Vec<String>, name: &str, path: &Path, max: u64) {
    match read_tail(path, max) {
        Ok(bytes) => entries.push((name.to_string(), bytes)),
        Err(e) => notes.push(format!("{name}: not included, {} ({e})", path.display())),
    }
}

fn read_tail(path: &Path, max: u64) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    let mut out = Vec::new();
    if len > max {
        file.seek(SeekFrom::Start(len - max))?;
        out.extend_from_slice(format!("[EDCAS diagnostics: only the last {max} of {len} bytes of this file are included]\n").as_bytes());
    }
    file.read_to_end(&mut out)?;
    Ok(out)
}

/// Elite's small text option files (`*.xml`, `*.fxcfg`) of `dir` as `Options/Graphics/<name>`;
/// whether the folder had any.
fn add_options(entries: &mut Vec<(String, Vec<u8>)>, notes: &mut Vec<String>, dir: &Path) -> bool {
    let Ok(read) = fs::read_dir(dir) else { return false };
    let mut files: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("xml") || e.eq_ignore_ascii_case("fxcfg"))
        })
        .collect();
    files.sort();
    let mut any = false;
    for path in files {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let entry = format!("Options/Graphics/{name}");
        if entries.iter().any(|(n, _)| *n == entry) {
            continue;
        }
        match fs::metadata(&path) {
            Ok(m) if m.len() > OPTION_FILE_MAX => notes.push(format!("{entry}: left out, {} bytes is too big for an option file", m.len())),
            _ => match fs::read(&path) {
                Ok(bytes) if std::str::from_utf8(&bytes).is_ok() => {
                    entries.push((entry, bytes));
                    any = true;
                }
                Ok(_) => notes.push(format!("{entry}: left out, not a text file")),
                Err(e) => notes.push(format!("{entry}: not included ({e})")),
            },
        }
    }
    any
}

/// One line per file in `ShaderFixes`: name, size, modification time (UTC) and SHA-256.
fn listing(fixes: &Path, notes: &mut Vec<String>) -> String {
    let mut out = String::from("name\tsize\tmodified (UTC)\tsha256\n");
    let read = match fs::read_dir(fixes) {
        Ok(read) => read,
        Err(e) => {
            notes.push(format!("{}: can't list ({e})", fixes.display()));
            return format!("{} can't be listed: {e}\n", fixes.display());
        }
    };
    let mut files: Vec<_> = read.flatten().collect();
    files.sort_by_key(std::fs::DirEntry::file_name);
    let mut budget = HASH_BUDGET;
    for entry in files {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(meta) = entry.metadata() else {
            let _ = writeln!(out, "{name}\t?\t?\t?");
            continue;
        };
        if meta.is_dir() {
            let _ = writeln!(out, "{name}/\t(folder)");
            continue;
        }
        let modified = meta.modified().map_or("?".to_string(), |t| DateTime::<Utc>::from(t).format("%Y-%m-%d %H:%M:%S").to_string());
        let hash = if meta.len() > budget {
            "(not hashed: over the 200 MB limit)".to_string()
        } else {
            budget -= meta.len();
            hash_file(&entry.path()).unwrap_or_else(|e| format!("(unreadable: {e})"))
        };
        let _ = writeln!(out, "{name}\t{}\t{modified}\t{hash}", meta.len());
    }
    out
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

// ------------------------------------------------------------------ text

/// Field names (lower case, part of the name) whose values never leave the machine.
const SECRET_NAMES: [&str; 8] = ["token", "key", "password", "passwd", "secret", "auth", "credential", "cookie"];

/// The settings as pretty JSON with secrets redacted: every field named like a token, key,
/// password or the like, and the login part of any URL. The settings hold none today
/// (2026-11-08); this is for the first one added.
fn settings_text(settings: &Value) -> String {
    let mut value = settings.clone();
    redact(&mut value);
    serde_json::to_string_pretty(&value).unwrap_or_else(|e| format!("settings could not be written: {e}"))
}

fn redact(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (name, v) in map.iter_mut() {
                let lower = name.to_ascii_lowercase();
                if !v.is_null() && SECRET_NAMES.iter().any(|s| lower.contains(s)) {
                    *v = Value::String("<redacted>".into());
                } else {
                    redact(v);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact),
        Value::String(s) => *s = strip_url_login(s),
        _ => {}
    }
}

/// `https://user:pass@host/x` → `https://<redacted>@host/x`; other text as it is.
fn strip_url_login(s: &str) -> String {
    let Some((scheme, rest)) = s.split_once("://") else { return s.to_string() };
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    match rest[..authority_end].rfind('@') {
        Some(at) if !scheme.contains(char::is_whitespace) => format!("{scheme}://<redacted>@{}", &rest[at + 1..]),
        _ => s.to_string(),
    }
}

/// The values of `d3dx.ini` that decide whether the mods load and read their parameters.
fn ini_summary(ini: &str) -> String {
    let get = |section: &str, key: &str| shader_mods::get_option(ini, section, key).unwrap_or_else(|| "(not set)".to_string());
    // 3DMigoto takes includes from any [Include...] section, so list every one of them.
    let mut includes = Vec::new();
    let mut in_include = false;
    for line in ini.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_include = t.len() >= 8 && t.is_char_boundary(8) && t[..8].eq_ignore_ascii_case("[include");
        } else if in_include && t.len() > 7 && t.is_char_boundary(7) && t[..7].eq_ignore_ascii_case("include") {
            if let Some((_, v)) = t.split_once('=') {
                includes.push(v.trim().to_string());
            }
        }
    }
    let includes = if includes.is_empty() { "(none)".to_string() } else { includes.join(", ") };
    format!(
        "  [System] params_file = {}\n  [System] params_fade = {}\n  [Include] include = {includes}\n  [Hunting] hunting = {}\n  [Logging] calls = {}\n",
        get("System", "params_file"),
        get("System", "params_fade"),
        get("Hunting", "hunting"),
        get("Logging", "calls"),
    )
}

fn system_text(inputs: &Inputs, ini: Option<&str>, notes: &[String]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "EDCAS diagnostics");
    let _ = writeln!(s, "client version: {}", inputs.version);
    let _ = writeln!(s, "build: {}", inputs.build);
    let _ = writeln!(s, "os / arch: {} / {}", std::env::consts::OS, std::env::consts::ARCH);
    let _ = writeln!(s, "time (UTC): {}", inputs.now.format("%Y-%m-%d %H:%M:%S"));
    let _ = writeln!(s);
    let journal = &inputs.journal_dir_setting;
    let _ = writeln!(
        s,
        "journal folder setting: {} ({})",
        if journal.is_empty() { "(empty)" } else { journal },
        if Path::new(journal).is_dir() { "exists" } else { "does not exist" }
    );
    let game = inputs.game_folder_setting.trim();
    let _ = writeln!(s, "game folder setting: {}", if game.is_empty() { "(empty, found from the journal folder)" } else { game });
    match &inputs.game_dir {
        Some(dir) => {
            let _ = writeln!(s, "game folder used: {} ({})", dir.display(), if dir.is_dir() { "exists" } else { "does not exist" });
            let _ = writeln!(s, "shader mods here: {:?} (this client is {})", shader_mods::status(dir, shader_mods::VERSION), shader_mods::VERSION);
            let _ = writeln!(s, "3DMigoto here: {}", shader_mods::dll_report(dir));
            let damage = shader_mods::damage(dir);
            let _ = writeln!(s, "another installer's changes: {}", if damage.is_empty() { "none".to_string() } else { damage.join("; ") });
        }
        None => {
            let _ = writeln!(s, "game folder used: none found");
        }
    }
    match ini {
        Some(ini) => {
            let _ = writeln!(s, "d3dx.ini:\n{}", ini_summary(ini));
        }
        None => {
            let _ = writeln!(s, "d3dx.ini: not read");
        }
    }
    let _ = writeln!(s, "body here: {}", if inputs.live.body_name.is_empty() { "(none)" } else { &inputs.live.body_name });
    let _ = writeln!(s, "cloud status: {}", if inputs.live.cloud_status.is_empty() { "(none)" } else { &inputs.live.cloud_status });
    let _ = writeln!(s, "log folder: {}", inputs.out_dir.display());
    let _ = writeln!(s);
    if notes.is_empty() {
        let _ = writeln!(s, "Everything looked for was found.");
    } else {
        let _ = writeln!(s, "Not in this zip:");
        for n in notes {
            let _ = writeln!(s, "  {n}");
        }
    }
    s
}

/// Logs what a reader of the log needs first, once at startup: which client this is, where
/// the journal and the game folder are and what state the shader mods stand in.
pub fn log_startup(settings: &Settings) {
    info!(
        "EDCAS client {} (build {}) on {} / {}",
        env!("CARGO_PKG_VERSION"),
        crate::build_info::BUILD,
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let journal = settings.journal_reader.journal_directory.trim();
    if journal.is_empty() {
        warn!("Journal folder: not set");
    } else if Path::new(journal).is_dir() {
        info!("Journal folder: {journal} (exists)");
    } else {
        warn!("Journal folder: {journal} does not exist");
    }
    match ground_tint::game_dir(settings) {
        None => info!("Game folder: not set and not found from the journal folder, so no shader mods can be managed"),
        Some(dir) if !dir.is_dir() => warn!("Game folder: {} does not exist", dir.display()),
        Some(dir) => {
            let how = if settings.game_graphics.game_folder.trim().is_empty() { "found from the journal folder" } else { "set in the settings" };
            info!("Game folder: {} ({how}); shader mods: {}", dir.display(), status_words(&shader_mods::status(&dir, shader_mods::VERSION)));
            info!("3DMigoto: {}", shader_mods::dll_report(&dir));
            let damage = shader_mods::damage(&dir);
            if !damage.is_empty() {
                warn!("Shader mods: another installer changed the game folder ({}); EDCAS repairs it when the game is closed (now, or at the next journal Shutdown)", damage.join("; "));
            }
        }
    }
}

fn status_words(status: &shader_mods::Status) -> String {
    use shader_mods::Status;
    match status {
        Status::NotInstalled => "not installed".to_string(),
        Status::Installed { ground_tint, star_light } => format!(
            "installed, version {} (same as this client); ground tint {}; star light {star_light} of {} game shaders copied",
            shader_mods::VERSION,
            if *ground_tint { "ready" } else { "waiting for a landing" },
            shader_mods::StarLightShaders::TOTAL
        ),
        Status::UpdateAvailable { installed } => format!(
            "installed version {}, this client is {}: an update is available",
            installed.as_deref().unwrap_or("unknown (by hand or from before versions were recorded)"),
            shader_mods::VERSION
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("edcas-diagnostics-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entry_names(zip: &Path) -> Vec<String> {
        let mut archive = zip::ZipArchive::new(File::open(zip).unwrap()).unwrap();
        (0..archive.len()).map(|i| archive.by_index(i).unwrap().name().to_string()).collect()
    }

    fn entry_text(zip: &Path, name: &str) -> String {
        let mut archive = zip::ZipArchive::new(File::open(zip).unwrap()).unwrap();
        let mut text = String::new();
        archive.by_name(name).unwrap().read_to_string(&mut text).unwrap();
        text
    }

    #[test]
    fn zip_holds_the_files_and_redacts_secrets() {
        let root = temp("zip");
        let game = root.join("game");
        let fixes = game.join("ShaderFixes");
        fs::create_dir_all(&fixes).unwrap();
        fs::write(game.join("d3dx.ini"), "[System]\r\nparams_file = ShaderFixes\\edcas_params.txt\r\nparams_fade = 4\r\n[Include]\r\ninclude = ShaderFixes\\edcas.ini\r\n[Hunting]\r\nhunting = 0\r\n").unwrap();
        fs::write(game.join("d3d11_log.txt"), "3Dmigoto loaded").unwrap();
        fs::write(fixes.join("edcas.ini"), "; ini").unwrap();
        fs::write(fixes.join("edcas_params.txt"), "x500 = 0").unwrap();
        fs::write(fixes.join("edcas_version.txt"), "0.0.1\n").unwrap();
        fs::write(fixes.join("edcas_ground.dds"), vec![7u8; 4096]).unwrap();

        // A Proton-shaped journal folder with its options two levels away.
        let user = root.join("pfx/drive_c/users/steamuser");
        let journal = user.join("Saved Games/Frontier Developments/Elite Dangerous");
        let options = user.join("AppData/Local/Frontier Developments/Elite Dangerous/Options/Graphics");
        fs::create_dir_all(&journal).unwrap();
        fs::create_dir_all(&options).unwrap();
        fs::write(journal.join("Status.json"), "{\"event\":\"Status\"}").unwrap();
        fs::write(journal.join("Journal.2026-11-08T000000.01.log"), "commander secrets").unwrap();
        fs::write(options.join("GraphicsConfigurationOverride.xml"), "<x/>").unwrap();
        fs::write(options.join("Custom.4.3.fxcfg"), "<fx/>").unwrap();
        fs::write(options.join("binary.xml"), [0u8, 159, 146, 150]).unwrap();

        let logs = root.join("logs");
        fs::create_dir_all(&logs).unwrap();
        fs::write(logs.join("log.log"), "current log").unwrap();
        // Only the last LOG_TAIL bytes of a long log go in.
        fs::write(logs.join("log.previous.log"), vec![b'a'; (LOG_TAIL as usize) + 10]).unwrap();

        let mut settings = serde_json::to_value(Settings::template_defaults()).unwrap();
        settings["api_token"] = Value::String("hunter2".into());
        settings["nested"] = serde_json::json!({ "Password": "hunter2", "url": "https://me:hunter2@example.com/x" });

        let journal_text = journal.to_string_lossy().into_owned();
        let inputs = Inputs {
            version: "9.9.9".into(),
            build: "dev".into(),
            now: DateTime::parse_from_rfc3339("2026-11-08T12:34:56Z").unwrap().into(),
            journal_dir_setting: journal_text.clone(),
            game_folder_setting: game.to_string_lossy().into_owned(),
            game_dir: Some(game.clone()),
            settings_json: settings,
            live: Live { body_name: "Test 1 a".into(), cloud_status: "Clouds: water ice".into() },
            log_current: Some(logs.join("log.log")),
            log_previous: logs.join("log.previous.log"),
            options_dirs: options_dirs(&journal_text),
            out_dir: root.join("out"),
        };
        assert_eq!(inputs.options_dirs, vec![options]);

        let zip = write(&inputs).unwrap();
        assert_eq!(zip.file_name().unwrap(), "edcas-diagnostics-9.9.9-20261108-123456.zip");
        let names = entry_names(&zip);
        for wanted in [
            "system.txt",
            "settings.json",
            "client-log.log",
            "client-log.previous.log",
            "d3dx.ini",
            "d3d11_log.txt",
            "ShaderFixes/edcas.ini",
            "ShaderFixes/edcas_params.txt",
            "ShaderFixes/edcas_version.txt",
            "ShaderFixes-listing.txt",
            "Status.json",
            "Options/Graphics/GraphicsConfigurationOverride.xml",
            "Options/Graphics/Custom.4.3.fxcfg",
        ] {
            assert!(names.iter().any(|n| n == wanted), "{wanted} missing from {names:?}");
        }
        // No journal files, no binary option files.
        assert!(!names.iter().any(|n| n.contains("Journal.") || n.contains("binary.xml")), "{names:?}");

        let settings_json = entry_text(&zip, "settings.json");
        assert!(!settings_json.contains("hunter2"), "{settings_json}");
        assert!(settings_json.contains("<redacted>"));

        let previous = entry_text(&zip, "client-log.previous.log");
        assert!(previous.starts_with("[EDCAS diagnostics: only the last"));
        assert!(previous.len() < (LOG_TAIL as usize) + 200);

        let listing = entry_text(&zip, "ShaderFixes-listing.txt");
        assert!(listing.contains("edcas_ground.dds\t4096\t"), "{listing}");
        let want: String = Sha256::digest(vec![7u8; 4096]).iter().map(|b| format!("{b:02x}")).collect();
        assert!(listing.contains(&want), "{listing}");

        let system = entry_text(&zip, "system.txt");
        for line in [
            "client version: 9.9.9",
            "[System] params_file = ShaderFixes\\edcas_params.txt",
            "[Include] include = ShaderFixes\\edcas.ini",
            "[Hunting] hunting = 0",
            "[Logging] calls = (not set)",
            "body here: Test 1 a",
            "cloud status: Clouds: water ice",
            "Options/Graphics/binary.xml: left out, not a text file",
        ] {
            assert!(system.contains(line), "{line:?} missing from\n{system}");
        }
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_missing_game_folder_is_a_note_not_an_error() {
        let root = temp("missing");
        let inputs = Inputs {
            version: "1.0.0".into(),
            build: "dev".into(),
            now: Utc::now(),
            journal_dir_setting: String::new(),
            game_folder_setting: String::new(),
            game_dir: None,
            settings_json: Value::Null,
            live: Live::default(),
            log_current: None,
            log_previous: root.join("nope.log"),
            options_dirs: Vec::new(),
            out_dir: root.clone(),
        };
        let zip = write(&inputs).unwrap();
        let system = entry_text(&zip, "system.txt");
        assert!(system.contains("no game folder known"), "{system}");
        assert!(system.contains("no journal folder set"), "{system}");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn urls_lose_their_login() {
        assert_eq!(strip_url_login("https://u:p@host.de/x"), "https://<redacted>@host.de/x");
        assert_eq!(strip_url_login("https://host.de/a@b"), "https://host.de/a@b");
        assert_eq!(strip_url_login("plain"), "plain");
    }
}
