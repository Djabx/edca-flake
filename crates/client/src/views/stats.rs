mod gui;

use std::sync::Arc;

use crate::cartography::UnsoldData;

/// Session statistics plus estimated exploration / exobiology earnings.
pub struct StatsView {
    /// Exploration data not yet sold, across sessions (see [`crate::cartography`]).
    pub unsold: Arc<UnsoldData>,
}

impl StatsView {
    pub fn new() -> Self {
        Self { unsold: Arc::default() }
    }
}

use edcas_common::format::credits as format_credits;

fn group_thousands(n: u64) -> String {
    edcas_common::format::group_thousands(n as i64)
}
