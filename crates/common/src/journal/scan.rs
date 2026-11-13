use serde::{Deserialize, Serialize};

use crate::journal::types::{AtmosphereComposition, Composition, Material, Parent, Ring};

/// Covers both star scans (StarType present) and body/planet scans.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scan {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "ScanType")]
    pub scan_type: Option<String>,
    #[serde(rename = "BodyName")]
    pub body_name: String,
    #[serde(rename = "BodyID")]
    pub body_id: i32,
    #[serde(rename = "SystemAddress")]
    pub system_address: i64,
    #[serde(rename = "StarPos")]
    pub star_pos: Option<Vec<f32>>,
    #[serde(rename = "StarSystem")]
    pub star_system: Option<String>,
    // Star-specific (absent for bodies)
    #[serde(rename = "StarType")]
    pub star_type: Option<String>,
    #[serde(rename = "Subclass")]
    pub subclass: Option<i32>,
    #[serde(rename = "StellarMass")]
    pub stellar_mass: Option<f32>,
    #[serde(rename = "AbsoluteMagnitude")]
    pub absolute_magnitude: Option<f32>,
    #[serde(rename = "Age_MY")]
    pub age_my: Option<i32>,
    #[serde(rename = "Luminosity")]
    pub luminosity: Option<String>,
    // Body-specific (absent for stars)
    #[serde(rename = "PlanetClass")]
    pub planet_class: Option<String>,
    #[serde(rename = "MassEM")]
    pub mass_em: Option<f32>,
    #[serde(rename = "SurfacePressure")]
    pub surface_pressure: Option<f32>,
    #[serde(rename = "Landable", default)]
    pub landable: bool,
    #[serde(rename = "TidalLock", default)]
    pub tidal_lock: bool,
    #[serde(rename = "TerraformState")]
    pub terraform_state: Option<String>,
    #[serde(rename = "Atmosphere")]
    pub atmosphere: Option<String>,
    #[serde(rename = "AtmosphereType")]
    pub atmosphere_type: Option<String>,
    #[serde(rename = "AtmosphereComposition")]
    pub atmosphere_composition: Option<Vec<AtmosphereComposition>>,
    #[serde(rename = "Volcanism")]
    pub volcanism: Option<String>,
    #[serde(rename = "Composition")]
    pub composition: Option<Composition>,
    #[serde(rename = "Materials")]
    pub materials: Option<Vec<Material>>,
    // Orbital mechanics (shared, absent for main stars)
    #[serde(rename = "Radius")]
    pub radius: Option<f32>,
    #[serde(rename = "DistanceFromArrivalLS")]
    pub distance_from_arrival_ls: Option<f32>,
    #[serde(rename = "RotationPeriod")]
    pub rotation_period: Option<f32>,
    #[serde(rename = "OrbitalPeriod")]
    pub orbital_period: Option<f32>,
    #[serde(rename = "Eccentricity")]
    pub eccentricity: Option<f32>,
    #[serde(rename = "OrbitalInclination")]
    pub orbital_inclination: Option<f32>,
    #[serde(rename = "AxialTilt")]
    pub axial_tilt: Option<f32>,
    #[serde(rename = "AscendingNode")]
    pub ascending_node: Option<f32>,
    #[serde(rename = "MeanAnomaly")]
    pub mean_anomaly: Option<f32>,
    #[serde(rename = "Periapsis")]
    pub periapsis: Option<f32>,
    #[serde(rename = "SemiMajorAxis")]
    pub semi_major_axis: Option<f32>,
    #[serde(rename = "SurfaceTemperature")]
    pub surface_temperature: Option<f32>,
    #[serde(rename = "SurfaceGravity")]
    pub surface_gravity: Option<f32>,
    #[serde(rename = "Rings")]
    pub rings: Option<Vec<Ring>>,
    #[serde(rename = "ReserveLevel")]
    pub reserve_level: Option<String>,
    #[serde(rename = "Parents")]
    pub parents: Option<Vec<Parent>>,
    /// Missing from journals before 3.3.
    #[serde(rename = "WasDiscovered")]
    pub was_discovered: Option<bool>,
    #[serde(rename = "WasMapped", default)]
    pub was_mapped: bool,
    /// Missing from journals before Odyssey's footfall tracking.
    #[serde(rename = "WasFootfalled")]
    pub was_footfalled: Option<bool>,
    #[serde(rename = "EstimatedValue")]
    pub estimated_value: Option<i64>,
    #[serde(rename = "horizons", default)]
    pub horizons: bool,
    #[serde(rename = "odyssey", default)]
    pub odyssey: bool,
}

