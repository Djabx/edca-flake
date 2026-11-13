//! Codex symbols as words: galactic regions and the names of codex entries, and the
//! families the codex pages group the entries into ([`articles`]).

pub mod art;
pub mod guardian;
pub mod thargoid;
pub mod articles;

/// The galactic region a codex region symbol (`$Codex_RegionName_18;`) names.
pub fn region_name(symbol: &str) -> Option<&'static str> {
    const REGIONS: [&str; 42] = [
        "Galactic Centre", "Empyrean Straits", "Ryker's Hope", "Odin's Hold", "Norma Arm",
        "Arcadian Stream", "Izanami", "Inner Orion-Perseus Conflux", "Inner Scutum-Centaurus Arm",
        "Norma Expanse", "Trojan Belt", "The Veils", "Newton's Vault", "The Conduit",
        "Outer Orion-Perseus Conflux", "Orion-Cygnus Arm", "Temple", "Inner Orion Spur",
        "Hawking's Gap", "Dryman's Point", "Sagittarius-Carina Arm", "Mare Somnia", "Acheron",
        "Formorian Frontier", "Hieronymus Delta", "Outer Scutum-Centaurus Arm", "Outer Arm",
        "Aquila's Halo", "Errant Marches", "Perseus Arm", "The Formidine Rift", "Vulcan Gate",
        "Elysian Shore", "Sanguineous Rim", "Outer Orion Spur", "Achilles's Altar", "Xibalba",
        "Lyra's Song", "Tenebrae", "The Abyss", "Kepler's Crest", "The Void",
    ];
    let n: usize = symbol.strip_prefix("$Codex_RegionName_")?.strip_suffix(';')?.parse().ok()?;
    REGIONS.get(n.checked_sub(1)?).copied()
}

/// A codex symbol read as words: `$Codex_Ent_IceGeysers_NitrogenGeysers_Name;` →
/// "Ice Geysers · Nitrogen Geysers", `$Codex_Ent_Guardian_Data_Logs_Name;` → "Guardian
/// Data Logs". For entries without a known name; species have theirs in
/// [`crate::exobiology`].
pub fn entry_words(symbol: &str) -> String {
    let core = symbol.trim_start_matches('$').trim_start_matches("Codex_Ent_").trim_end_matches(';').trim_end_matches("_Name");
    // A CamelCase last part is the type of what the parts before it name.
    let parts: Vec<&str> = core.split('_').filter(|p| !p.is_empty()).collect();
    let words = |p: &str| {
        // A capital after a small letter starts a word; capitals in a row are one (TG).
        let mut out = String::new();
        for c in p.chars() {
            if c.is_uppercase() && out.ends_with(|x: char| x.is_lowercase()) {
                out.push(' ');
            }
            out.push(c);
        }
        match out.as_str() {
            "Clds" => "Clouds".to_string(),
            _ => out,
        }
    };
    let camel = |p: &str| p.chars().filter(|c| c.is_uppercase()).count() >= 2;
    match parts.split_last() {
        Some((last, head)) if !head.is_empty() && camel(last) => {
            format!("{} · {}", head.iter().map(|p| words(p)).collect::<Vec<_>>().join(" "), words(last))
        }
        _ => parts.iter().map(|p| words(p)).collect::<Vec<_>>().join(" "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_read_by_number() {
        assert_eq!(region_name("$Codex_RegionName_18;"), Some("Inner Orion Spur"));
        assert_eq!(region_name("$Codex_RegionName_1;"), Some("Galactic Centre"));
        assert_eq!(region_name("$Codex_RegionName_42;"), Some("The Void"));
        assert_eq!(region_name("$Codex_RegionName_43;"), None);
        assert_eq!(region_name("nonsense"), None);
    }

    #[test]
    fn entries_read_as_words() {
        assert_eq!(entry_words("$Codex_Ent_IceGeysers_NitrogenGeysers_Name;"), "Ice Geysers · Nitrogen Geysers");
        assert_eq!(entry_words("$Codex_Ent_Guardian_Data_Logs_Name;"), "Guardian Data Logs");
        assert_eq!(entry_words("$Codex_Ent_Thargoid_Barnacle_Matrix_Name;"), "Thargoid Barnacle Matrix");
        assert_eq!(entry_words("$Codex_Ent_Fumarole_SulphurDioxideMagma_Name;"), "Fumarole · Sulphur Dioxide Magma");
        assert_eq!(entry_words("$Codex_Ent_Gas_Vents_SilicateVapourGeysers_Name;"), "Gas Vents · Silicate Vapour Geysers");
        assert_eq!(entry_words("$Codex_Ent_Gas_Clds_Light_Name;"), "Gas Clouds Light");
        assert_eq!(entry_words("$Codex_Ent_TG_DataScan_Name;"), "TG · Data Scan");
    }
}
