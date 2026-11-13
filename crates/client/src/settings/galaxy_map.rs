use serde::{Deserialize, Serialize};

/// Most stars per layer the server hands out (its cap).
pub const MAX_STARS: u32 = 100_000;
pub const MIN_STARS: u32 = 1_000;

/// How much the Galaxy Map fetches and draws. More stars fill the view better but take
/// longer to load (about 110 bytes per system before compression).
#[derive(Serialize, Deserialize, Clone)]
pub struct GalaxyMapSettings {
    /// Systems drawn singly (with names) around the camera's focus: the nearest this many.
    #[serde(default = "default_stars")]
    pub nearby_stars: u32,
    /// Systems sampled evenly from the wider area around the focus.
    #[serde(default = "default_stars")]
    pub sampled_stars: u32,
}

fn default_stars() -> u32 {
    20_000
}

impl Default for GalaxyMapSettings {
    fn default() -> Self {
        Self { nearby_stars: default_stars(), sampled_stars: default_stars() }
    }
}
