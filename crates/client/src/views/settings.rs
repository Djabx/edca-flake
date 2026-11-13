mod gui;

use crate::event_shim::{KeyCode, KeyEvent};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc;
use tracing::{info, warn, error};

use crate::journal_reader::{find_all_journal_files, find_latest_journal_file, start_bulk_upload, BulkUploadProgress};

use crate::settings::Settings;
use crate::settings::icons::Icon;
use crate::game_graphics;
use crate::shader_mods;
use crate::settings::game_graphics::{OverrideChoice, TintMode, DUST_REACH_RANGE, FOG_DEPTH_RANGE, MAX_LOOK_PERCENT};
use crate::settings::journal_reader::ActionAtShutdownSignal;
use crate::views::ViewEvent;

#[derive(Default, Clone, Copy, PartialEq)]
enum SettingsSection {
    #[default]
    JournalReader,
    Appearance,
    Explorer,
    GalaxyMap,
    GameGraphics,
    Alerts,
    Icons,
    Stars,
    Planets,
    About,
}

impl SettingsSection {
    fn all() -> Vec<Self> {
        vec![
            SettingsSection::JournalReader,
            SettingsSection::Appearance,
            SettingsSection::Explorer,
            SettingsSection::GalaxyMap,
            SettingsSection::GameGraphics,
            SettingsSection::Alerts,
            SettingsSection::Icons,
            SettingsSection::Stars,
            SettingsSection::Planets,
            SettingsSection::About,
        ]
    }

    fn title(&self) -> &'static str {
        match self {
            SettingsSection::JournalReader => "Journal Reader",
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Explorer => "Explorer",
            SettingsSection::GalaxyMap => "Galaxy Map",
            SettingsSection::GameGraphics => "Game Graphics",
            SettingsSection::Alerts => "Alerts",
            SettingsSection::Icons => "Icons",
            SettingsSection::Stars => "Stars",
            SettingsSection::Planets => "Planets",
            SettingsSection::About => "About",
        }
    }
}

