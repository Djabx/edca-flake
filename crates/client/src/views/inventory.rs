mod gui;

use crate::event_shim::{KeyCode, KeyEvent};

use crate::views::ViewEvent;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Materials,
    Cargo,
    Backpack,
    ShipLocker,
}

impl Tab {
    fn next(self) -> Option<Self> {
        match self {
            Self::Materials => Some(Self::Cargo),
            Self::Cargo => Some(Self::Backpack),
            Self::Backpack => Some(Self::ShipLocker),
            Self::ShipLocker => None,
        }
    }
    fn prev(self) -> Option<Self> {
        match self {
            Self::Materials => None,
            Self::Cargo => Some(Self::Materials),
            Self::Backpack => Some(Self::Cargo),
            Self::ShipLocker => Some(Self::Backpack),
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Materials => "Materials",
            Self::Cargo => "Cargo",
            Self::Backpack => "Backpack",
            Self::ShipLocker => "Ship Locker",
        }
    }
}

pub struct InventoryView {
    tab: Tab,
    scroll: usize,
}

impl InventoryView {
    pub fn new() -> Self {
        Self { tab: Tab::Materials, scroll: 0 }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> ViewEvent {
        match key.code {
            KeyCode::Char('w') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::Char('s') | KeyCode::Down => {
                self.scroll += 1;
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.scroll += 10;
            }
            KeyCode::Char('d') | KeyCode::Right => {
                if let Some(next) = self.tab.next() {
                    self.tab = next;
                    self.scroll = 0;
                }
            }
            KeyCode::Char('a') | KeyCode::Left => {
                if let Some(prev) = self.tab.prev() {
                    self.tab = prev;
                    self.scroll = 0;
                }
            }
            _ => {}
        }
        ViewEvent::None
    }
}

/// Storage cap of a material: its Material Trader grade (G1 300 … G5 100).
///
/// Derived from our own data: the highest count each material reaches in the
/// `Materials` events stored on the server (2026-11-14), where thousands of
/// commanders sit at the cap. Frontier's older rarity grades differ for many
/// encoded materials and must not be used here.
fn material_cap(name: &str) -> i32 {
    match name.to_ascii_lowercase().as_str() {
        // ── Raw ──
        "carbon" | "iron" | "lead" | "nickel" | "phosphorus" | "rhenium" | "sulphur" => 300,
        "arsenic" | "chromium" | "germanium" | "manganese" | "vanadium" | "zinc" | "zirconium" => 250,
        "boron" | "cadmium" | "mercury" | "molybdenum" | "niobium" | "tin" | "tungsten" => 200,
        "antimony" | "polonium" | "ruthenium" | "selenium" | "technetium" | "tellurium" | "yttrium" => 150,

        // ── Manufactured ──
        "basicconductors" | "chemicalstorageunits" | "compactcomposites" | "crystalshards"
        | "gridresistors" | "heatconductionwiring" | "mechanicalscrap" | "salvagedalloys"
        | "temperedalloys" | "wornshieldemitters" | "guardian_powercell"
        | "guardian_sentinel_wreckagecomponents" | "tg_abrasion03" => 300,
        "chemicalprocessors" | "conductivecomponents" | "filamentcomposites"
        | "galvanisingalloys" | "heatdispersionplate" | "heatresistantceramics"
        | "hybridcapacitors" | "mechanicalequipment" | "shieldemitters"
        | "uncutfocuscrystals" | "guardian_powerconduit" | "tg_causticshard"
        | "unknowncarapace" => 250,
        "chemicaldistillery" | "conductiveceramics" | "electrochemicalarrays"
        | "focuscrystals" | "heatexchangers" | "highdensitycomposites"
        | "mechanicalcomponents" | "phasealloys" | "precipitatedalloys"
        | "shieldingsensors" | "unknownenergycell" | "guardian_sentinel_weaponparts"
        | "guardian_techcomponent" | "tg_abrasion02" | "tg_biomechanicalconduits"
        | "tg_causticgeneratorparts" | "tg_wreckagecomponents" => 200,
        "chemicalmanipulators" | "compoundshielding" | "conductivepolymers"
        | "configurablecomponents" | "fedproprietarycomposites" | "heatvanes"
        | "polymercapacitors" | "protolightalloys" | "refinedfocuscrystals"
        | "thermicalloys" | "unknowntechnologycomponents" | "tg_causticcrystal"
        | "tg_weaponparts" => 150,
        "biotechconductors" | "exquisitefocuscrystals" | "fedcorecomposites"
        | "imperialshielding" | "improvisedcomponents" | "militarygradealloys"
        | "militarysupercapacitors" | "pharmaceuticalisolators" | "protoheatradiators"
        | "protoradiolicalloys" | "unknownenergysource" | "unknownorganiccircuitry"
        | "unknowncorechip" | "tg_abrasion01" | "tg_propulsionelement" => 100,

        // ── Encoded ──
        "bulkscandata" | "disruptedwakeechoes" | "encryptedfiles" | "legacyfirmware"
        | "scrambledemissiondata" | "shieldcyclerecordings" => 300,
        "archivedemissiondata" | "consumerfirmware" | "encryptioncodes" | "fsdtelemetry"
        | "scanarchives" | "shieldsoakanalysis" | "tg_structuraldata" => 250,
        "emissiondata" | "industrialfirmware" | "scandatabanks" | "shielddensityreports"
        | "symmetrickeys" | "wakesolutions" | "tg_compositiondata" | "tg_interdictiondata"
        | "tg_shipflightdata" | "tg_shutdowndata" | "unknownshipsignature" => 200,
        "decodedemissiondata" | "encodedscandata" | "encryptionarchives"
        | "hyperspacetrajectories" | "securityfirmware" | "shieldpatternanalysis"
        | "ancientbiologicaldata" | "ancientculturaldata" | "ancienthistoricaldata"
        | "ancientlanguagedata" | "ancienttechnologicaldata" | "tg_residuedata"
        | "tg_shipsystemsdata" | "unknownwakedata" => 150,
        "adaptiveencryptors" | "classifiedscandata" | "compactemissionsdata"
        | "dataminedwake" | "embeddedfirmware" | "shieldfrequencydata" => 100,

        // Unknown to our data (e.g. Guardian blueprint segments, max seen 34).
        _ => 150,
    }
}

#[cfg(test)]
mod tests {
    use super::material_cap;

    #[test]
    fn encoded_caps_follow_the_material_trader_grades() {
        // The 15 rows of the build-158 bug report, by trader grade.
        for n in ["adaptiveencryptors", "shieldfrequencydata", "classifiedscandata", "dataminedwake", "compactemissionsdata"] {
            assert_eq!(material_cap(n), 100, "{n}");
        }
        for n in ["securityfirmware", "shieldpatternanalysis", "decodedemissiondata"] {
            assert_eq!(material_cap(n), 150, "{n}");
        }
        for n in ["wakesolutions", "symmetrickeys", "shielddensityreports"] {
            assert_eq!(material_cap(n), 200, "{n}");
        }
        for n in ["shieldsoakanalysis", "encryptioncodes", "archivedemissiondata"] {
            assert_eq!(material_cap(n), 250, "{n}");
        }
        for n in ["encryptedfiles", "scrambledemissiondata"] {
            assert_eq!(material_cap(n), 300, "{n}");
        }
    }

    #[test]
    fn names_match_regardless_of_case() {
        assert_eq!(material_cap("BiotechConductors"), 100);
        assert_eq!(material_cap("SalvagedAlloys"), 300);
    }
}
