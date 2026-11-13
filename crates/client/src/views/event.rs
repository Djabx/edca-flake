pub enum ViewEvent {
    None,
    Consumed,
    SettingsChanged,
    /// Leave a contextual view for the tab it was opened from (what `q` does globally).
    Back,
    /// Speak a sample phrase at the current voice volume so the user can test the setting.
    TestVoice,
    /// Write the diagnostics zip (Settings → Shader Mods); the app adds what only it knows.
    SaveDiagnostics,
    OpenFactions(String),
    /// Show a system on the galaxy map (from the Search tab's Systems).
    OpenSystem { address: i64, name: String },
    /// An embedded view's "start typing" key: the Search tab's shared field takes it.
    FocusSearch,
    OpenSearchNearest { commodity: String, canonical_name: String, system: String, ship_pad_size: char },
    OpenMultiSearch { commodities: Vec<String>, canonical_names: Vec<String>, system: String, ship_pad_size: char },
    TrackConstruction(i64),
    /// Toggle whether the fleet carrier with this market_id is owned by the commander
    /// (persisted to my_carriers.json and reflected in journal.carrier_cargo).
    ToggleMyCarrier(i64),
}
