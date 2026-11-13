use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Icon {
    /// The symbol shown, for entries drawn as text (see [`shows_symbol`]); empty for the
    /// ones the explorer paints itself, which take only the colour.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub char: String,
    pub color: String,
    pub enabled: bool,
}

/// The `icons` entries the client reads, besides the ring classes (`eRingClass_*`).
const USED_ICONS: &[&str] = &["bio_signal", "geo_signal", "human_signal", "unknown_signal", "belt_cluster"];

/// Whether an `icons` entry is read at all.
pub fn used_icon(key: &str) -> bool {
    USED_ICONS.contains(&key) || key.starts_with("eRingClass_")
}

/// Whether an `icons` entry's symbol is shown; the others (belt clusters, ring classes, and
/// every star and planet) only lend their colour to a shape the explorer draws.
pub fn shows_symbol(key: &str) -> bool {
    matches!(key, "bio_signal" | "geo_signal" | "human_signal" | "unknown_signal")
}
