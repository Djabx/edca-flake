//! Thargoid systems as the journal names them: the war states of the `ThargoidWar`
//! block, the surface sites, conflict zones and codex entries.

/// The war states, as the journal writes them, with what the game's map calls them.
pub const WAR_STATES: &[(&str, &str)] = &[
    ("Thargoid_Stronghold", "Stronghold"),
    ("Thargoid_Controlled", "Controlled"),
    ("Thargoid_Harvest", "Invasion"),
    ("Thargoid_Probing", "Alert"),
    ("Thargoid_Recovery", "Recovery"),
];

/// A war state's name for people ("Thargoid_Harvest" → "Invasion"), or the symbol as it
/// is for a state not listed.
pub fn war_state_name(state: &str) -> &str {
    WAR_STATES.iter().find(|(s, _)| *s == state).map(|(_, name)| *name).unwrap_or(state)
}

/// The settlements that are Thargoid sites, with their names for people.
pub const SITES: &[(&str, &str)] = &[
    ("$Settlement_Unflattened_TGMegaBarnacle", "Barnacle site"),
    ("$Settlement_Unflattened_WreckedUnknown", "Crashed ship"),
];

/// The symbol of a Thargoid settlement name without its index, or `None` for any other.
pub fn site_symbol(settlement: &str) -> Option<&'static str> {
    let symbol = settlement.split(':').next()?.trim_end_matches(';');
    SITES.iter().find(|(s, _)| *s == symbol).map(|(s, _)| *s)
}

/// A Thargoid site's name for people.
pub fn site_name(symbol: &str) -> &str {
    SITES.iter().find(|(s, _)| *s == symbol).map(|(_, name)| *name).unwrap_or(symbol)
}

/// The intensity of a Thargoid conflict zone signal (`$Warzone_TG_VeryHigh:#index=1;` →
/// `VeryHigh`), or `None` for any other signal.
pub fn conflict_zone(signal: &str) -> Option<&str> {
    let rest = signal.split("$Warzone_TG_").nth(1)?;
    let intensity = rest.split([':', ';']).next()?;
    (!intensity.is_empty()).then_some(intensity)
}

/// A conflict zone intensity for people: "VeryHigh" → "Very high".
pub fn intensity_name(intensity: &str) -> &str {
    match intensity {
        "Low" => "Low",
        "Med" => "Medium",
        "High" => "High",
        "VeryHigh" => "Very high",
        other => other,
    }
}

/// The kinds of Thargoid codex entries, in the order lists show them.
pub const CODEX_GROUPS: &[&str] =
    &["Interceptors", "Hunters", "Scouts", "Scavengers", "Spires", "Surface sites", "Barnacles", "Wrecks", "Other"];

/// The kind of a Thargoid codex entry (`$Codex_Ent_Cyclops_Name;` → "Interceptors").
pub fn codex_group(symbol: &str) -> &'static str {
    let has = |part: &str| symbol.contains(part);
    if has("Wrecked_") {
        "Wrecks"
    } else if has("Thargoid_Barnacle") || has("Thargoid_Coral") {
        "Barnacles"
    } else if has("Thargoid_Tower") {
        "Spires"
    } else if has("_TG_") || has("Caustic_Generator") {
        "Surface sites"
    } else if ["Cyclops", "Basilisk", "Medusa", "Hydra", "Orthrus"].iter().any(|s| has(s)) {
        "Interceptors"
    } else if ["Scythe", "Glaive"].iter().any(|s| has(s)) {
        "Hunters"
    } else if has("Scavenger") {
        "Scavengers"
    } else if ["Marauder", "Inciter", "Berserker", "Regenerator", "Scout"].iter().any(|s| has(s)) {
        "Scouts"
    } else {
        "Other"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn war_states_read_like_the_galaxy_map() {
        assert_eq!(war_state_name("Thargoid_Harvest"), "Invasion");
        assert_eq!(war_state_name("Thargoid_Probing"), "Alert");
        assert_eq!(war_state_name("Thargoid_Something"), "Thargoid_Something");
    }

    #[test]
    fn sites_and_conflict_zones_come_from_their_names() {
        assert_eq!(site_symbol("$Settlement_Unflattened_TGMegaBarnacle:#index=1;"), Some(SITES[0].0));
        assert_eq!(site_symbol("$Ancient:#index=1;"), None);
        assert_eq!(conflict_zone("$Warzone_TG_VeryHigh:#index=1;"), Some("VeryHigh"));
        assert_eq!(conflict_zone("INV Paramount $Warzone_TG_Low;"), Some("Low"));
        assert_eq!(conflict_zone("$USS_Type_NonHuman;"), None);
        assert_eq!(intensity_name("Med"), "Medium");
    }

    #[test]
    fn codex_entries_fall_into_groups() {
        assert_eq!(codex_group("$Codex_Ent_Cyclops_Name;"), "Interceptors");
        assert_eq!(codex_group("$Codex_Ent_Wrecked_Interceptor_Name;"), "Wrecks");
        assert_eq!(codex_group("$Codex_Ent_Wrecked_Scout_Name;"), "Wrecks");
        assert_eq!(codex_group("$Codex_Ent_Thargoid_Tower_ExtraHigh_Name;"), "Spires");
        assert_eq!(codex_group("$Codex_Ent_TG_DataScan_Name;"), "Surface sites");
        assert_eq!(codex_group("$Codex_Ent_Thargoid_Coral_Root_Name;"), "Barnacles");
        assert_eq!(codex_group("$Codex_Ent_Scythe_Name;"), "Hunters");
        assert_eq!(codex_group("$Codex_Ent_Scavengers_Name;"), "Scavengers");
        assert_eq!(codex_group("$Codex_Ent_Marauder_Name;"), "Scouts");
        for group in CODEX_GROUPS {
            assert!(!group.is_empty());
        }
    }
}