impl Scan {
    pub fn is_star(&self) -> bool {
        self.star_type.is_some()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScanBaryCentre {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "StarSystem")]
    pub star_system: String,
    #[serde(rename = "SystemAddress")]
    pub system_address: i64,
    #[serde(rename = "BodyID")]
    pub body_id: i32,
    #[serde(rename = "DistanceFromArrivalLS", default)]
    pub distance_from_arrival_ls: f32,
    #[serde(rename = "Parents")]
    pub parents: Option<Vec<Parent>>,
    #[serde(rename = "SemiMajorAxis")]
    pub semi_major_axis: Option<f32>,
    #[serde(rename = "Eccentricity")]
    pub eccentricity: Option<f32>,
    #[serde(rename = "OrbitalInclination")]
    pub orbital_inclination: Option<f32>,
    #[serde(rename = "Periapsis")]
    pub periapsis: Option<f32>,
    #[serde(rename = "OrbitalPeriod")]
    pub orbital_period: Option<f32>,
    #[serde(rename = "AscendingNode")]
    pub ascending_node: Option<f32>,
    #[serde(rename = "MeanAnomaly")]
    pub mean_anomaly: Option<f32>,
    #[serde(rename = "horizons", default)]
    pub horizons: bool,
    #[serde(rename = "odyssey", default)]
    pub odyssey: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SaaSignalsFound {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "SystemAddress")]
    pub system_address: i64,
    #[serde(rename = "BodyName")]
    pub body_name: String,
    #[serde(rename = "BodyID")]
    pub body_id: i32,
    #[serde(rename = "Signals")]
    pub signals: Vec<SaaSignal>,
    #[serde(rename = "Genuses")]
    pub genuses: Option<Vec<SaaGenus>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SaaSignal {
    #[serde(rename = "Type")]
    pub signal_type: String,
    #[serde(rename = "Type_Localised")]
    pub signal_type_localised: Option<String>,
    #[serde(rename = "Count")]
    pub count: i32,
}

impl FssSignalDiscovered {
    /// The fleet carrier this signal names, if it is one: `(callsign, name)`.
    pub fn carrier(&self) -> Option<(String, String)> {
        carrier_from_signal(&self.signal_name, self.signal_type.as_deref()?)
    }
}

/// A carrier's callsign and name from its FSS signal. Owners can rename their carrier, and
/// every sighting carries the name it has at that moment:
///
/// - fleet carriers read `"FUXBAU Q2K-BHB"`: the name, then the `XXX-XXX` callsign;
/// - squadron carriers read `"Schattenburg | STT1"`: the name, then the squadron's tag.
///
/// A carrier that has no name yet shows its callsign alone; that gives `None`, as does
/// anything that isn't a carrier.
pub fn carrier_from_signal(signal_name: &str, signal_type: &str) -> Option<(String, String)> {
    let signal_name = signal_name.trim();
    let (name, callsign) = match signal_type {
        "FleetCarrier" => {
            let (name, callsign) = signal_name.rsplit_once(' ')?;
            let b = callsign.as_bytes();
            let callsign_like = b.len() == 7
                && b[3] == b'-'
                && b.iter().enumerate().all(|(i, c)| i == 3 || c.is_ascii_uppercase() || c.is_ascii_digit());
            if !callsign_like {
                return None;
            }
            (name, callsign)
        }
        "SquadronCarrier" => {
            let (name, tag) = signal_name.rsplit_once(" | ")?;
            if tag.is_empty() || tag.contains(char::is_whitespace) {
                return None;
            }
            (name, tag)
        }
        _ => return None,
    };
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return None;
    }
    Some((callsign.to_owned(), name.to_owned()))
}

#[cfg(test)]
mod carrier_signal_tests {
    use super::carrier_from_signal;

    fn parse(n: &str, t: &str) -> Option<(String, String)> {
        carrier_from_signal(n, t)
    }

    #[test]
    fn fleet_carrier_names() {
        assert_eq!(parse("FUXBAU Q2K-BHB", "FleetCarrier"), Some(("Q2K-BHB".into(), "FUXBAU".into())));
        assert_eq!(parse("-1IPA- ERRANT VENTURE K9H-90G", "FleetCarrier"), Some(("K9H-90G".into(), "-1IPA- ERRANT VENTURE".into())));
        assert_eq!(parse("[13th] - ANDROMEDA J1F-GVV", "FleetCarrier"), Some(("J1F-GVV".into(), "[13th] - ANDROMEDA".into())));
        // Unnamed: the callsign alone.
        assert_eq!(parse("Q2K-BHB", "FleetCarrier"), None);
        assert_eq!(parse("SOME NAME q2k-bhb", "FleetCarrier"), None);
    }

