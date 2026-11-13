use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pins {
    #[serde(default)]
    pub stations: HashSet<i64>,
    #[serde(default)]
    pub carriers: HashSet<i64>,
    #[serde(default)]
    pub factions: HashSet<String>,
    #[serde(default)]
    pub constructions: HashSet<i64>,
    #[serde(default)]
    pub my_carriers: HashSet<i64>,
}

impl Pins {
    pub fn load() -> Self {
        {
            let path = Self::native_path();
            // Try the canonical file first, then the leftover .tmp (written but not yet renamed).
            let loaded = std::fs::read_to_string(&path)
                .ok()
                .and_then(|data| serde_json::from_str(&data).ok());
            if let Some(pins) = loaded {
                return pins;
            }
            // Fallback: if pins.json is missing or corrupt, try the temp file.
            let tmp = path.with_extension("json.tmp");
            std::fs::read_to_string(&tmp)
                .ok()
                .and_then(|data| serde_json::from_str(&data).ok())
                .unwrap_or_default()
        }
    }

    pub fn save(&self) {
        {
            let path = Self::native_path();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
                if let Ok(data) = serde_json::to_string_pretty(self) {
                    let tmp = parent.join("pins.json.tmp");
                    if std::fs::write(&tmp, &data).is_ok() {
                        let _ = std::fs::rename(&tmp, &path);
                    }
                }
            }
        }
    }

    fn native_path() -> std::path::PathBuf {
        crate::settings::config_dir().join("pins.json")
    }
}
