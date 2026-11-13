use tracing::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod appearance;
pub mod explorer;
pub mod galaxy_map;
pub mod game_graphics;
pub mod icons;
pub mod journal_reader;
pub use icons::Icon;

use crate::settings::journal_reader::JournalReaderSettings;

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub appearance: appearance::AppearanceSettings,
    pub journal_reader: journal_reader::JournalReaderSettings,
    pub explorer: explorer::ExplorerSettings,
    #[serde(default)]
    pub galaxy_map: galaxy_map::GalaxyMapSettings,
    /// The game's graphics override file, as EDCAS manages it.
    #[serde(default)]
    pub game_graphics: game_graphics::GameGraphicsSettings,
    #[serde(default = "HashMap::default")]
    pub icons: HashMap<String, Icon>,
    #[serde(default = "HashMap::default")]
    pub stars: HashMap<String, Icon>,
    #[serde(default = "HashMap::default")]
    pub planets: HashMap<String, Icon>,
    /// Base URL of the edcas-eddn REST API, e.g. "http://localhost:3000"
    #[serde(default = "default_api_url")]
    pub api_url: String,
    /// Whether to upload journal data to the edcas API. Default: true.
    #[serde(default = "default_true")]
    pub edcas_api_enabled: bool,
    /// Whether to upload journal data to the EDDN network. Default: true.
    #[serde(default = "default_true")]
    pub eddn_enabled: bool,
    /// EDDN upload gateway URL.
    #[serde(default = "default_eddn_url")]
    pub eddn_url: String,
    /// When true, EDDN messages are sent to the test pipeline (`/test` schemaRef suffix).
    #[serde(default)]
    pub eddn_test_mode: bool,
    /// In-app alert/notification toggles.
    #[serde(default)]
    pub alerts: AlertSettings,
}

/// Per-alert and global toggles for the in-app alerting system. Defaults to everything on
/// so the feature works out of the box; serde defaults keep older settings.json files valid.
#[derive(Serialize, Deserialize, Clone)]
pub struct AlertSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Speak alerts aloud via Piper TTS (no-op unless a voice model is installed).
    #[serde(default)]
    pub voice: bool,
    /// Voice playback volume as a percentage (0–100).
    #[serde(default = "default_voice_volume")]
    pub voice_volume: u8,
    #[serde(default = "default_true")]
    pub low_fuel: bool,
    #[serde(default = "default_true")]
    pub overheating: bool,
    #[serde(default = "default_true")]
    pub interdiction: bool,
    #[serde(default = "default_true")]
    pub in_danger: bool,
    #[serde(default = "default_true")]
    pub low_oxygen: bool,
    #[serde(default = "default_true")]
    pub low_health: bool,
    #[serde(default = "default_true")]
    pub non_human_signal: bool,
    #[serde(default = "default_true")]
    pub guardian_signal: bool,
    #[serde(default = "default_true")]
    pub high_bio_signals: bool,
    #[serde(default = "default_true")]
    pub high_geo_signals: bool,
    /// Minimum biological/geological signals on a body for the "high signals" alerts to fire.
    #[serde(default = "default_signal_threshold")]
    pub high_bio_threshold: u8,
    #[serde(default = "default_signal_threshold")]
    pub high_geo_threshold: u8,
    /// An undiscovered Earth-like, water or ammonia world, or a terraformable body.
    #[serde(default = "default_true")]
    pub valuable_body: bool,
    #[serde(default = "default_true")]
    pub carrier_jump_ready: bool,
}

fn default_signal_threshold() -> u8 {
    3
}

fn default_voice_volume() -> u8 {
    50
}

impl Default for AlertSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            voice: false,
            voice_volume: default_voice_volume(),
            low_fuel: true,
            overheating: true,
            interdiction: true,
            in_danger: true,
            low_oxygen: true,
            low_health: true,
            non_human_signal: true,
            guardian_signal: true,
            high_bio_signals: true,
            high_geo_signals: true,
            high_bio_threshold: default_signal_threshold(),
            high_geo_threshold: default_signal_threshold(),
            valuable_body: true,
            carrier_jump_ready: true,
        }
    }
}

impl AlertSettings {
    pub fn to_config(&self) -> crate::alerts::AlertConfig {
        crate::alerts::AlertConfig {
            enabled: self.enabled,
            low_fuel: self.low_fuel,
            overheating: self.overheating,
            interdiction: self.interdiction,
            in_danger: self.in_danger,
            low_oxygen: self.low_oxygen,
            low_health: self.low_health,
            non_human_signal: self.non_human_signal,
            guardian_signal: self.guardian_signal,
            high_bio_signals: self.high_bio_signals,
            high_geo_signals: self.high_geo_signals,
            high_bio_threshold: self.high_bio_threshold,
            high_geo_threshold: self.high_geo_threshold,
            valuable_body: self.valuable_body,
            carrier_jump_ready: self.carrier_jump_ready,
        }
    }
}

