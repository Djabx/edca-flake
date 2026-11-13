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

fn material_cap(name: &str) -> i32 {
    match name {
        // ── Raw G1 (cap 300) ──
        "carbon" | "iron" | "lead" | "nickel" | "phosphorus" | "rhenium" | "sulphur" => 300,
        // ── Raw G2 (cap 250) ──
        "arsenic" | "chromium" | "germanium" | "manganese" | "vanadium" | "zinc" | "zirconium" => 250,
        // ── Raw G3 (cap 200) ──
        "boron" | "cadmium" | "mercury" | "molybdenum" | "niobium" | "tin" | "tungsten" => 200,
        // ── Raw G4 (cap 150) ──
        "antimony" | "indium" | "palladium" | "polonium" | "ruthenium" | "selenium" | "technetium" | "tellurium" | "yttrium" => 150,
        // ── Raw G5 (cap 100) ──
        "technetiummagnetics" | "protoradiolicalloy" | "protolightalloy" | "protoheatsink" => 100,

        // ── Manufactured G1 (cap 300) ──
        "basicconductors" | "chemicalstorageunits" | "compactcomposites" | "crystalshards"
        | "gridresistors" | "heatconductionwiring" | "mechanicalscrap" | "salvagedalloysv1"
        | "temperedalloys" | "wornshieldemitters" => 300,
        // ── Manufactured G2 (cap 250) ──
        "chemicalprocessors" | "conductivecomponents" | "filamentcomposites"
        | "flawedfocuscrystals" | "galvanisingalloys" | "heatdispersionplate"
        | "heatresistantceramics" | "hybridcapacitors" | "mechanicalequipment"
        | "shieldemitters" | "uncutfocuscrystals" | "irregularcomponents" => 250,
        // ── Manufactured G3 (cap 200) ──
        "chemicaldistillery" | "conductiveceramics" | "electrochemicalarrays"
        | "focuscrystals" | "heatexchangers" | "highdensitycomposites"
        | "mechanicalcomponents" | "phasealloys" | "precipitatedalloys"
        | "refinedfocuscrystals" | "shieldingsensors" | "unknownfragment" => 200,
        // ── Manufactured G4 (cap 150) ──
        "chemicalmanipulators" | "compoundshielding" | "conductivepolymers"
        | "configurablecomponents" | "heatvanes" | "polymercapacitors"
        | "propulsionelements" | "protolightalloys" | "refinedalloys"
        | "thermicalloys" | "wornalloys" | "biotechconductors" => 150,
        // ── Manufactured G5 (cap 100) ──
        "biotechconductors2" | "compressedliquid" | "coredynamicscomposites"
        | "fedcorecomposites" | "fedproprietarycomposites" | "imperialshielding"
        | "improvisedfittings" | "militarygrades" | "militarygradealloys"
        | "militarysupercapacitors" | "pharmaceuticalisolators"
        | "proprietarycomposites" | "protoheatradiators" | "protoradiolicalloys"
        | "securecryptographickey" | "unknownenergycell"
        | "unknownorganiccircuitry" | "unknowntechnologycomponents" => 100,

        // ── Encoded G1 (cap 300) ──
        "bulkscandata" | "disruptedwakeechoes" | "legacyfirmware"
        | "shieldcyclerecordings" | "shieldsoakanalysis" | "wakesolutions" => 300,
        // ── Encoded G2 (cap 250) ──
        "abberantshieldpatternanalysis" | "abnormalcompactEmissionsdata"
        | "adaptiveencryptors" | "consumerfirmware" | "fsdtelemetry"
        | "inconsistentshieldsoakanalysis" | "jammingcodes" | "scanarchives"
        | "scrambledemissiondata" | "shieldfrequencydata"
        | "symmetrickeys" | "unusualencryptedfiles" => 250,
        // ── Encoded G3 (cap 200) ──
        "classifiedscandata" | "classifiedscanfragment" | "decodedemissiondata"
        | "differencedscanner" | "emissiondata" | "encryptedfiles"
        | "encryptioncodes" | "industrialfirmware" | "iotdatabank"
        | "largecapacitypowerregulator" | "modifiedconsumerfirmware"
        | "scandatabanks" | "securityfirmware"
        | "shieldpatternanalysis" | "wakescans" => 200,
        // ── Encoded G4 (cap 150) ──
        "adaptiveencryptors2" | "ancientbiologicaldata" | "ancientculturaldata"
        | "ancienthistoricaldata" | "ancientlanguagedata" | "ancienttechnologicaldata"
        | "archivedemissiondata" | "atypicaldisruptedwakeechoes"
        | "atypicalencryptionarchives" | "classifiedscandata2"
        | "dataportfolio" | "encryptionarchives" | "encryptionkeys"
        | "hyperspacetrajectories" | "legacyfirmware2" | "lostandfoundartefacts"
        | "patternepsilonobeliskdata" | "patternalphaobeliskdata"
        | "patternbetaobeliskdata" | "patterngammaobeliskdata"
        | "patternzetaobeliskdata" | "quirkycomponentdata"
        | "scandatabanks2" | "securityfirmware2" | "shieldsoakanalysis2"
        | "taggedencryptioncodes" | "unclassifiedrelic" => 150,
        // ── Encoded G5 (cap 100) ──
        "adaptiveencryptors3" | "classifiedscandata3" | "patterndeltaobeliskdata"
        | "specialisedlegacyfirmware" | "unknownenergysource"
        | "unknownshipsignature" | "unknownwakescan" => 100,

        // fallback
        _ => 150,
    }
}