    #[test]
    fn squadron_carrier_names() {
        assert_eq!(parse("Schattenburg | STT1", "SquadronCarrier"), Some(("STT1".into(), "Schattenburg".into())));
        assert_eq!(parse("colla republicats | RCAT", "SquadronCarrier"), Some(("RCAT".into(), "colla republicats".into())));
        assert_eq!(parse("FLCS", "SquadronCarrier"), None);
    }

    #[test]
    fn other_signals_are_not_carriers() {
        assert_eq!(parse("Jameson Memorial", "Outpost"), None);
        assert_eq!(parse("FUXBAU Q2K-BHB", "USS"), None);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SaaGenus {
    #[serde(rename = "Genus")]
    pub genus: String,
    #[serde(rename = "Genus_Localised")]
    pub genus_localised: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FssSignalDiscovered {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "SystemAddress")]
    pub system_address: i64,
    #[serde(rename = "SignalName")]
    pub signal_name: String,
    #[serde(rename = "SignalName_Localised")]
    pub signal_name_localised: Option<String>,
    #[serde(rename = "USSType")]
    pub uss_type: Option<String>,
    #[serde(rename = "USSType_Localised")]
    pub uss_type_localised: Option<String>,
    #[serde(rename = "SpawningState")]
    pub spawning_state: Option<String>,
    #[serde(rename = "SpawningState_Localised")]
    pub spawning_state_localised: Option<String>,
    #[serde(rename = "SpawningFaction")]
    pub spawning_faction: Option<String>,
    #[serde(rename = "ThreatLevel")]
    pub threat_level: Option<i32>,
    #[serde(rename = "TimeRemaining")]
    pub time_remaining: Option<f32>,
    #[serde(rename = "SignalType")]
    pub signal_type: Option<String>,
    #[serde(rename = "IsStation", default)]
    pub is_station: bool,
    #[serde(rename = "BodyID")]
    pub body_id: Option<i32>,
    #[serde(rename = "BodyName")]
    pub body_name: Option<String>,
    #[serde(rename = "horizons", default)]
    pub horizons: bool,
    #[serde(rename = "odyssey", default)]
    pub odyssey: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FssBodySignals {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "SystemAddress")]
    pub system_address: i64,
    #[serde(rename = "BodyName")]
    pub body_name: String,
    #[serde(rename = "BodyID")]
    pub body_id: i32,
    #[serde(rename = "Signals")]
    pub signals: Vec<SaaSignal>,
    #[serde(rename = "horizons", default)]
    pub horizons: bool,
    #[serde(rename = "odyssey", default)]
    pub odyssey: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FssDiscoveryScan {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "SystemName")]
    pub system_name: String,
    #[serde(rename = "SystemAddress")]
    pub system_address: i64,
    #[serde(rename = "Progress")]
    pub progress: f32,
    #[serde(rename = "BodyCount")]
    pub body_count: u32,
    #[serde(rename = "NonBodyCount")]
    pub non_body_count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FssAllBodiesFound {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "SystemName")]
    pub system_name: String,
    #[serde(rename = "SystemAddress")]
    pub system_address: i64,
    #[serde(rename = "Count")]
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NavBeaconScan {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "SystemAddress")]
    pub system_address: i64,
    #[serde(rename = "NumBodies")]
    pub num_bodies: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScanOrganic {
    #[serde(rename = "timestamp")]
    pub timestamp: String,
    #[serde(rename = "ScanType")]
    pub scan_type: String,
    #[serde(rename = "Genus")]
    pub genus: String,
    #[serde(rename = "Genus_Localised")]
    pub genus_localised: Option<String>,
    #[serde(rename = "Species")]
    pub species: String,
    #[serde(rename = "Species_Localised")]
    pub species_localised: Option<String>,
    #[serde(rename = "SystemAddress")]
    pub system_address: i64,
    /// The body's id, from `Body`; `None` where a journal wrote something else there.
    #[serde(rename = "Body", default, deserialize_with = "body_id")]
    pub body_id: Option<i32>,
}

fn body_id<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<i32>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(v.as_i64().and_then(|id| i32::try_from(id).ok()))
}