#[derive(Clone, PartialEq)]
enum CellType {
    /// A non-interactive group heading that starts a new "paragraph" of related settings.
    /// Navigation skips over these and they have no editable value.
    Header(&'static str),
    /// A non-interactive, non-selectable status line (message, is_warning) shown under a
    /// field to report computed/live state (e.g. how many journal files were found).
    Info(String, bool),
    Label(String),
    StringValue(String),
    BoolValue(bool),
    ToggleEnabled(bool),
    Button(&'static str),
}

struct GridRow {
    cells: Vec<CellType>,
}

/// Computes the live "is this path right?" status shown under the Journal Directory field.
fn journal_directory_status(settings: &Settings) -> CellType {
    let dir = settings.journal_reader.journal_directory.trim();
    if dir.is_empty() {
        return CellType::Info("  (not set)".to_string(), true);
    }
    let count = find_all_journal_files(&PathBuf::from(dir)).len();
    if count == 0 {
        CellType::Info("  ⚠ 0 journal files found at this path".to_string(), true)
    } else {
        CellType::Info(format!("  ✓ {count} journal file(s) found"), false)
    }
}

/// The ground tint modes as the settings show them.
pub(crate) const TINT_MODES: [&str; 2] = ["Per planet", "Fixed colour"];

fn tint_mode_name(mode: TintMode) -> &'static str {
    match mode {
        TintMode::PerPlanet => TINT_MODES[0],
        TintMode::Fixed => TINT_MODES[1],
    }
}

/// Whether the planet ground tint can reach the game, shown under its settings.
fn ground_tint_status(settings: &Settings) -> CellType {
    use crate::ground_tint;
    match ground_tint::game_dir(settings) {
        None => CellType::Info("⚠ The game folder wasn't found next to the journal folder — set it above".to_string(), true),
        Some(dir) if !dir.is_dir() => CellType::Info("⚠ That folder doesn't exist".to_string(), true),
        Some(dir) if !ground_tint::installed(&dir) => CellType::Info(
            "⚠ The shader mods aren't installed in this game folder yet — Install them under Shader Mods below".to_string(),
            true,
        ),
        Some(_) if settings.game_graphics.ground_tint && settings.game_graphics.ground_tint_mode == TintMode::Fixed => CellType::Info(
            "✓ Every planet's ground takes this colour, also with EDCAS closed".to_string(),
            false,
        ),
        Some(_) if settings.game_graphics.ground_tint => CellType::Info(
            "✓ The ground follows the planet you are at".to_string(),
            false,
        ),
        Some(_) => CellType::Info("Off — 3DMigoto shows the game's own ground colours".to_string(), false),
    }
}

/// What the planet clouds do, shown under their settings.
fn planet_clouds_status(settings: &Settings, now: &str) -> CellType {
    if !settings.game_graphics.planet_clouds {
        return CellType::Info("Off — 3DMigoto shows the game's own sky".to_string(), false);
    }
    if now.is_empty() {
        return CellType::Info("✓ The sky follows the planet you are at, while EDCAS runs".to_string(), false);
    }
    CellType::Info(format!("✓ {now}"), false)
}

/// Rank of a star class key for display ordering: the Harvard spectral sequence (O B A F G K M,
/// hot→cool), then the cool brown-dwarf classes, giants/supergiants, proto stars, Wolf-Rayet,
/// carbon, S-type, white dwarfs, compact remnants and finally non-stellar bodies. Keys not in
/// the table rank last and fall back to alphabetical order, so unknown future classes still sort
/// deterministically.
fn star_class_rank(key: &str) -> usize {
    const ORDER: &[&str] = &[
        // Main sequence, hot → cool
        "O", "B", "A", "F", "G", "K", "M",
        // Cool dwarfs
        "L", "T", "Y",
        // Giants & supergiants
        "A_BlueWhiteSuperGiant", "F_WhiteSuperGiant", "K_OrangeGiant", "M_RedGiant",
        "M_RedSuperGiant",
        // Proto stars
        "TTS", "AeBe",
        // Wolf-Rayet
        "W", "WN", "WNC", "WC", "WO",
        // Carbon stars
        "C", "CN", "CJ", "CH", "CHd", "CS",
        // S-type
        "S", "MS", "MS S",
        // White dwarfs
        "D", "DA", "DAB", "DAV", "DAZ", "DAO", "DB", "DBV", "DBZ", "DC", "DCV", "DO", "DOV",
        "DQ", "DX",
        // Compact remnants
        "N", "H", "SupermassiveBlackHole",
        // Non-stellar / exotic
        "X", "RoguePlanet", "Nebula", "StellarRemnantNebula",
    ];
    ORDER.iter().position(|&c| c == key).unwrap_or(usize::MAX)
}

pub struct SettingsView {
    section: SettingsSection,
    sidebar_focus: usize,
    row: usize,
    col: usize,
    editing: bool,
    edit_buffer: String,
    /// Validation error from the last edit attempt, or a short status message from the last
    /// button press (e.g. a precondition that wasn't met). Shown inline; cleared on the next
    /// edit/button interaction.
    feedback: Option<String>,
    focus: FocusArea,
    bulk_upload_rx: Option<mpsc::Receiver<BulkUploadProgress>>,
    bulk_upload_progress: Option<BulkUploadProgress>,
    /// What the game graphics section last found or wrote: (message, is_warning).
    graphics_status: Option<(String, bool)>,
    /// What the clouds of the planet the commander is at come to, kept by the app.
    pub cloud_status: String,
    /// The shader mods job that is running (install, update, remove), what it is doing now,
    /// and how the last one ended: (message, is_warning).
    shader_rx: Option<mpsc::Receiver<shader_mods::Event>>,
    shader_step: String,
    shader_result: Option<(String, bool)>,
    /// What another installer (EDHM's) has undone in the game folder, checked at most every
    /// few seconds: the check reads the dll and `d3dx.ini`.
    damage: std::cell::RefCell<Option<(std::time::Instant, Vec<&'static str>)>>,
    /// The diagnostics zip being written, and the path of the last one written (for Copy Path).
    diagnostics_rx: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    diagnostics_path: Option<String>,
    clipboard: Option<arboard::Clipboard>,
}

enum FocusArea {
    Sidebar,
    Content,
}

impl SettingsView {
    pub fn new() -> Self {
        Self {
            section: SettingsSection::default(),
            sidebar_focus: 0,
            row: 0,
            col: 0,
            editing: false,
            edit_buffer: String::new(),
            feedback: None,
            focus: FocusArea::Sidebar,
            bulk_upload_rx: None,
            bulk_upload_progress: None,
            graphics_status: None,
            cloud_status: String::new(),
            shader_rx: None,
            shader_step: String::new(),
            shader_result: None,
            damage: std::cell::RefCell::new(None),
            diagnostics_rx: None,
            diagnostics_path: None,
            clipboard: None,
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent, settings: &mut Settings) -> ViewEvent {
        if self.editing {
            match key.code {
                KeyCode::Enter => {
                    self.apply_edit(settings);
                    if self.feedback.is_none() {
                        self.editing = false;
                        self.edit_buffer.clear();
                        return ViewEvent::SettingsChanged;
                    }
                    // Invalid input: stay in edit mode so the user can correct it.
                }
                KeyCode::Esc => {
                    self.editing = false;
                    self.edit_buffer.clear();
                    self.feedback = None;
                }
                KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                }
                KeyCode::Backspace => {
                    self.edit_buffer.pop();
                }
                _ => {}
            }
            return ViewEvent::Consumed;
        }

        // Any navigation/action key starts fresh — stale feedback from a previous field or
        // button press shouldn't linger once the user has moved on.
        self.feedback = None;

        match key.code {
            KeyCode::Tab => {
                let sections = SettingsSection::all();
                self.sidebar_focus = (self.sidebar_focus + 1) % sections.len();
                self.section = sections[self.sidebar_focus];
                self.reset_content_position(settings);
                return ViewEvent::Consumed;
            }
            KeyCode::Char('w') | KeyCode::Up => match self.focus {
                FocusArea::Sidebar => {
                    if self.sidebar_focus > 0 {
                        self.sidebar_focus -= 1;
                        self.section = SettingsSection::all()[self.sidebar_focus];
                        self.reset_content_position(settings);
                    }
                }
                FocusArea::Content => {
                    self.move_to_prev_selectable(settings);
                }
            },
            KeyCode::Char('s') | KeyCode::Down => match self.focus {
                FocusArea::Sidebar => {
                    let section_count = SettingsSection::all().len();
                    if self.sidebar_focus + 1 < section_count {
                        self.sidebar_focus += 1;
                        self.section = SettingsSection::all()[self.sidebar_focus];
                        self.reset_content_position(settings);
                    }
                }
                FocusArea::Content => {
                    self.move_to_next_selectable(settings);
                }
            },
            KeyCode::Char('a') | KeyCode::Left => match self.focus {
                FocusArea::Sidebar => {}
                FocusArea::Content => {
                    if self.col > 0 {
                        self.col -= 1;
                    } else {
                        self.focus = FocusArea::Sidebar;
                    }
                }
            },
            KeyCode::Char('d') | KeyCode::Right => match self.focus {
                FocusArea::Sidebar => {
                    self.focus = FocusArea::Content;
                }
                FocusArea::Content => {
                    let grid = self.build_grid(settings);
                    let max_col = if self.row < grid.len() {
                        grid[self.row].cells.len()
                    } else {
                        0
                    };
                    if self.col + 1 < max_col {
                        self.col += 1;
                    }
                }
            },
            KeyCode::Char(' ') | KeyCode::Enter => {
                if matches!(self.focus, FocusArea::Content) {
                    let grid = self.build_grid(settings);
                    if self.row < grid.len() && self.col < grid[self.row].cells.len() {
                        let cell = grid[self.row].cells[self.col].clone();
                        match cell {
                            CellType::StringValue(ref s) => {
                                self.editing = true;
                                self.edit_buffer = s.clone();
                                self.feedback = None;
                            }
                            CellType::BoolValue(_) => {
                                self.toggle_bool(settings);
                                return ViewEvent::SettingsChanged;
                            }
                            CellType::ToggleEnabled(_) => {
                                self.toggle_enabled(settings);
                                return ViewEvent::SettingsChanged;
                            }
                            CellType::Button("[ Open in default app ]") => {
                                self.feedback = self.open_current_log(settings).err();
                            }
                            CellType::Button("[ Upload All Journal Logs ]") => {
                                self.feedback = self.start_bulk_upload(settings).err();
                            }
                            CellType::Button("[ Play Sample ]") => {
                                return ViewEvent::TestVoice;
                            }
                            CellType::Button("[ Install ]" | "[ Update ]") => self.start_shader_job(settings, shader_mods::Job::Install),
                            CellType::Button("[ Finish Game Shaders ]") => self.start_shader_job(settings, shader_mods::Job::FinishGameShaders),
                            CellType::Button("[ Repair ]") => self.start_shader_job(settings, shader_mods::Job::Repair),
                            CellType::Button("[ Remove ]") => self.start_shader_job(settings, shader_mods::Job::Remove),
                            CellType::Button("[ Save Diagnostics ]") => return ViewEvent::SaveDiagnostics,
                            CellType::Button("[ Copy Path ]") => self.copy_diagnostics_path(),
                            CellType::Button("[ Copy ]") => self.copy_launch_option(),
                            CellType::Button("[ Restore Original File ]") => {
                                self.restore_graphics(settings);
                                return ViewEvent::SettingsChanged;
                            }
                            CellType::Button(_) => {}
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }

        ViewEvent::None
    }

    fn reset_content_position(&mut self, settings: &Settings) {
        if self.section == SettingsSection::GameGraphics {
            self.refresh_graphics_status(settings);
        }
        self.row = 0;
        self.col = 0;
        // The first row may be a group header; land on the first selectable row instead.
        if !self.row_selectable(self.row, settings) {
            self.move_to_next_selectable(settings);
        }
        self.clamp_col(settings);
    }

    /// A row is selectable unless its first cell is a [`CellType::Header`] (a group heading).
    fn row_selectable(&self, idx: usize, settings: &Settings) -> bool {
        let grid = self.build_grid(settings);
        !matches!(
            grid.get(idx).and_then(|r| r.cells.first()),
            Some(CellType::Header(_)) | Some(CellType::Info(_, _))
        )
    }

    /// Moves the cursor up to the nearest selectable row, skipping headers.
    fn move_to_prev_selectable(&mut self, settings: &Settings) {
        let mut i = self.row;
        while i > 0 {
            i -= 1;
            if self.row_selectable(i, settings) {
                self.row = i;
                self.clamp_col(settings);
                return;
            }
        }
    }

    /// Moves the cursor down to the nearest selectable row, skipping headers.
    fn move_to_next_selectable(&mut self, settings: &Settings) {
        let row_count = self.build_grid(settings).len();
        let mut i = self.row + 1;
        while i < row_count {
            if self.row_selectable(i, settings) {
                self.row = i;
                self.clamp_col(settings);
                return;
            }
            i += 1;
        }
    }

    /// Label of the row the cursor is on, if its first cell is a [`CellType::Label`]. Used by
    /// the edit/toggle handlers so they key off the setting's name rather than a row index
    /// (which shifts when rows are regrouped or headers are inserted).
    fn row_label(&self, settings: &Settings) -> Option<String> {
        let grid = self.build_grid(settings);
        match grid.get(self.row).and_then(|r| r.cells.first()) {
            Some(CellType::Label(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn clamp_col(&mut self, settings: &Settings) {
        let grid = self.build_grid(settings);
        if self.row < grid.len() {
            let max_col = grid[self.row].cells.len();
            if self.col >= max_col {
                self.col = max_col.saturating_sub(1);
            }
        }
    }

    fn is_icon_section(&self) -> bool {
        matches!(
            self.section,
            SettingsSection::Icons | SettingsSection::Stars | SettingsSection::Planets
        )
    }

    fn get_icon_keys(&self, settings: &Settings) -> Vec<String> {
        let icons: &HashMap<String, Icon> = match self.section {
            SettingsSection::Icons => &settings.icons,
            SettingsSection::Stars => &settings.stars,
            SettingsSection::Planets => &settings.planets,
            _ => &settings.icons,
        };
        let mut keys: Vec<String> = icons.keys().cloned().collect();
        if self.section == SettingsSection::Stars {
            // Order by the Harvard spectral sequence rather than alphabetically, with the key
            // name as a stable tie-breaker for any classes sharing a rank.
            keys.sort_by(|a, b| {
                star_class_rank(a)
                    .cmp(&star_class_rank(b))
                    .then_with(|| a.cmp(b))
            });
        } else {
            keys.sort();
        }
        keys
    }

    fn build_grid(&self, settings: &Settings) -> Vec<GridRow> {
        if self.section == SettingsSection::About {
            return Vec::new();
        }
        if self.is_icon_section() {
            self.build_icon_grid(settings)
        } else {
            self.build_regular_grid(settings)
        }
    }

    fn build_regular_grid(&self, settings: &Settings) -> Vec<GridRow> {
        match self.section {
            SettingsSection::JournalReader => vec![
                // ── Journal Reader ──────────────────────────────────────
                GridRow {
                    cells: vec![
                        CellType::Label("Journal Directory".to_string()),
                        CellType::StringValue(settings.journal_reader.journal_directory.clone()),
                    ],
                },
                GridRow { cells: vec![journal_directory_status(settings)] },
                GridRow {
                    cells: vec![
                        CellType::Label("Current Log File".to_string()),
                        CellType::Button("[ Open in default app ]"),
                    ],
                },
                GridRow {
                    cells: vec![
                        CellType::Label("Action at Shutdown".to_string()),
                        CellType::StringValue(settings.journal_reader.action_at_shutdown_signal.to_string()),
                    ],
                },
                // ── edcas API ───────────────────────────────────────────
                GridRow {
                    cells: vec![CellType::Header("edcas API")],
                },
                GridRow {
                    cells: vec![
                        CellType::Label("edcas API URL".to_string()),
                        CellType::StringValue(settings.api_url.clone()),
                    ],
                },
                GridRow {
                    cells: vec![
                        CellType::Label("Send to edcas API".to_string()),
                        CellType::BoolValue(settings.edcas_api_enabled),
                    ],
                },
                GridRow {
                    cells: vec![
                        CellType::Label("Upload History".to_string()),
                        CellType::Button("[ Upload All Journal Logs ]"),
                    ],
                },
                // ── EDDN ────────────────────────────────────────────────
                GridRow {
                    cells: vec![CellType::Header("EDDN")],
                },
                GridRow {
                    cells: vec![
                        CellType::Label("Send to EDDN".to_string()),
                        CellType::BoolValue(settings.eddn_enabled),
                    ],
                },
                GridRow {
                    cells: vec![
                        CellType::Label("EDDN URL".to_string()),
                        CellType::StringValue(settings.eddn_url.clone()),
                    ],
                },
                GridRow {
                    cells: vec![
                        CellType::Label("EDDN Test Mode".to_string()),
                        CellType::BoolValue(settings.eddn_test_mode),
                    ],
                },
            ],
            SettingsSection::Appearance => vec![GridRow {
                cells: vec![
                    CellType::Label("Accent Color (name, #rrggbb, or r,g,b)".to_string()),
                    CellType::StringValue(settings.appearance.color.clone()),
                ],
            }],
            SettingsSection::Explorer => vec![GridRow {
                cells: vec![
                    CellType::Label("Include System Name".to_string()),
                    CellType::BoolValue(settings.explorer.include_system_name),
                ],
            }],
            SettingsSection::GalaxyMap => vec![
                GridRow {
                    cells: vec![
                        CellType::Label("Stars Near Focus".to_string()),
                        CellType::StringValue(settings.galaxy_map.nearby_stars.to_string()),
                    ],
                },
                GridRow {
                    cells: vec![
                        CellType::Label("Stars Sampled Around".to_string()),
                        CellType::StringValue(settings.galaxy_map.sampled_stars.to_string()),
                    ],
                },
            ],
            SettingsSection::GameGraphics => {
                let g = &settings.game_graphics;
                let (status, warning) = self.graphics_status.clone().unwrap_or_default();
                let mut rows = vec![
                    GridRow { cells: vec![CellType::Header("Graphics Override File")] },
                    GridRow {
                        cells: vec![CellType::Label("Manage Override File".to_string()), CellType::BoolValue(g.manage)],
                    },
                    GridRow {
                        cells: vec![CellType::Label("Graphics Folder".to_string()), CellType::StringValue(g.folder.clone())],
                    },
                    GridRow { cells: vec![CellType::Info(status, warning)] },
                    GridRow {
                        cells: vec![
                            CellType::Label("Original File".to_string()),
                            CellType::Button("[ Restore Original File ]"),
                        ],
                    },
                ];
                let mut group = "";
                for o in game_graphics::OPTIONS {
                    if o.group != group {
                        group = o.group;
                        rows.push(GridRow { cells: vec![CellType::Header(o.group)] });
                    }
                    let choice = g.options.get(o.id);
                    rows.push(GridRow {
                        cells: vec![
                            CellType::Label(o.label.to_string()),
                            CellType::BoolValue(choice.is_some_and(|c| c.enabled)),
                            CellType::StringValue(choice.map(|c| c.value.clone()).unwrap_or_default()),
                        ],
                    });
                }
                // The mods' own rows, by what they change: the install block first (Game Folder is its
                // gate), then the planet's surface, its weather, the light, the cockpit (2026-11-09).
                rows.extend(self.shader_mod_rows(settings));
                rows.extend([
                    GridRow { cells: vec![CellType::Header("Planet Surface")] },
                    GridRow {
                        cells: vec![CellType::Label("Planet Ground Tint".to_string()), CellType::BoolValue(g.ground_tint)],
                    },
                    GridRow {
                        cells: vec![CellType::Label("  Tint Mode".to_string()), CellType::StringValue(tint_mode_name(g.ground_tint_mode).to_string())],
                    },
                ]);
                if g.ground_tint_mode == TintMode::Fixed {
                    rows.push(GridRow {
                        cells: vec![CellType::Label("  Tint Colour".to_string()), CellType::StringValue(g.ground_tint_colour.clone())],
                    });
                }
                let percent = |label: &str, value: u16| GridRow {
                    cells: vec![CellType::Label(label.to_string()), CellType::StringValue(value.to_string())],
                };
                let switch = |label: &str, on: bool| GridRow { cells: vec![CellType::Label(label.to_string()), CellType::BoolValue(on)] };
                rows.extend([
                    percent("  Tint Strength %", u16::from(g.ground_tint_strength)),
                    switch("Ground Detail", g.ground_detail),
                    percent("  Detail Strength (%)", g.ground_detail_strength),
                    GridRow { cells: vec![ground_tint_status(settings)] },
                    GridRow { cells: vec![CellType::Header("Weather")] },
                    switch("Planet Clouds", g.planet_clouds),
                    percent("  Cloud Strength %", u16::from(g.planet_clouds_strength)),
                    switch("  Volumetric Clouds (costs FPS)", g.planet_clouds_volumetric),
                    percent("  Cloud Thickness (%)", g.planet_cloud_thickness),
                    percent("  Cloud Edge Softness (%)", g.planet_cloud_softness),
                    switch("Ground Fog", g.planet_fog),
                    percent("  Fog Density (%)", g.planet_fog_density),
                    GridRow {
                        cells: vec![CellType::Label("  Fog Depth (m)".to_string()), CellType::StringValue(g.planet_fog_depth.to_string())],
                    },
                    percent("  Lamp Glow in Fog (%)", g.planet_lamp_glow),
                    switch("Wind-blown Dust", g.planet_dust),
                    percent("  Dust Strength (%)", g.planet_dust_strength),
                    GridRow {
                        cells: vec![CellType::Label("  Dust Reach (km)".to_string()), CellType::StringValue(g.planet_dust_reach_km.to_string())],
                    },
                    GridRow { cells: vec![planet_clouds_status(settings, &self.cloud_status)] },
                    GridRow { cells: vec![CellType::Header("Lighting")] },
                    switch("Star Light Colour", g.star_light),
                    percent("  Star Light Strength (%)", g.star_light_strength()),
                    switch("  Witch Space Rainbow", g.star_light_rainbow),
                    GridRow { cells: vec![CellType::Header("Cockpit")] },
                    switch("Canopy Frost", g.planet_canopy_frost),
                    percent("  Canopy Frost Strength (%)", g.planet_canopy_frost_strength),
                ]);
                rows
            }
            SettingsSection::Alerts => {
                let a = &settings.alerts;
                let toggle = |label: &str, on: bool| GridRow {
                    cells: vec![CellType::Label(label.to_string()), CellType::BoolValue(on)],
                };
                vec![
                    toggle("Alerts Enabled", a.enabled),
                    toggle("Voice (TTS)", a.voice),
                    GridRow {
                        cells: vec![
                            CellType::Label("  Voice Volume %".to_string()),
                            CellType::StringValue(a.voice_volume.to_string()),
                        ],
                    },
                    GridRow {
                        cells: vec![
                            CellType::Label("  Test Voice".to_string()),
                            CellType::Button("[ Play Sample ]"),
                        ],
                    },
                    GridRow { cells: vec![CellType::Header("Hazards")] },
                    toggle("Low Fuel", a.low_fuel),
                    toggle("Overheating", a.overheating),
                    toggle("Being Interdicted", a.interdiction),
                    toggle("In Danger", a.in_danger),
                    toggle("Low Oxygen", a.low_oxygen),
                    toggle("Low Health", a.low_health),
                    GridRow { cells: vec![CellType::Header("Discoveries")] },
                    toggle("Non-Human Signal", a.non_human_signal),
                    toggle("Guardian Signal", a.guardian_signal),
                    toggle("High Bio Signals", a.high_bio_signals),
                    GridRow {
                        cells: vec![
                            CellType::Label("  Bio Threshold".to_string()),
                            CellType::StringValue(a.high_bio_threshold.to_string()),
                        ],
                    },
                    toggle("High Geo Signals", a.high_geo_signals),
                    GridRow {
                        cells: vec![
                            CellType::Label("  Geo Threshold".to_string()),
                            CellType::StringValue(a.high_geo_threshold.to_string()),
                        ],
                    },
                    toggle("Undiscovered Valuable Body", a.valuable_body),
                    GridRow { cells: vec![CellType::Header("Fleet Carrier")] },
                    toggle("Carrier Jump Ready", a.carrier_jump_ready),
                ]
            }
            _ => vec![],
        }
    }

    fn build_icon_grid(&self, settings: &Settings) -> Vec<GridRow> {
        let keys = self.get_icon_keys(settings);
        let icons: &HashMap<String, Icon> = match self.section {
            SettingsSection::Icons => &settings.icons,
            SettingsSection::Stars => &settings.stars,
            SettingsSection::Planets => &settings.planets,
            _ => &settings.icons,
        };

        keys.iter()
            .filter_map(|key| {
                icons.get(key).map(|icon| {
                    let mut cells = vec![CellType::Label(key.clone())];
                    if self.shows_symbol(key) {
                        cells.push(CellType::StringValue(icon.char.clone()));
                    }
                    cells.push(CellType::StringValue(icon.color.clone()));
                    cells.push(CellType::ToggleEnabled(icon.enabled));
                    GridRow { cells }
                })
            })
            .collect()
    }

    /// Whether an icon row of this section has a symbol to edit, before its colour; the
    /// rest are drawn as shapes and take only the colour.
    fn shows_symbol(&self, key: &str) -> bool {
        self.section == SettingsSection::Icons && crate::settings::icons::shows_symbol(key)
    }

    /// Validates and commits the current edit buffer. On success, clears `self.feedback`.
    /// On failure, sets `self.feedback` to an explanatory message and leaves `settings`
    /// unchanged so the caller can keep the field in edit mode.
    fn apply_edit(&mut self, settings: &mut Settings) {
        let value = self.edit_buffer.clone();
        self.feedback = None;

        if self.is_icon_section() {
            let icon_keys = self.get_icon_keys(settings);
            if let Some(key) = icon_keys.get(self.row) {
                let icon = match self.section {
                    SettingsSection::Icons => settings.icons.get_mut(key),
                    SettingsSection::Stars => settings.stars.get_mut(key),
                    SettingsSection::Planets => settings.planets.get_mut(key),
                    _ => None,
                };
                // Without a symbol cell the colour sits in the symbol's column.
                let field = if self.shows_symbol(key) { self.col } else { self.col + 1 };
                if let Some(icon) = icon {
                    match field {
                        1 => {
                            info!("Updated icon char for '{}': '{}'", key, value);
                            icon.char = value;
                        }
                        2 => {
                            if value.trim().is_empty() || crate::theme::parse_rgb(value.trim()).is_some() {
                                info!("Updated icon color for '{}': '{}'", key, value);
                                icon.color = value;
                            } else {
                                self.feedback = Some(
                                    "Invalid color. Use a name (e.g. orange), #rrggbb, or r,g,b".to_string(),
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else {
            match self.section {
                SettingsSection::JournalReader => match self.row_label(settings).as_deref() {
                    Some("Journal Directory") => {
                        info!("Updated journal directory: '{}'", value);
                        settings.journal_reader.journal_directory = value;
                    }
                    Some("Action at Shutdown") => {
                        if let Ok(action) = ActionAtShutdownSignal::from_str(&value) {
                            info!("Updated shutdown action: {}", action);
                            settings.journal_reader.action_at_shutdown_signal = action;
                        } else {
                            warn!("Invalid shutdown action value: '{}'", value);
                            self.feedback = Some("Expected: Exit, Nothing, or Continue".to_string());
                        }
                    }
                    Some("edcas API URL") => {
                        info!("Updated edcas API URL: '{}'", value);
                        settings.api_url = value;
                    }
                    Some("EDDN URL") => {
                        info!("Updated EDDN URL: '{}'", value);
                        settings.eddn_url = value;
                    }
                    _ => {}
                },
                SettingsSection::Appearance => {
                    if self.row == 0 {
                        if value.trim().is_empty() || crate::theme::parse_rgb(value.trim()).is_some() {
                            info!("Updated appearance color: '{}'", value);
                            settings.appearance.color = value;
                        } else {
                            self.feedback = Some(
                                "Invalid color. Use a name (e.g. orange), #rrggbb, or r,g,b".to_string(),
                            );
                        }
                    }
                }
                SettingsSection::GalaxyMap => {
                    use crate::settings::galaxy_map::{MAX_STARS, MIN_STARS};
                    let label = self.row_label(settings);
                    match value.trim().replace(['_', ',', '.', ' '], "").parse::<u32>() {
                        Ok(n) if (MIN_STARS..=MAX_STARS).contains(&n) => match label.as_deref() {
                            Some("Stars Near Focus") => {
                                settings.galaxy_map.nearby_stars = n;
                                info!("Updated galaxy map stars near focus: {n}");
                            }
                            Some("Stars Sampled Around") => {
                                settings.galaxy_map.sampled_stars = n;
                                info!("Updated galaxy map sampled stars: {n}");
                            }
                            _ => {}
                        },
                        _ => {
                            self.feedback = Some(format!("Must be a whole number {MIN_STARS}-{MAX_STARS}"));
                        }
                    }
                }
                SettingsSection::GameGraphics => match self.row_label(settings).as_deref() {
                    Some("  Tint Strength %") => match value.trim().trim_end_matches('%').trim().parse::<u8>() {
                        Ok(n) if n <= 100 => {
                            settings.game_graphics.ground_tint_strength = n;
                            info!("Updated ground tint strength: {n} %");
                        }
                        _ => self.feedback = Some("Must be a whole number 0-100".to_string()),
                    },
                    Some("  Cloud Strength %") => match value.trim().trim_end_matches('%').trim().parse::<u8>() {
                        Ok(n) if n <= 100 => {
                            settings.game_graphics.planet_clouds_strength = n;
                            info!("Updated planet cloud strength: {n} %");
                        }
                        _ => self.feedback = Some("Must be a whole number 0-100".to_string()),
                    },
                    Some(label @ ("  Cloud Thickness (%)" | "  Cloud Edge Softness (%)" | "  Fog Density (%)" | "  Dust Strength (%)" | "  Detail Strength (%)" | "  Canopy Frost Strength (%)" | "  Lamp Glow in Fog (%)" | "  Star Light Strength (%)")) => {
                        match value.trim().trim_end_matches('%').trim().parse::<u16>() {
                            Ok(n) if n <= MAX_LOOK_PERCENT => {
                                let g = &mut settings.game_graphics;
                                match label {
                                    "  Cloud Thickness (%)" => g.planet_cloud_thickness = n,
                                    "  Cloud Edge Softness (%)" => g.planet_cloud_softness = n,
                                    "  Canopy Frost Strength (%)" => g.planet_canopy_frost_strength = n,
                                    "  Lamp Glow in Fog (%)" => g.planet_lamp_glow = n,
                                    "  Star Light Strength (%)" => g.set_star_light_strength(n),
                                    "  Dust Strength (%)" => g.planet_dust_strength = n,
                                    "  Detail Strength (%)" => g.ground_detail_strength = n,
                                    _ => g.planet_fog_density = n,
                                }
                                info!("Updated {label}: {n}");
                            }
                            _ => self.feedback = Some(format!("Must be a whole number 0-{MAX_LOOK_PERCENT}")),
                        }
                    }
                    Some("  Dust Reach (km)") => match value.trim().trim_end_matches("km").trim().parse::<u16>() {
                        Ok(n) if DUST_REACH_RANGE.contains(&n) => {
                            settings.game_graphics.planet_dust_reach_km = n;
                            info!("Updated planet dust reach: {n} km");
                        }
                        _ => self.feedback = Some(format!("Must be a whole number {}-{} (kilometres)", DUST_REACH_RANGE.start(), DUST_REACH_RANGE.end())),
                    },
                    Some("  Fog Depth (m)") => match value.trim().trim_end_matches('m').trim().parse::<u16>() {
                        Ok(n) if FOG_DEPTH_RANGE.contains(&n) => {
                            settings.game_graphics.planet_fog_depth = n;
                            info!("Updated planet fog depth: {n} m");
                        }
                        _ => self.feedback = Some(format!("Must be a whole number {}-{} (metres)", FOG_DEPTH_RANGE.start(), FOG_DEPTH_RANGE.end())),
                    },
                    Some("  Tint Mode") => match TINT_MODES.iter().position(|m| m.eq_ignore_ascii_case(value.trim())) {
                        Some(i) => {
                            settings.game_graphics.ground_tint_mode = if i == 0 { TintMode::PerPlanet } else { TintMode::Fixed };
                            info!("Updated ground tint mode: {}", TINT_MODES[i]);
                        }
                        None => self.feedback = Some("Expected: Per planet or Fixed colour".to_string()),
                    },
                    Some("  Tint Colour") => match crate::theme::parse_rgb(&value) {
                        Some(_) => {
                            settings.game_graphics.ground_tint_colour = value.trim().to_string();
                            info!("Updated ground tint colour: {}", value.trim());
                        }
                        None => self.feedback = Some("Use #rrggbb, r,g,b or a colour name".to_string()),
                    },
                    Some("Game Folder") => {
                        info!("Updated game folder: '{}'", value.trim());
                        settings.game_graphics.game_folder = value.trim().to_string();
                    }
                    Some("Graphics Folder") => {
                        info!("Updated game graphics folder: '{}'", value.trim());
                        settings.game_graphics.folder = value.trim().to_string();
                        self.sync_graphics(settings);
                    }
                    Some(label) => {
                        if let Some(o) = game_graphics::option_by_label(label) {
                            match o.kind.normalize(&value) {
                                // Setting a value opts in to it.
                                Ok(v) => {
                                    info!("Game graphics override {}: {}", o.id, v);
                                    settings.game_graphics.options.insert(o.id.to_string(), OverrideChoice { enabled: true, value: v });
                                    self.sync_graphics(settings);
                                }
                                Err(e) => self.feedback = Some(e),
                            }
                        }
                    }
                    None => {}
                },
                SettingsSection::Alerts => {
                    let label = self.row_label(settings);
                    if let Ok(n) = value.trim().parse::<u8>() {
                        match label.as_deref() {
                            Some("  Bio Threshold") => {
                                settings.alerts.high_bio_threshold = n;
                                info!("Updated high bio threshold: {n}");
                            }
                            Some("  Geo Threshold") => {
                                settings.alerts.high_geo_threshold = n;
                                info!("Updated high geo threshold: {n}");
                            }
                            Some("  Voice Volume %") => {
                                if n <= 100 {
                                    settings.alerts.voice_volume = n;
                                    info!("Updated voice volume: {}%", settings.alerts.voice_volume);
                                } else {
                                    self.feedback = Some("Must be 0-100".to_string());
                                }
                            }
                            _ => {}
                        }
                    } else {
                        warn!("Invalid threshold value: '{}'", value);
                        self.feedback = Some("Must be a whole number 0-255".to_string());
                    }
                }
                _ => {}
            }
        }
    }

    /// The Shader Mods block of the Game Graphics section: the buttons that fit what the
    /// game folder has, what the mods stand at, how the last job ended, and, outside Windows,
    /// the Steam launch option Proton needs.
    fn shader_mod_rows(&self, settings: &Settings) -> Vec<GridRow> {
        use shader_mods::Status;
        let info = |text: String, warning: bool| GridRow { cells: vec![CellType::Info(text, warning)] };
        let mut rows = vec![
            GridRow { cells: vec![CellType::Header("Shader Mods (3DMigoto)")] },
            GridRow { cells: vec![CellType::Label("Game Folder".to_string()), CellType::StringValue(settings.game_graphics.game_folder.clone())] },
        ];
        // Save Diagnostics is always there, also when the game folder is the problem: that is
        // when a tester most needs to send us the zip.
        let diagnostics = |cells: &mut Vec<CellType>| {
            cells.push(CellType::Button("[ Save Diagnostics ]"));
            if self.diagnostics_path.is_some() {
                cells.push(CellType::Button("[ Copy Path ]"));
            }
        };
        let found = crate::ground_tint::game_dir(settings);
        let not_found = found.is_none();
        match found.filter(|d| d.is_dir()) {
            None => {
                let mut cells = vec![CellType::Label("Shader Mods".to_string())];
                diagnostics(&mut cells);
                rows.push(GridRow { cells });
                let why = if not_found { "wasn't found next to the journal folder" } else { "doesn't exist" };
                rows.push(info(format!("⚠ The game folder {why} — set Game Folder above"), true));
            }
            Some(dir) => {
                let status = shader_mods::status(&dir, shader_mods::VERSION);
                let want = shader_mods::Wanted::of(&settings.game_graphics);
                let (buttons, text): (&[&'static str], String) = match &status {
                    Status::NotInstalled => (
                        &["[ Install ]"],
                        "Not installed — Install downloads 3DMigoto and sets up ground tint, clouds, ground fog and canopy frost".to_string(),
                    ),
                    Status::Installed { ground_tint: true, star_light } if *star_light == shader_mods::StarLightShaders::TOTAL => {
                        (&["[ Remove ]"], format!("✓ Installed (EDCAS {}) — the ground tint and the star light are ready", shader_mods::VERSION))
                    }
                    Status::Installed { ground_tint: true, star_light } if !want.star_light => (
                        &["[ Finish Game Shaders ]", "[ Remove ]"],
                        format!(
                            "✓ Installed (EDCAS {}) — the ground tint is ready; star light: {star_light} of {} game shaders copied. Star Light Colour is off, so the game is not asked for the rest",
                            shader_mods::VERSION,
                            shader_mods::StarLightShaders::TOTAL
                        ),
                    ),
                    Status::Installed { ground_tint: true, star_light } => (
                        &["[ Finish Game Shaders ]", "[ Remove ]"],
                        format!(
                            "✓ Installed (EDCAS {}) — the ground tint is ready; star light: {star_light} of {} game shaders copied, the rest come as the game runs (space, then a planet with an atmosphere)",
                            shader_mods::VERSION,
                            shader_mods::StarLightShaders::TOTAL
                        ),
                    ),
                    Status::Installed { ground_tint: false, .. } if !want.ground => (
                        &["[ Finish Game Shaders ]", "[ Remove ]"],
                        "✓ Clouds, fog and frost are installed. Ground Tint, Ground Detail and the dust are off, so the game is not asked for its ground shaders".to_string(),
                    ),
                    Status::Installed { ground_tint: false, .. } => (
                        &["[ Finish Game Shaders ]", "[ Remove ]"],
                        "✓ Clouds, fog and frost are installed. Ground tint: land on a planet once, then press Finish Game Shaders".to_string(),
                    ),
                    Status::UpdateAvailable { installed } => (
                        &["[ Update ]", "[ Remove ]"],
                        format!(
                            "Update available: installed {}, this EDCAS is {}",
                            installed.as_deref().map_or("by hand or unknown".to_string(), |v| format!("from EDCAS {v}")),
                            shader_mods::VERSION
                        ),
                    ),
                };
                let damage = {
                    let mut cached = self.damage.borrow_mut();
                    match &*cached {
                        Some((at, found)) if at.elapsed() < std::time::Duration::from_secs(5) => found.clone(),
                        _ => {
                            let found = shader_mods::damage(&dir);
                            *cached = Some((std::time::Instant::now(), found.clone()));
                            found
                        }
                    }
                };
                let mut cells = vec![CellType::Label("Shader Mods".to_string())];
                if !damage.is_empty() {
                    cells.push(CellType::Button("[ Repair ]"));
                }
                cells.extend(buttons.iter().map(|b| CellType::Button(b)));
                diagnostics(&mut cells);
                rows.push(GridRow { cells });
                rows.push(info(text, false));
                if !damage.is_empty() {
                    rows.push(info("⚠ Another installer (EDHM's) changed the game folder. Press Repair (close the game first on Windows); EDCAS also repairs it by itself when the game closes.".to_string(), true));
                    rows.extend(damage.iter().map(|d| info(format!("   • {d}"), true)));
                }
            }
        }
        if let Some((message, warning)) = &self.shader_result {
            let prefix = if *warning { "" } else { "✓ " };
            rows.extend(message.lines().enumerate().map(|(i, l)| info(if i == 0 { format!("{prefix}{l}") } else { l.to_string() }, *warning)));
        }
        if cfg!(not(windows)) {
            rows.push(GridRow { cells: vec![CellType::Label("Steam Launch Option".to_string()), CellType::Button("[ Copy ]")] });
            rows.push(info(format!("Under Proton set this in Steam (game properties → Launch Options): {}", shader_mods::STEAM_LAUNCH_OPTION), false));
        }
        rows
    }

    /// Starts install/update/remove on a thread of its own; [`Self::poll_shader_mods`] picks up the end.
    fn start_shader_job(&mut self, settings: &Settings, job: shader_mods::Job) {
        if self.shader_rx.is_some() {
            return;
        }
        let Some(dir) = crate::ground_tint::game_dir(settings) else {
            self.feedback = Some("The game folder wasn't found: set Game Folder above".to_string());
            return;
        };
        info!("Shader mods: {job:?} in {}", dir.display());
        self.shader_result = None;
        self.shader_step = "Working…".to_string();
        self.shader_rx = Some(shader_mods::start(job, dir, settings.api_url.clone(), shader_mods::Wanted::of(&settings.game_graphics)));
    }

    /// Starts the diagnostics zip on a thread of its own; [`Self::poll_shader_mods`] picks up the end.
    pub fn start_diagnostics(&mut self, settings: &Settings, live: crate::diagnostics::Live) {
        if self.diagnostics_rx.is_some() {
            return;
        }
        info!("Diagnostics: writing the zip");
        self.shader_result = None;
        self.shader_step = "Collecting diagnostics…".to_string();
        self.diagnostics_rx = Some(crate::diagnostics::start(settings, live));
    }

    /// Takes in what the running shader mods job and the diagnostics zip reported.
    pub fn poll_shader_mods(&mut self) {
        if let Some(rx) = &self.diagnostics_rx {
            if let Ok(result) = rx.try_recv() {
                self.shader_result = Some(match result {
                    Ok(path) => {
                        let path = path.display().to_string();
                        self.diagnostics_path = Some(path.clone());
                        (format!("Diagnostics saved: {path}\nSend this file with your bug report; Copy Path puts its location on the clipboard."), false)
                    }
                    Err(e) => (format!("⚠ Diagnostics failed: {e}"), true),
                });
                self.diagnostics_rx = None;
            }
        }
        let Some(rx) = &self.shader_rx else { return };
        let mut done = None;
        while let Ok(event) = rx.try_recv() {
            match event {
                shader_mods::Event::Step(s) => self.shader_step = format!("{s}…"),
                shader_mods::Event::Done(r) => done = Some(r),
            }
        }
        if let Some(result) = done {
            self.shader_result = Some(match result {
                Ok(message) => (message, false),
                Err(e) => {
                    let mut chars = e.chars();
                    let first = chars.next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default();
                    (format!("⚠ {first}{}", chars.as_str()), true)
                }
            });
            self.shader_rx = None;
        }
    }

    /// Puts the saved diagnostics zip's path on the clipboard.
    fn copy_diagnostics_path(&mut self) {
        let Some(path) = self.diagnostics_path.clone() else { return };
        let copied = self.copy_text(&path);
        self.shader_result = Some(if copied {
            (format!("Path copied: {path}"), false)
        } else {
            (format!("⚠ The clipboard isn't available: the zip is at {path}"), true)
        });
    }

    fn copy_text(&mut self, text: &str) -> bool {
        // Created on first use and kept: on X11/Wayland the clipboard is served by this
        // object's thread, so dropping it takes the text with it.
        if self.clipboard.is_none() {
            self.clipboard = arboard::Clipboard::new().ok();
        }
        self.clipboard.as_mut().is_some_and(|c| c.set_text(text).is_ok())
    }

    /// Puts the Steam launch option on the clipboard.
    fn copy_launch_option(&mut self) {
        let copied = self.copy_text(shader_mods::STEAM_LAUNCH_OPTION);
        self.shader_result = Some(if copied {
            ("Launch option copied".to_string(), false)
        } else {
            ("⚠ The clipboard isn't available: copy the launch option from the line below".to_string(), true)
        });
    }

    fn start_bulk_upload(&mut self, settings: &Settings) -> Result<(), String> {
        let Some(api_url) = settings.edcas_api_url() else {
            warn!("edcas API upload disabled or URL not configured — cannot upload journal history");
            return Err("edcas API upload is disabled — enable it above first".to_string());
        };
        let dir = settings.journal_reader.journal_directory.trim().to_string();
        if dir.is_empty() {
            warn!("Journal directory not configured");
            return Err("No journal directory configured — set one above first".to_string());
        }
        info!("Starting bulk journal upload from: {}", dir);
        let rx = start_bulk_upload(PathBuf::from(dir), api_url);
        self.bulk_upload_rx = Some(rx);
        self.bulk_upload_progress = None;
        Ok(())
    }

    pub fn poll_bulk_upload(&mut self) {
        if let Some(ref rx) = self.bulk_upload_rx {
            // Drain all pending updates, keeping the last one.
            let mut last = None;
            while let Ok(p) = rx.try_recv() {
                last = Some(p);
            }
            if let Some(p) = last {
                let done = p.done || p.error.is_some();
                self.bulk_upload_progress = Some(p);
                if done {
                    self.bulk_upload_rx = None;
                }
            }
        }
    }

    fn open_current_log(&self, settings: &Settings) -> Result<(), String> {
        let dir = &settings.journal_reader.journal_directory;
        if dir.is_empty() {
            warn!("Journal directory not configured");
            return Err("No journal directory configured — set one above first".to_string());
        }
        match find_latest_journal_file(&PathBuf::from(dir)) {
            Some(path) => {
                info!("Opening log file: {}", path.display());
                if let Err(e) = opener::open(&path) {
                    error!("Failed to open log file: {}", e);
                    return Err(format!("Failed to open log file: {e}"));
                }
                Ok(())
            }
            None => {
                warn!("No journal log file found in: {}", dir);
                Err(format!("No journal log file found in: {dir}"))
            }
        }
    }

    /// Toggles the boolean setting at the focused section/row.
    fn toggle_bool(&mut self, settings: &mut Settings) {
        match self.section {
            SettingsSection::GameGraphics => match self.row_label(settings).as_deref() {
                Some("Planet Ground Tint") => {
                    let g = &mut settings.game_graphics;
                    g.ground_tint = !g.ground_tint;
                    info!("Toggled planet ground tint: {}", g.ground_tint);
                }
                Some("Ground Detail") => {
                    let g = &mut settings.game_graphics;
                    g.ground_detail = !g.ground_detail;
                    info!("Toggled ground detail: {}", g.ground_detail);
                }
                Some("Planet Clouds") => {
                    let g = &mut settings.game_graphics;
                    g.planet_clouds = !g.planet_clouds;
                    info!("Toggled planet clouds: {}", g.planet_clouds);
                }
                Some("  Volumetric Clouds (costs FPS)") => {
                    let g = &mut settings.game_graphics;
                    g.planet_clouds_volumetric = !g.planet_clouds_volumetric;
                    info!("Toggled volumetric planet clouds: {}", g.planet_clouds_volumetric);
                }
                Some("Ground Fog") => {
                    let g = &mut settings.game_graphics;
                    g.planet_fog = !g.planet_fog;
                    info!("Toggled planet ground fog: {}", g.planet_fog);
                }
                Some("Wind-blown Dust") => {
                    let g = &mut settings.game_graphics;
                    g.planet_dust = !g.planet_dust;
                    info!("Toggled wind-blown dust: {}", g.planet_dust);
                }
                Some("Canopy Frost") => {
                    let g = &mut settings.game_graphics;
                    g.planet_canopy_frost = !g.planet_canopy_frost;
                    info!("Toggled canopy frost: {}", g.planet_canopy_frost);
                }
                Some("Star Light Colour") => {
                    let g = &mut settings.game_graphics;
                    g.star_light = !g.star_light;
                    info!("Toggled star light colour: {}", g.star_light);
                }
                Some("  Witch Space Rainbow") => {
                    let g = &mut settings.game_graphics;
                    g.star_light_rainbow = !g.star_light_rainbow;
                    info!("Toggled witch space rainbow: {}", g.star_light_rainbow);
                }
                Some("Manage Override File") => {
                    let g = &mut settings.game_graphics;
                    g.manage = !g.manage;
                    info!("Toggled game graphics management: {}", g.manage);
                    if g.manage {
                        // Keep what the player already set in the file by hand.
                        if let Some(dir) = game_graphics::graphics_dir(settings) {
                            match game_graphics::adopt_existing(&mut settings.game_graphics, &dir) {
                                Ok(n) if n > 0 => info!("Adopted {n} override(s) from the game's file"),
                                Ok(_) => {}
                                Err(e) => warn!("{e}"),
                            }
                        }
                    }
                    self.sync_graphics(settings);
                }
                Some(label) => {
                    if let Some(o) = game_graphics::option_by_label(label) {
                        let choice = settings
                            .game_graphics
                            .options
                            .entry(o.id.to_string())
                            .or_insert_with(|| OverrideChoice { enabled: false, value: o.suggested.to_string() });
                        choice.enabled = !choice.enabled;
                        if choice.value.trim().is_empty() {
                            choice.value = o.suggested.to_string();
                        }
                        info!("Toggled game graphics override {}: {}", o.id, choice.enabled);
                        self.sync_graphics(settings);
                    }
                }
                None => {}
            },
            SettingsSection::Explorer => {
                settings.explorer.include_system_name = !settings.explorer.include_system_name;
                info!("Toggled include_system_name: {}", settings.explorer.include_system_name);
            }
            SettingsSection::JournalReader => match self.row_label(settings).as_deref() {
                Some("Send to edcas API") => {
                    settings.edcas_api_enabled = !settings.edcas_api_enabled;
                    info!("Toggled edcas API upload: {}", settings.edcas_api_enabled);
                }
                Some("Send to EDDN") => {
                    settings.eddn_enabled = !settings.eddn_enabled;
                    info!("Toggled EDDN upload: {}", settings.eddn_enabled);
                }
                Some("EDDN Test Mode") => {
                    settings.eddn_test_mode = !settings.eddn_test_mode;
                    info!("Toggled EDDN test mode: {}", settings.eddn_test_mode);
                }
                _ => {}
            },
            SettingsSection::Alerts => {
                let label = self.row_label(settings);
                let a = &mut settings.alerts;
                match label.as_deref() {
                    Some("Alerts Enabled") => a.enabled = !a.enabled,
                    Some("Voice (TTS)") => a.voice = !a.voice,
                    Some("Low Fuel") => a.low_fuel = !a.low_fuel,
                    Some("Overheating") => a.overheating = !a.overheating,
                    Some("Being Interdicted") => a.interdiction = !a.interdiction,
                    Some("In Danger") => a.in_danger = !a.in_danger,
                    Some("Low Oxygen") => a.low_oxygen = !a.low_oxygen,
                    Some("Low Health") => a.low_health = !a.low_health,
                    Some("Non-Human Signal") => a.non_human_signal = !a.non_human_signal,
                    Some("Guardian Signal") => a.guardian_signal = !a.guardian_signal,
                    Some("High Bio Signals") => a.high_bio_signals = !a.high_bio_signals,
                    Some("High Geo Signals") => a.high_geo_signals = !a.high_geo_signals,
                    Some("Undiscovered Valuable Body") => a.valuable_body = !a.valuable_body,
                    Some("Carrier Jump Ready") => a.carrier_jump_ready = !a.carrier_jump_ready,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Writes the overrides into the game's file when EDCAS manages it, and says how that
    /// went.
    fn sync_graphics(&mut self, settings: &Settings) {
        if !settings.game_graphics.manage {
            self.refresh_graphics_status(settings);
            return;
        }
        let Some(dir) = game_graphics::graphics_dir(settings) else {
            self.refresh_graphics_status(settings);
            return;
        };
        self.graphics_status = Some(match game_graphics::write(&settings.game_graphics, &dir) {
            Ok(w) => {
                info!("Wrote {} game graphics override(s) to {}", w.ours, w.path.display());
                let others = match w.others {
                    0 => String::new(),
                    1 => " · 1 other value kept".to_string(),
                    n => format!(" · {n} other values kept"),
                };
                (format!("✓ {} override(s) written{others} — restart the game to see them", w.ours), false)
            }
            Err(e) => {
                warn!("Game graphics: {e}");
                (e, true)
            }
        });
    }

    /// Where the game's file is, without writing it.
    fn refresh_graphics_status(&mut self, settings: &Settings) {
        let g = &settings.game_graphics;
        self.graphics_status = Some(match game_graphics::graphics_dir(settings) {
            // The folder is in the field above, so the status names only the file.
            Some(dir) if dir.is_dir() => {
                let file = game_graphics::FILE_NAME;
                let what = if dir.join(file).exists() { file.to_string() } else { format!("{file} (not there yet)") };
                if g.manage {
                    (format!("✓ EDCAS manages {what}"), false)
                } else {
                    (format!("Off — EDCAS leaves {what} as it is"), false)
                }
            }
            Some(_) => ("⚠ That folder doesn't exist".to_string(), true),
            None => ("⚠ The game's Options/Graphics folder wasn't found next to the journal folder — set it above".to_string(), true),
        });
    }

    /// Puts the game's file back as it was and stops managing it; the choices stay, so
    /// turning management on again brings them back.
    fn restore_graphics(&mut self, settings: &mut Settings) {
        let Some(dir) = game_graphics::graphics_dir(settings) else {
            self.feedback = Some("The game's Options/Graphics folder wasn't found".to_string());
            return;
        };
        match game_graphics::restore(&settings.game_graphics, &dir) {
            Ok(msg) => {
                info!("Game graphics: {msg}");
                settings.game_graphics.manage = false;
                self.graphics_status = Some((format!("✓ {msg}"), false));
            }
            Err(e) => {
                warn!("Game graphics: {e}");
                self.feedback = Some(e);
            }
        }
    }

    fn toggle_enabled(&self, settings: &mut Settings) {
        let icon_keys = self.get_icon_keys(settings);
        if let Some(key) = icon_keys.get(self.row) {
            let icon = match self.section {
                SettingsSection::Icons => settings.icons.get_mut(key),
                SettingsSection::Stars => settings.stars.get_mut(key),
                SettingsSection::Planets => settings.planets.get_mut(key),
                _ => None,
            };
            if let Some(icon) = icon {
                icon.enabled = !icon.enabled;
                info!("Toggled icon '{}' enabled: {}", key, icon.enabled);
            }
        }
    }

    pub fn save_settings(&self, settings: &Settings) {
        {
            let json = serde_json::to_string_pretty(settings).expect("Failed to serialize settings");
            let settings_path = crate::settings::config_dir().join("settings.json");

            if let Some(parent) = settings_path.parent() {
                let _ = std::fs::create_dir_all(parent);
                let tmp = parent.join("settings.json.tmp");
                if std::fs::write(&tmp, &json).is_ok() {
                    match std::fs::rename(&tmp, &settings_path) {
                        Ok(_) => tracing::info!("Settings saved to {}", settings_path.display()),
                        Err(e) => tracing::error!("Failed to save settings to {}: {}", settings_path.display(), e),
                    }
                } else {
                    tracing::error!("Failed to write settings tmp file");
                }
            }
        }
    }
}