fn default_api_url() -> String {
    "https://edcas.de".into()
}

fn default_true() -> bool {
    true
}

fn default_eddn_url() -> String {
    "https://eddn.edcd.io:4430/upload/".into()
}

impl Default for Settings {
    fn default() -> Self {
        Settings::load_native()
    }
}

// ─── Config directory ─────────────────────────────────────────────────────────

/// Returns the OS-appropriate config directory for edcas-client.
///
/// - Windows: `%APPDATA%\edcas-client`
/// - Linux/macOS: `$XDG_CONFIG_HOME/edcas-client` or `~/.config/edcas-client`
pub fn config_dir() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    if let Ok(appdata) = std::env::var("APPDATA") {
        return std::path::PathBuf::from(appdata).join("edcas-client");
    }

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::PathBuf::from(xdg).join("edcas-client");
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home).join(".config").join("edcas-client");
    }
    std::path::PathBuf::from("edcas-client-config")
}

// ─── Log directory ────────────────────────────────────────────────────────────

/// Where the client's log goes: the per-user place each OS keeps logs in, so players find it
/// wherever EDCAS was started from (the working directory was the home folder from a menu,
/// next to the .exe on Windows, and Flatpak's private `/tmp` would have lost it).
///
/// - Windows: `%LOCALAPPDATA%\edcas-client`
/// - Linux/macOS: `$XDG_STATE_HOME/edcas-client` or `~/.local/state/edcas-client`
pub fn log_dir() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        return std::path::PathBuf::from(local).join("edcas-client");
    }

    if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
        return std::path::PathBuf::from(xdg).join("edcas-client");
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home).join(".local").join("state").join("edcas-client");
    }
    std::env::temp_dir().join("edcas-client")
}

static LOG_FILE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Records the log file this run writes (once, at start), for Settings → About.
pub fn set_log_file(path: std::path::PathBuf) {
    let _ = LOG_FILE.set(path);
}

/// The log file this run writes, once logging has started.
pub fn log_file() -> Option<&'static std::path::Path> {
    LOG_FILE.get().map(std::path::PathBuf::as_path)
}

// ─── Native settings ──────────────────────────────────────────────────────────

impl Settings {
    /// The edcas-API base URL to upload to, or `None` when uploading is disabled or unset.
    pub fn edcas_api_url(&self) -> Option<String> {
        let url = self.api_url.trim();
        if self.edcas_api_enabled && !url.is_empty() {
            Some(url.to_string())
        } else {
            None
        }
    }

    /// The EDDN uploader configuration, or `None` when EDDN uploads are disabled.
    pub fn eddn_config(&self) -> Option<crate::eddn::EddnConfig> {
        if self.eddn_enabled && !self.eddn_url.trim().is_empty() {
            // Carriers the commander owns; their `Shipyard.json` lists the player's own
            // parked fleet, which must not be uploaded.
            let owned_carriers = crate::my_carriers::MyCarriersData::load()
                .carriers
                .keys()
                .copied()
                .collect();
            Some(crate::eddn::EddnConfig {
                url: self.eddn_url.trim().to_string(),
                test_mode: self.eddn_test_mode,
                owned_carriers,
            })
        } else {
            None
        }
    }

    fn load_native() -> Self {
        use std::fs::File;
        use std::io::Read;
        use std::path::Path;

        let cfg_dir = config_dir();
        let settings_path = cfg_dir.join("settings.json");

        let mut settings_file = if settings_path.exists() {
            info!("Accessing settings file at {}", settings_path.display());
            match File::open(&settings_path) {
                Ok(file) => file,
                Err(e) => {
                    warn!(
                        "Unable to open settings file {}: {} — using defaults",
                        settings_path.display(),
                        e
                    );
                    return Self::template_defaults();
                }
            }
        } else {
            info!("Settings file not found, creating at {}", settings_path.display());
            create_settings_in(&cfg_dir)
        };

        let mut json_string = String::new();
        if let Err(e) = settings_file.read_to_string(&mut json_string) {
            warn!("Unable to read settings file: {} — using defaults", e);
            return Self::template_defaults();
        }

        let mut settings: Settings = match serde_json::from_str(&json_string) {
            Ok(settings) => settings,
            Err(e) => {
                warn!(
                    "Invalid json in settings file ({}); backing it up and using defaults",
                    e
                );
                back_up_bad_settings(&settings_path);
                return Self::template_defaults();
            }
        };

        settings.drop_unused_icon_fields();
        settings.game_graphics.adopt_defaults();
        if !Path::new(&settings.journal_reader.journal_directory).exists() {
            warn!("journal path {} does not exist", settings.journal_reader.journal_directory);
            settings.journal_reader = JournalReaderSettings::default();
        }
        debug!("Journal logs: {}", &settings.journal_reader.journal_directory);

        settings
    }

