//! Guardian sites as the journal names them.
//!
//! A commander flying to a Guardian site gets an `ApproachSettlement` named
//! `$Ancient:#index=1;` for ruins, or `$Ancient_Small_005:#index=1;` and the like for a
//! structure. The structure symbols stand for its layout; commanders know them by the
//! nicknames below.

/// What a Guardian settlement is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SiteKind {
    /// Ruins: obelisks with the Guardian Codex, relic towers.
    Ruins,
    /// A structure: data terminals, pylons and sentinels, and a blueprint.
    Structure,
}

/// The symbol of a Guardian settlement name without its index
/// (`$Ancient_Small_005:#index=1;` → `$Ancient_Small_005`), or `None` for any other name.
pub fn site_symbol(settlement: &str) -> Option<&str> {
    let symbol = settlement.split(':').next()?.trim_end_matches(';');
    (symbol == "$Ancient" || symbol.starts_with("$Ancient_")).then_some(symbol)
}

/// Ruins or a structure, for a symbol from [`site_symbol`].
pub fn site_kind(symbol: &str) -> Option<SiteKind> {
    match symbol {
        "$Ancient" => Some(SiteKind::Ruins),
        s if s.starts_with("$Ancient_") => Some(SiteKind::Structure),
        _ => None,
    }
}

/// The structure layouts by symbol, with the names commanders call them.
pub const STRUCTURES: &[(&str, &str)] = &[
    ("$Ancient_Tiny_001", "Lacrosse"),
    ("$Ancient_Tiny_002", "Crossroads"),
    ("$Ancient_Tiny_003", "Fistbump"),
    ("$Ancient_Small_001", "Hammerbot"),
    ("$Ancient_Small_002", "Bear"),
    ("$Ancient_Small_003", "Bowl"),
    ("$Ancient_Small_005", "Turtle"),
    ("$Ancient_Medium_001", "Robolobster"),
    ("$Ancient_Medium_002", "Squid"),
    ("$Ancient_Medium_003", "Stickyhand"),
];

/// The size of a structure layout: "tiny", "small" or "medium".
pub fn structure_size(symbol: &str) -> Option<&'static str> {
    ["Tiny", "Small", "Medium"]
        .into_iter()
        .find(|size| symbol.starts_with(&format!("$Ancient_{size}_")))
        .map(|size| match size {
            "Tiny" => "tiny",
            "Small" => "small",
            _ => "medium",
        })
}

/// A site's name for people: "Ruins", "Turtle", or "Structure" for a layout not listed.
pub fn site_name(symbol: &str) -> &str {
    match site_kind(symbol) {
        Some(SiteKind::Ruins) => "Ruins",
        Some(SiteKind::Structure) => STRUCTURES
            .iter()
            .find(|(s, _)| *s == symbol)
            .map(|(_, name)| *name)
            .unwrap_or("Structure"),
        None => symbol,
    }
}

/// The Guardian codex entries, with what to call them in a list.
pub const CODEX_ENTRIES: &[(&str, &str)] = &[
    ("$Codex_Ent_Guardian_Data_Logs_Name;", "Obelisks"),
    ("$Codex_Ent_Relic_Tower_Name;", "Relic towers"),
    ("$Codex_Ent_Guardian_Terminal_Name;", "Terminals"),
    ("$Codex_Ent_Guardian_Pylon_Name;", "Pylons"),
    ("$Codex_Ent_Guardian_Sentinel_Name;", "Sentinels"),
    ("$Codex_Ent_Guardian_Beacons_Name;", "Beacon"),
];

/// A Guardian codex entry's name in a list ("Obelisks"), or `None` for any other entry.
pub fn codex_feature(symbol: &str) -> Option<&'static str> {
    CODEX_ENTRIES.iter().find(|(s, _)| *s == symbol).map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settlement_names_become_sites() {
        assert_eq!(site_symbol("$Ancient:#index=2;"), Some("$Ancient"));
        assert_eq!(site_symbol("$Ancient_Small_005:#index=1;"), Some("$Ancient_Small_005"));
        assert_eq!(site_symbol("$Settlement_Unflattened_WreckedUnknown:#index=1;"), None);
        assert_eq!(site_symbol("Ancient Ruins"), None);
        assert_eq!(site_kind("$Ancient"), Some(SiteKind::Ruins));
        assert_eq!(site_kind("$Ancient_Medium_002"), Some(SiteKind::Structure));
    }

    #[test]
    fn structures_have_names_and_sizes() {
        assert_eq!(site_name("$Ancient"), "Ruins");
        assert_eq!(site_name("$Ancient_Small_005"), "Turtle");
        assert_eq!(site_name("$Ancient_Large_009"), "Structure");
        assert_eq!(structure_size("$Ancient_Tiny_003"), Some("tiny"));
        assert_eq!(structure_size("$Ancient_Medium_001"), Some("medium"));
        assert_eq!(structure_size("$Ancient"), None);
        for (symbol, _) in STRUCTURES {
            assert!(structure_size(symbol).is_some(), "{symbol}");
        }
    }

    #[test]
    fn guardian_codex_entries_have_list_names() {
        assert_eq!(codex_feature("$Codex_Ent_Relic_Tower_Name;"), Some("Relic towers"));
        assert_eq!(codex_feature("$Codex_Ent_Aleoids_01_A_Name;"), None);
    }
}