    /// Last-resort defaults parsed from the embedded `settings-example.json`.
    ///
    /// Used when the on-disk settings file is missing, unreadable, or malformed.
    /// `Default for Settings` delegates to [`load_native`], so the fallback must
    /// *not* go through `Settings::default()` (that would recurse) — it parses the
    /// compiled-in template instead. The template is embedded at build time and is
    /// therefore trusted; a parse failure here is a build error, not user input.
    pub(crate) fn template_defaults() -> Self {
        let mut settings: Self = serde_json::from_str(SETTINGS_TEMPLATE)
            .expect("embedded settings-example.json must be valid JSON");
        settings.drop_unused_icon_fields();
        settings.game_graphics.adopt_defaults();
        settings
    }

    /// Drops what older settings files hold but nothing reads: `icons` entries no view
    /// uses, and the symbols of entries the explorer draws as shapes. They'd only sit in
    /// the settings page doing nothing; the next save writes the file without them.
    fn drop_unused_icon_fields(&mut self) {
        self.icons.retain(|key, _| icons::used_icon(key));
        for (_, icon) in self.icons.iter_mut().filter(|(k, _)| !icons::shows_symbol(k)) {
            icon.char.clear();
        }
        for icon in self.stars.values_mut().chain(self.planets.values_mut()) {
            icon.char.clear();
        }
    }
}

/// Renames a malformed settings file aside (`settings.json.bak`) and reseeds a fresh
/// template, so the next launch starts from a clean file instead of failing on the
/// same bad input again.
fn back_up_bad_settings(path: &std::path::Path) {
    let backup = path.with_extension("json.bak");
    match std::fs::rename(path, &backup) {
        Ok(()) => {
            warn!("Backed up invalid settings to {}", backup.display());
            if let Err(e) = write_template(path) {
                warn!("Could not reseed settings template: {}", e);
            }
        }
        Err(e) => warn!("Could not back up invalid settings file {}: {}", path.display(), e),
    }
}

/// Creates the config directory and copies a settings template into it.
/// Default settings template, embedded at compile time from the crate's
/// `settings-example.json` — so a fresh install seeds config with no external file
/// (no cwd dependency, no packaged template required).
const SETTINGS_TEMPLATE: &str = include_str!("../settings-example.json");
/// An admin-installed template overrides the embedded one if present.
const SYSTEM_TEMPLATE: &str = "/etc/edcas-client/settings-example.json";

/// Writes the default template (system override if present, else the embedded one).
fn write_template(dest: &std::path::Path) -> std::io::Result<()> {
    if std::path::Path::new(SYSTEM_TEMPLATE).exists() {
        std::fs::copy(SYSTEM_TEMPLATE, dest).map(|_| ())
    } else {
        std::fs::write(dest, SETTINGS_TEMPLATE)
    }
}

/// Falls back to `settings.json` in the current directory if the dir can't be created.
fn create_settings_in(cfg_dir: &std::path::Path) -> std::fs::File {
    use std::fs::File;

    let dest = cfg_dir.join("settings.json");

    if std::fs::create_dir_all(cfg_dir).is_ok() {
        if let Err(e) = write_template(&dest) {
            warn!("Could not write settings template: {}", e);
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o644));
            }
            info!("Created settings file at {}", dest.display());
        }
        if dest.exists() {
            return File::open(&dest).expect("Could not open new settings file");
        }
    } else {
        warn!("Could not create config dir {}", cfg_dir.display());
    }

    // Last resort: use settings.json in the current directory.
    if !std::path::Path::new("settings.json").exists() {
        write_template(std::path::Path::new("settings.json"))
            .expect("Could not create settings.json");
    }
    info!("Falling back to settings.json in current directory");
    File::open("settings.json").expect("Could not open settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_icon_settings_lose_what_nothing_reads() {
        let mut settings = Settings::template_defaults();
        let icon = |ch: &str| Icon { char: ch.into(), color: "white".into(), enabled: true };
        settings.icons.insert("landable".into(), icon("L"));
        settings.icons.insert("belt_cluster".into(), icon("◆"));
        settings.icons.insert("bio_signal".into(), icon("🌿"));
        settings.stars.insert("K".into(), icon("✺"));
        settings.drop_unused_icon_fields();
        assert!(!settings.icons.contains_key("landable"));
        assert_eq!(settings.icons["belt_cluster"].char, "");
        assert_eq!(settings.icons["bio_signal"].char, "🌿");
        assert_eq!(settings.stars["K"].char, "");
        // A file without symbols loads, and saves without them.
        let json = serde_json::to_string(&settings.stars["K"]).unwrap();
        assert_eq!(json, r#"{"color":"white","enabled":true}"#);
        let back: Icon = serde_json::from_str(&json).unwrap();
        assert_eq!(back.char, "");
    }
}
