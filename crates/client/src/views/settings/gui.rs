//! Desktop-window drawing for [`SettingsView`]: a section sidebar and the section's fields as
//! real controls — text fields, switches, a slider, number fields, colour pickers and swatches
//! and the icon tables.
//!
//! Every change goes through the keyboard's paths: a control first moves the cursor
//! (`row`/`col`) onto its cell, then either presses the cell as Space/Enter would, or fills the
//! edit buffer and commits it as Enter would — so validation, saving and the live accent all
//! come from `handle_key` and `ViewEvent::SettingsChanged`. Switches, choices, swatches, the
//! slider and number fields save at once, a picked colour when the pointer lets go; text fields save on Enter or when they lose focus,
//! and Esc cancels.

use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, Id, Rect, RichText, Sense, Shape, Stroke, Ui};
use egui_extras::{Column, Size, StripBuilder, TableBuilder};

use super::{mods_installed, CellType, FocusArea, SettingsSection, SettingsView, GRAPHICS_TOPICS, MOD_TOPICS};
use crate::event_shim::{KeyCode, KeyEvent};
use crate::gui::theme::{self, accent, accent_dim, accent_hi, accent_mute, alpha, BG, CYAN, GREEN, GREY, RED, TEXT, TEXT_DIM};
use crate::gui::widgets;
use crate::gui::colour_picker;
use crate::settings::game_graphics::{DUST_REACH_RANGE, FOG_DEPTH_RANGE, HUD_BRIGHTNESS_RANGE, MAX_LOOK_PERCENT};
use crate::settings::Settings;
use crate::views::ViewEvent;

const LOGO_PNG: &[u8] = include_bytes!("../../../../../assets/graphics/logo/edcas_128.png");

const SIDEBAR_W: f32 = 200.0;
/// Where the Game Graphics page keeps which topic it shows at its top, for the sidebar's mark.
const TOPIC_SHOWN: &str = "settings-graphics-topic";
const LABEL_W: f32 = 240.0;
const ROW_H: f32 = 30.0;
/// Named accents `crate::theme::parse_rgb` knows, offered as swatches.
const ACCENTS: [&str; 10] = ["orange", "purple", "white", "red", "green", "blue", "yellow", "cyan", "magenta", "teal"];
const SHUTDOWN_ACTIONS: [&str; 3] = ["Exit", "Nothing", "Continue"];
/// What the picker says under its field where only the colour counts.
const GAME_COLOUR_NOTE: &str = "Only the colour counts here: how bright it is drawn stays the game's.";
const GAME_FADE_NOTE: &str = "Only the colours count here: how bright they are drawn stays the game's. A fade runs across the screen, not across each panel.";
/// Gauge colours offered as swatches next to a HUD gauge's colour field.
const GAUGE_PRESETS: [(&str, &str); 8] = [
    ("red", "#ff3030"),
    ("orange", "#ff8000"),
    ("yellow", "#ffd700"),
    ("green", "#30e060"),
    ("cyan", "#30d8e8"),
    ("blue", "#4080ff"),
    ("purple", "#b060ff"),
    ("white", "#ffffff"),
];
/// Ground tint colours offered as swatches next to the colour field.
const TINT_PRESETS: [(&str, &str); 7] = [
    ("ice", "#a8c8ff"),
    ("frost", "#e6efff"),
    ("steel", "#9aa6b2"),
    ("sand", "#d9c08f"),
    ("rust", "#b5653a"),
    ("moss", "#8fa06a"),
    ("ash", "#8a8580"),
];

/// The help under a Game Graphics topic, after its last row (2026-11-19: they stood together at the
/// page's end, far from what they explain).
fn topic_hint(ui: &mut Ui, topic: &str) {
    let text = match topic {
        "Weather" => "Clouds are rare: they form only where a gas of the planet's air is cold enough to freeze, \
             and most planets with an atmosphere are too warm for that. The ✓ lines under Planet Surface \
             and Weather say what the planet you are at gets. Ground fog shows only on such planets, near the ground (below about \
             10 km, none from 25 km up), at night and until the sun is about 20° up, filling the lowest \
             ground in view up to Fog Depth (on a high ridge you can be above it). At night it is as dark \
             as the night: switch on the ship's lights or the suit torch to see it.",
        "HUD" => "HUD: the colour turns a ship's panels, its head-up lines and the radar (its disc, rings and contacts), each as its row under HUD Colour says: left Elite's, turned by the hue shift, or in one colour of its own (which paints over the second colour Elite marks things with), or in two that fade into each other across the screen (pick Fade in the colour's picker; the game loads the shaders for a fade at its next start); a contact Elite marks \
             red (a hostile ship) stays red, the two holograms keep Elite's colours under it, and so do the SRV's and the suit's HUD. The game has two shaders for the panels, \
             with and without the cockpit's shadows, and hands EDCAS each the first time it draws with it: until it has used both, the panels keep Elite's colours where it takes the other. \
             Ship Hologram Colour gives your own ship's hologram, right of the radar, one colour, and Target \
             Hologram Colour the target's, left of it. A gauge or a hologram with a look of its own keeps it where Elite would change the colour, as \
             a warning say. Everything here is off until you switch it on, and with another HUD mod (EDHM) in the \
             game folder EDCAS leaves the HUD to it.",
        "Graphics Overrides" => "Elite Dangerous reads GraphicsConfigurationOverride.xml when it starts: its values replace the \
             game's own, beyond what the in-game menu offers. With Manage on, EDCAS writes the overrides you \
             switch on into every quality level, removes the ones you switch off, and leaves everything else in \
             the file alone. Your original file is kept next to it (.edcas-original); Restore puts it back. \
             Restart the game after a change.",
        _ => return,
    };
    ui.add_space(6.0);
    widgets::hint(ui, text);
}

/// Keeps the more important of two events from one frame: a save outranks everything else.
fn keep(ev: &mut ViewEvent, new: ViewEvent) {
    match (&*ev, &new) {
        (_, ViewEvent::None) | (ViewEvent::SettingsChanged, _) => {}
        _ => *ev = new,
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::from(code)
}

impl SettingsView {
    pub fn ui(&mut self, ui: &mut Ui, settings: &mut Settings) -> ViewEvent {
        let mut ev = ViewEvent::None;
        StripBuilder::new(ui)
            .size(Size::exact(SIDEBAR_W))
            .size(Size::exact(10.0))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    widgets::hud_panel(ui, "Settings", "", |ui| self.sidebar_ui(ui, settings, &mut ev));
                });
                strip.empty();
                strip.cell(|ui| {
                    let caption = match self.section {
                        SettingsSection::About => "help · build",
                        SettingsSection::Icons => "symbol · colour · shown",
                        SettingsSection::Stars | SettingsSection::Planets => "colour · shown",
                        SettingsSection::GameGraphics => "opt in · pick your value",
                        _ => "changes save at once",
                    };
                    widgets::hud_panel(ui, self.section.title(), caption, |ui| self.content_ui(ui, settings, &mut ev));
                });
            });
        ev
    }

    /// The section list. Clicking one selects it as the arrow keys do in the sidebar.
    fn sidebar_ui(&mut self, ui: &mut Ui, settings: &mut Settings, ev: &mut ViewEvent) {
        ui.add_space(2.0);
        let sidebar_focused = matches!(self.focus, FocusArea::Sidebar);
        for (i, section) in SettingsSection::all().into_iter().enumerate() {
            let current = i == self.sidebar_focus;
            // While the fields have the keyboard, the current section is outlined, not filled.
            let filled = current && sidebar_focused;
            let resp = widgets::selectable_row(ui, filled, 30.0, |ui| {
                let col = if current && !filled { accent_hi() } else { widgets::row_text_color(filled, TEXT) };
                let font = if current { theme::body_semi(16.5) } else { theme::body(16.5) };
                ui.add(egui::Label::new(RichText::new(section.title()).font(font).color(col)).selectable(false));
            });
            if current && !filled {
                ui.painter().rect_stroke(resp.rect, 0.0, Stroke::new(1.0, accent()), egui::StrokeKind::Inside);
                ui.painter().rect_filled(Rect::from_min_size(resp.rect.min, vec2(3.0, resp.rect.height())), 0.0, accent());
            }
            if resp.clicked() {
                keep(ev, self.finish_open_edit(settings));
                self.feedback = None;
                self.focus = FocusArea::Sidebar;
                if i != self.sidebar_focus {
                    self.sidebar_focus = i;
                    self.section = section;
                    self.reset_content_position(settings);
                }
            }
            if section == SettingsSection::GameGraphics && self.section == SettingsSection::GameGraphics {
                self.topics_ui(ui);
            }
        }
        ui.with_layout(egui::Layout::bottom_up(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for (k, desc) in [("a / d", "sidebar / fields"), ("w / s", "move"), ("tab", "next section")] {
                ui.horizontal(|ui| widgets::key_hint(ui, k, desc));
            }
        });
    }

    /// The Game Graphics page's topics under its sidebar entry: a click scrolls the page to the
    /// topic's header; the topic the page shows at its top is marked (as the page found it last frame).
    fn topics_ui(&mut self, ui: &mut Ui) {
        let shown = ui.ctx().data(|d| d.get_temp::<usize>(Id::unique(TOPIC_SHOWN)));
        for (t, topic) in GRAPHICS_TOPICS.iter().enumerate() {
            let here = shown == Some(t);
            let resp = widgets::selectable_row(ui, false, 24.0, |ui| {
                ui.add_space(16.0);
                let col = if here { accent_hi() } else { TEXT_DIM };
                let font = if here { theme::body_semi(14.5) } else { theme::body(14.5) };
                ui.add(egui::Label::new(RichText::new(*topic).font(font).color(col)).selectable(false));
            });
            if here {
                ui.painter().rect_filled(Rect::from_min_size(resp.rect.min + vec2(6.0, 4.0), vec2(2.0, resp.rect.height() - 8.0)), 0.0, accent());
            }
            if resp.clicked() {
                self.jump_to = Some(topic);
            }
        }
    }

    fn content_ui(&mut self, ui: &mut Ui, settings: &mut Settings, ev: &mut ViewEvent) {
        let progress_h = if self.bulk_upload_progress.is_some() { 58.0 } else { 0.0 };
        let body_h = (ui.available_height() - progress_h).max(40.0);
        ui.allocate_ui(vec2(ui.available_width(), body_h), |ui| match self.section {
            SettingsSection::About => about_ui(ui),
            SettingsSection::Icons | SettingsSection::Stars | SettingsSection::Planets => self.icons_ui(ui, settings, ev),
            _ => self.fields_ui(ui, settings, ev),
        });
        if self.bulk_upload_progress.is_some() {
            self.progress_ui(ui);
        }
    }

    // ------------------------------------------------------------ cursor & commits

    /// Puts the cursor on a cell the pointer used, as the arrow keys would.
    fn select_cell(&mut self, row: usize, col: usize) {
        self.focus = FocusArea::Content;
        self.row = row;
        self.col = col;
    }

    /// Closes an edit still open on some cell: saves it like Enter, or drops it like Esc when
    /// the value doesn't validate.
    fn finish_open_edit(&mut self, settings: &mut Settings) -> ViewEvent {
        if !self.editing {
            return ViewEvent::None;
        }
        let ev = self.handle_key(&key(KeyCode::Enter), settings);
        if self.editing {
            self.handle_key(&key(KeyCode::Esc), settings);
            return ViewEvent::None;
        }
        ev
    }

    /// Presses a cell (switch, button) exactly as Space does on it.
    fn press_cell(&mut self, row: usize, col: usize, settings: &mut Settings) -> ViewEvent {
        let mut ev = self.finish_open_edit(settings);
        self.select_cell(row, col);
        keep(&mut ev, self.handle_key(&key(KeyCode::Char(' ')), settings));
        ev
    }

    /// Writes `value` into a cell as if it had been typed and saved with Enter.
    fn commit_value(&mut self, row: usize, col: usize, value: String, settings: &mut Settings) -> ViewEvent {
        let mut ev = self.finish_open_edit(settings);
        self.select_cell(row, col);
        self.editing = true;
        self.edit_buffer = value;
        keep(&mut ev, self.handle_key(&key(KeyCode::Enter), settings));
        ev
    }

    fn cursor_on(&self, row: usize) -> bool {
        matches!(self.focus, FocusArea::Content) && self.row == row
    }

    /// Whether the keyboard cursor moved since the last frame (to scroll it into view).
    fn cursor_moved(&self, ui: &Ui) -> bool {
        let id = Id::unique("settings-cursor");
        let now = (self.sidebar_focus, self.row, matches!(self.focus, FocusArea::Content));
        let before = ui.ctx().data_mut(|d| {
            let before = d.get_temp::<(usize, usize, bool)>(id);
            d.insert_temp(id, now);
            before
        });
        before != Some(now)
    }

    // ------------------------------------------------------------ regular sections

    fn fields_ui(&mut self, ui: &mut Ui, settings: &mut Settings, ev: &mut ViewEvent) {
        let grid = self.build_grid(settings);
        let moved = self.cursor_moved(ui);
        let graphics = self.section == SettingsSection::GameGraphics;
        // Without the shader mods in the game folder, what they would do is greyed out and locked.
        let locked = graphics && !mods_installed(settings);
        egui::ScrollArea::vertical()
            .id_salt(("settings-fields", self.sidebar_focus))
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                // The topic shown at the top of the view: the last whose header lies above a line a
                // little under the view's top edge.
                let view_top = ui.clip_rect().top() + 40.0;
                let mut shown = 0;
                let mut topic: Option<&str> = None;
                // The game graphics take the window's width: beside each override stands what it does
                // and what it costs, which on a wide screen is one line (the owner, 2026-11-18).
                let widest = if self.section == SettingsSection::GameGraphics { f32::INFINITY } else { 900.0 };
                ui.set_max_width(ui.available_width().min(widest));
                ui.add_space(2.0);
                widgets::hint(ui, "Switches, choices, sliders, swatches and picked colours save at once. Text fields save on Enter or when you click elsewhere; Esc cancels.");
                ui.add_space(6.0);
                for (idx, row) in grid.iter().enumerate() {
                    match row.cells.first() {
                        Some(CellType::Header(title)) if graphics && !GRAPHICS_TOPICS.contains(title) => {
                            // A group of the override file, under its topic.
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.add_space(10.0);
                                ui.label(RichText::new(*title).font(theme::body_semi(14.5)).color(accent_hi()));
                            });
                        }
                        Some(CellType::Header(title)) => {
                            if let Some(done) = topic.take() {
                                topic_hint(ui, done);
                            }
                            ui.add_space(8.0);
                            let at = ui.next_widget_position();
                            widgets::section(ui, title, None);
                            if let Some(t) = GRAPHICS_TOPICS.iter().position(|t| t == title).filter(|_| graphics) {
                                topic = Some(GRAPHICS_TOPICS[t]);
                                if at.y <= view_top {
                                    shown = t;
                                }
                                if self.jump_to == Some(GRAPHICS_TOPICS[t]) {
                                    self.jump_to = None;
                                    ui.scroll_to_rect(Rect::from_min_size(at, vec2(1.0, 24.0)), Some(Align::TOP));
                                }
                                if locked && MOD_TOPICS.contains(&GRAPHICS_TOPICS[t]) {
                                    ui.horizontal(|ui| {
                                        ui.add_space(10.0);
                                        ui.label(RichText::new("Install the shader mods (Shader Mods, above) to change these.").font(theme::body(14.5)).color(TEXT_DIM));
                                    });
                                }
                            }
                        }
                        Some(CellType::Info(msg, warning)) => {
                            ui.horizontal(|ui| {
                                ui.add_space(LABEL_W);
                                let col = if *warning { RED } else { GREEN };
                                ui.label(RichText::new(msg.trim()).color(col).font(theme::body(14.5)));
                            });
                        }
                        Some(CellType::Label(label)) => {
                            let values = row.cells[1..].to_vec();
                            if locked && topic.is_some_and(|t| MOD_TOPICS.contains(&t)) {
                                ui.add_enabled_ui(false, |ui| {
                                    ui.multiply_opacity(0.45);
                                    self.field_row(ui, idx, label, values, moved, settings, ev);
                                });
                            } else {
                                self.field_row(ui, idx, label, values, moved, settings, ev);
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(done) = topic {
                    topic_hint(ui, done);
                }
                if graphics {
                    ui.ctx().data_mut(|d| d.insert_temp(Id::unique(TOPIC_SHOWN), shown));
                    // Room under the last topic, so that a jump to it can bring its header to the top.
                    ui.add_space(ui.clip_rect().height() * 0.6);
                }
                if self.section == SettingsSection::Appearance {
                    self.accent_extras(ui, settings, ev);
                }
                if self.section == SettingsSection::GalaxyMap {
                    ui.add_space(6.0);
                    widgets::hint(
                        ui,
                        "How many stars the Galaxy Map loads and draws (1,000 – 100,000 each). Near focus: every \
                         system closest to where you look, with names. Sampled around: an even sample of the wider \
                         area. More fills the view better but loads slower (about 1 MB per 10,000 stars).",
                    );
                }
                ui.add_space(8.0);
            });
    }

    /// One labelled setting: the label column, its control, and the error under it when the
    /// last edit of this row failed.
    #[allow(clippy::too_many_arguments)]
    fn field_row(&mut self, ui: &mut Ui, idx: usize, label: &str, values: Vec<CellType>, moved: bool, settings: &mut Settings, ev: &mut ViewEvent) {
        let graphics_option = if self.section == SettingsSection::GameGraphics { crate::game_graphics::option_by_label(label) } else { None };
        let cursor = self.cursor_on(idx);
        let bg = ui.painter().add(Shape::Noop);
        let mut notes_below = false;
        let rect = ui
            .horizontal(|ui| {
                ui.set_min_height(ROW_H);
                let (lr, lresp) = ui.allocate_exact_size(vec2(LABEL_W, ROW_H), Sense::click());
                let sub = label.starts_with("  ");
                let col = if cursor && self.col == 0 { accent_hi() } else if sub { TEXT_DIM } else { CYAN };
                let x = lr.left() + if sub { 26.0 } else { 10.0 };
                if sub {
                    ui.painter().line_segment([pos2(lr.left() + 12.0, lr.top() + 4.0), pos2(lr.left() + 12.0, lr.center().y)], Stroke::new(1.0, accent_mute()));
                    ui.painter().line_segment([pos2(lr.left() + 12.0, lr.center().y), pos2(lr.left() + 20.0, lr.center().y)], Stroke::new(1.0, accent_mute()));
                }
                // The grid label spells out the accepted formats; here the field's
                // placeholder and the hint under the swatches carry them.
                let shown = label.trim().trim_end_matches(" %").trim_end_matches(" (%)").trim_end_matches(" (name, #rrggbb, or r,g,b)");
                ui.painter().text(pos2(x, lr.center().y), Align2::LEFT_CENTER, shown, theme::body(15.5), col);
                if lresp.clicked() {
                    keep(ev, self.finish_open_edit(settings));
                    self.feedback = None;
                    self.select_cell(idx, 1);
                }
                if lresp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                for (i, cell) in values.into_iter().enumerate() {
                    self.value_cell(ui, idx, i + 1, label, cell, settings, ev);
                }
                if let Some(o) = graphics_option {
                    if o.choices.is_empty() {
                        ui.add_space(8.0);
                        widgets::hint(ui, &format!("game {} · {}", o.game, o.kind.range()));
                    }
                    ui.add_space(12.0);
                    let room = ui.available_width() - 10.0;
                    if room >= NOTES_MIN_W {
                        option_notes(ui, o, room);
                    } else {
                        notes_below = true;
                    }
                }
            })
            .response
            .rect;
        if let (Some(o), true) = (graphics_option, notes_below) {
            ui.horizontal(|ui| {
                ui.add_space(LABEL_W);
                let room = ui.available_width() - 10.0;
                option_notes(ui, o, room);
            });
        }
        if cursor {
            let r = Rect::from_min_max(pos2(rect.left(), rect.top() - 1.0), pos2(ui.max_rect().right(), rect.bottom() + 1.0));
            ui.painter().set(bg, Shape::rect_filled(r, 0.0, alpha(accent(), 22)));
            ui.painter().rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 0.0, accent());
            if moved {
                ui.scroll_to_rect(r, None);
            }
            if let Some(msg) = &self.feedback {
                ui.horizontal(|ui| {
                    ui.add_space(LABEL_W);
                    ui.label(RichText::new(format!("⚠ {msg}")).color(RED).font(theme::body_semi(15.0)));
                });
            }
        }
    }

    /// The control for a row's value cell, picked by what the setting is.
    #[allow(clippy::too_many_arguments)]
    fn value_cell(&mut self, ui: &mut Ui, idx: usize, col: usize, label: &str, cell: CellType, settings: &mut Settings, ev: &mut ViewEvent) {
        let cursor = self.cursor_on(idx) && self.col == col;
        match cell {
            CellType::StringValue(value) => match label {
                "Action at Shutdown" => {
                    let current = SHUTDOWN_ACTIONS.iter().position(|a| *a == value).unwrap_or(2);
                    let resp = ui.scope(|ui| widgets::tab_strip(ui, &SHUTDOWN_ACTIONS, current));
                    if let Some(i) = resp.inner {
                        if i != current {
                            keep(ev, self.commit_value(idx, col, SHUTDOWN_ACTIONS[i].to_string(), settings));
                        }
                    }
                    let what = match current {
                        0 => "exit EDCAS",
                        1 => "stop reading",
                        _ => "keep watching for the next session",
                    };
                    resp.response.on_hover_text(format!("What the journal reader does when the game shuts down: {what}"));
                    if ui.available_width() > 240.0 {
                        ui.add_space(8.0);
                        widgets::hint(ui, what);
                    }
                }
                "  Voice Volume %" => {
                    let v = settings.alerts.voice_volume;
                    self.percent_slider(ui, idx, v, "Volume of spoken alerts — drag, or click the number to type", settings, ev);
                }
                "  Tint Mode" => {
                    let modes = super::TINT_MODES;
                    let current = modes.iter().position(|m| *m == value).unwrap_or(0);
                    let resp = ui.scope(|ui| widgets::tab_strip(ui, &modes, current));
                    if let Some(i) = resp.inner {
                        if i != current {
                            keep(ev, self.commit_value(idx, col, modes[i].to_string(), settings));
                        }
                    }
                    resp.response.on_hover_text("Per planet: after what the planet you are at is made of (EDCAS has to run). Fixed colour: one colour on every planet, kept when EDCAS closes.");
                }
                "  Tint Colour" => {
                    let stored = crate::theme::parse_rgb(&value).unwrap_or((255, 255, 255));
                    self.colour_cell(ui, idx, col, &value, stored, 110.0, "#rrggbb or r,g,b", false, GAME_COLOUR_NOTE, settings, ev);
                    ui.add_space(10.0);
                    self.swatches(ui, idx, &value, &TINT_PRESETS, settings, ev);
                }
                "  Hue Shift (°)" => {
                    let v = settings.game_graphics.hud_hue_shift;
                    self.ranged_slider(ui, idx, v, 0..=359, "°", "How far the HUD's colours are turned around the colour circle: 180 takes Elite's orange to blue, 120 to green, 270 to purple. Hostile red on the radar and the holograms keep their colours — drag, or click the number to type", settings, ev);
                }
                "  HUD Brightness (%)" => {
                    let v = settings.game_graphics.hud_brightness;
                    self.ranged_slider(ui, idx, v, HUD_BRIGHTNESS_RANGE, " %", "How bright the HUD is, 100 % as Elite draws it (on top of the game's own HUD brightness) — drag, or click the number to type", settings, ev);
                }
                _ if super::GAUGE_ROWS.iter().any(|(look, _, _)| *look == label) => {
                    let looks = super::GAUGE_LOOKS;
                    let current = looks.iter().position(|l| *l == value).unwrap_or(0);
                    let resp = ui.scope(|ui| widgets::tab_strip(ui, &looks, current));
                    if let Some(i) = resp.inner {
                        if i != current {
                            keep(ev, self.commit_value(idx, col, looks[i].to_string(), settings));
                        }
                    }
                    resp.response.on_hover_text("Elite: as the game draws it. Spectrum: the colours of the rainbow along the gauge. Colour: one colour of your own. Each pixel stays as bright as Elite drew it.");
                }
                _ if super::HUD_PART_ROWS.iter().any(|(look, _, _)| *look == label) => {
                    let looks = super::PART_LOOKS;
                    let current = looks.iter().position(|l| *l == value).unwrap_or(0);
                    let resp = ui.scope(|ui| widgets::tab_strip(ui, &looks, current));
                    if let Some(i) = resp.inner {
                        if i != current {
                            keep(ev, self.commit_value(idx, col, looks[i].to_string(), settings));
                        }
                    }
                    resp.response.on_hover_text("Elite: as the game draws it. Turned: its colours turned by the hue shift, so what Elite draws in two colours stays two. Colour: one colour of your own for all of it. Each pixel stays as bright as Elite drew it, times the HUD brightness.");
                }
                _ if super::HUD_PART_ROWS.iter().any(|(_, colour, _)| *colour == label) => {
                    // One colour, or two that fade into each other across the picture.
                    let (first, second) = super::parse_fade(&value).unwrap_or_default();
                    let stored = crate::theme::parse_rgb(&first).unwrap_or((255, 255, 255));
                    self.fade_cell(ui, idx, col, &value, stored, crate::theme::parse_rgb(&second), 190.0, "#rrggbb, or two with > to fade", GAME_FADE_NOTE, settings, ev);
                    ui.add_space(10.0);
                    self.swatches(ui, idx, &first, &GAUGE_PRESETS, settings, ev);
                }
                _ if super::HUD_PART_ROWS.iter().any(|(_, _, fade)| *fade == label) => {
                    let ways = super::FADE_WAYS;
                    let current = ways.iter().position(|w| *w == value).unwrap_or(0);
                    let resp = ui.scope(|ui| widgets::tab_strip(ui, &ways, current));
                    if let Some(i) = resp.inner {
                        if i != current {
                            keep(ev, self.commit_value(idx, col, ways[i].to_string(), settings));
                        }
                    }
                    resp.response.on_hover_text("Across: the first colour at the screen's left edge, the second at its right one. Up: the first at the bottom, the second at the top. Swap the two colours for the other direction.");
                }
                _ if super::HOLOGRAM_ROWS.iter().any(|(_, colour)| *colour == label) || super::GAUGE_ROWS.iter().any(|(_, colour, _)| *colour == label) => {
                    let stored = crate::theme::parse_rgb(&value).unwrap_or((255, 255, 255));
                    self.colour_cell(ui, idx, col, &value, stored, 110.0, "#rrggbb or r,g,b", false, GAME_COLOUR_NOTE, settings, ev);
                    ui.add_space(10.0);
                    self.swatches(ui, idx, &value, &GAUGE_PRESETS, settings, ev);
                }
                "  Tint Strength %" => {
                    let v = settings.game_graphics.ground_tint_strength;
                    self.percent_slider(ui, idx, v, "How far the ground's colour moves towards ice or metal — drag, or click the number to type", settings, ev);
                }
                "  Detail Strength (%)" => {
                    let v = settings.game_graphics.ground_detail_strength;
                    self.ranged_slider(ui, idx, v, 0..=MAX_LOOK_PERCENT, " %", "How strong the ground detail's colour variation and relief are, 100 % as designed (0 switches it off) — drag, or click the number to type", settings, ev);
                }
                "  Cloud Strength %" => {
                    let v = settings.game_graphics.planet_clouds_strength;
                    self.percent_slider(ui, idx, v, "How visible the clouds are — drag, or click the number to type", settings, ev);
                }
                "  Cloud Thickness (%)" => {
                    let v = settings.game_graphics.planet_cloud_thickness;
                    self.ranged_slider(ui, idx, v, 0..=MAX_LOOK_PERCENT, " %", "How thick the cloud slab is, 100 % as worked out for the planet (only with volumetric clouds; 0 is the thin shell) — drag, or click the number to type", settings, ev);
                }
                "  Cloud Edge Softness (%)" => {
                    let v = settings.game_graphics.planet_cloud_softness;
                    self.ranged_slider(ui, idx, v, 0..=MAX_LOOK_PERCENT, " %", "How soft the clouds' edges are, 100 % the usual — drag, or click the number to type", settings, ev);
                }
                "  Fog Density (%)" => {
                    let v = settings.game_graphics.planet_fog_density;
                    self.ranged_slider(ui, idx, v, 0..=MAX_LOOK_PERCENT, " %", "How dense the ground fog is, 100 % as worked out for the planet — drag, or click the number to type", settings, ev);
                }
                "  Fog Depth (m)" => {
                    let v = settings.game_graphics.planet_fog_depth;
                    self.ranged_slider(ui, idx, v, FOG_DEPTH_RANGE, " m", "How deep the ground fog is, in metres — drag, or click the number to type", settings, ev);
                }
                "  Dust Strength (%)" => {
                    let v = settings.game_graphics.planet_dust_strength;
                    self.ranged_slider(ui, idx, v, 0..=MAX_LOOK_PERCENT, " %", "How thick the wind-blown dust is, 100 % as the planet's air works out (none on airless planets) — drag, or click the number to type", settings, ev);
                }
                "  Dust Reach (km)" => {
                    let v = settings.game_graphics.planet_dust_reach_km;
                    self.ranged_slider(ui, idx, v, DUST_REACH_RANGE, " km", "How far away the dust drifting over the land is drawn, in kilometres (the dust close by always reaches 400 m) — drag, or click the number to type", settings, ev);
                }
                "  Lamp Glow in Fog (%)" => {
                    let v = settings.game_graphics.planet_lamp_glow;
                    self.ranged_slider(ui, idx, v, 0..=MAX_LOOK_PERCENT, " %", "How strongly the ship's lights, the suit torch and other lamps light the fog, 100 % the physical glow (white at night in Elite's exposure; 0 is none) — drag, or click the number to type", settings, ev);
                }
                "  Star Light Strength (%)" => {
                    let v = settings.game_graphics.star_light_strength();
                    self.ranged_slider(ui, idx, v, 0..=MAX_LOOK_PERCENT, " %", &format!("How far the light goes towards the star's own colour, 100 % its colour in full as a camera records it (0 is the game's white light). EDCAS's default is {} %: left there, it follows the default when a release changes it — drag, or click the number to type", crate::settings::game_graphics::STAR_LIGHT_STRENGTH_DEFAULT), settings, ev);
                }
                "  Canopy Frost Strength (%)" => {
                    let v = settings.game_graphics.planet_canopy_frost_strength;
                    self.ranged_slider(ui, idx, v, 0..=MAX_LOOK_PERCENT, " %", "How strong the frost on the cockpit glass is in cloud and fog, 100 % the usual (0 is practically none) — drag, or click the number to type", settings, ev);
                }
                "  Bio Threshold" | "  Geo Threshold" => self.number_field(ui, idx, label, &value, settings, ev),
                _ if self.section == SettingsSection::Appearance => {
                    let stored = crate::theme::parse_rgb(&value).unwrap_or_else(crate::theme::accent_rgb);
                    self.colour_cell(ui, idx, col, &value, stored, 180.0, "name, #rrggbb or r,g,b", true, "", settings, ev);
                }
                _ if self.section == SettingsSection::GameGraphics && crate::game_graphics::option_by_label(label).is_some_and(|o| !o.choices.is_empty()) => {
                    if let Some(o) = crate::game_graphics::option_by_label(label) {
                        self.choice_cell(ui, idx, col, o, &value, settings, ev);
                    }
                }
                _ => {
                    let option = crate::game_graphics::option_by_label(label).filter(|_| self.section == SettingsSection::GameGraphics);
                    let width = match (self.section, option) {
                        (_, Some(_)) => 150.0,
                        (SettingsSection::Appearance, _) => 220.0,
                        _ => (ui.available_width() - 14.0).min(620.0),
                    };
                    let hint = match (label, option) {
                        (_, Some(o)) => o.suggested,
                        ("Journal Directory", _) => "folder holding Journal.*.log",
                        ("edcas API URL", _) => "https://edcas.de",
                        ("EDDN URL", _) => "https://eddn.edcd.io:4430/upload/",
                        ("Graphics Folder", _) => "found next to the journal folder",
                        ("Game Folder", _) => "…/Products/elite-dangerous-odyssey-64, found from the journal folder",
                        _ => "name, #rrggbb or r,g,b",
                    };
                    self.text_cell(ui, idx, col, &value, width, hint, settings, ev);
                }
            },
            CellType::BoolValue(on) => {
                if switch(ui, on, cursor).on_hover_text("Click to toggle (space)").clicked() {
                    keep(ev, self.press_cell(idx, col, settings));
                }
            }
            CellType::Button(text) => {
                let name = text.trim_start_matches("[ ").trim_end_matches(" ]");
                let tip = match text {
                    "[ Upload All Journal Logs ]" => "Uploads every journal file in the journal folder to the edcas API (space)",
                    "[ Play Sample ]" => "Speaks a sample alert at the current volume (space)",
                    "[ Restore Original File ]" => "Puts the game's override file back as it was before EDCAS changed it, and stops managing it (space)",
                    "[ Install ]" => "Downloads 3DMigoto from the edcas server and installs it with EDCAS's shaders into the game folder (space)",
                    "[ Update ]" => "Installs the shaders of this EDCAS version over the installed ones (close the game first on Windows) (space)",
                    "[ Finish Game Shaders ]" => "Makes the ground tint and star light shaders from the ones the game exported; restart the game after (space)",
                    "[ Repair ]" => "Puts EDCAS's d3d11.dll and its d3dx.ini lines back after another 3DMigoto install (EDHM's, or what EDHM left behind) replaced them; no download (close the game first on Windows) (space)",
                    "[ Remove ]" => "Removes EDCAS's shaders and puts the original d3d11.dll back if another mod still uses it; the rest of 3DMigoto stays (close the game first on Windows) (space)",
                    "[ Save Diagnostics ]" => "Writes one zip into EDCAS's log folder to send with a bug report: EDCAS's log, the game folder's d3dx.ini and 3DMigoto's log, the ShaderFixes file list with hashes, Status.json, Elite's graphics option files and these settings (no passwords or keys, no journal files) (space)",
                    "[ Copy Path ]" => "Copies the location of the diagnostics zip just saved, to paste into your bug report (space)",
                    "[ Copy ]" => "Copies the Steam launch option to the clipboard (space)",
                    _ => "Opens the newest journal file in your default app (space)",
                };
                let shader_job = matches!(text, "[ Install ]" | "[ Update ]" | "[ Finish Game Shaders ]" | "[ Repair ]" | "[ Remove ]");
                let diagnostics = text == "[ Save Diagnostics ]";
                let busy = (text == "[ Upload All Journal Logs ]" && self.bulk_upload_rx.is_some())
                    || (shader_job && self.shader_rx.is_some())
                    || (diagnostics && self.diagnostics_rx.is_some());
                let resp = widgets::hud_button_enabled(ui, name, !busy);
                if cursor {
                    ui.painter().rect_stroke(resp.rect.expand(2.0), 0.0, Stroke::new(1.0, accent_hi()), egui::StrokeKind::Outside);
                }
                if resp.on_hover_text(tip).clicked() {
                    keep(ev, self.press_cell(idx, col, settings));
                }
                if busy {
                    widgets::hint(ui, if shader_job || diagnostics { &self.shader_step } else { "upload running…" });
                }
            }
            _ => {}
        }
    }

    /// A graphics override's value as a row of picks (client 0.43.222; a text field before): the
    /// option's choices, the game's own marked, and the player's own value among them where it is
    /// none of them (taken over from a file made by hand). The one in force is lit while the
    /// override is on; picking one switches it on. From the keyboard Space or Enter takes the next.
    #[allow(clippy::too_many_arguments)]
    fn choice_cell(&mut self, ui: &mut Ui, row: usize, col: usize, o: &crate::game_graphics::GraphicsOption, value: &str, settings: &mut Settings, ev: &mut ViewEvent) {
        let tidy = |v: &str| o.kind.normalize(v).unwrap_or_else(|_| v.trim().to_string());
        let number = |v: &str| v.parse::<f64>().unwrap_or(f64::MAX);
        let mut picks: Vec<String> = o.choices.iter().map(|c| tidy(c)).collect();
        let stored = tidy(value);
        if !value.trim().is_empty() && !picks.contains(&stored) {
            let at = picks.iter().position(|p| number(p) > number(&stored)).unwrap_or(picks.len());
            picks.insert(at, stored.clone());
        }
        let on = settings.game_graphics.options.get(o.id).is_some_and(|c| c.enabled);
        let current = picks.iter().position(|p| on && *p == stored);
        let here = self.cursor_on(row) && self.col == col;
        if self.editing && here {
            self.edit_buffer = picks[current.map_or(0, |i| (i + 1) % picks.len())].clone();
            keep(ev, self.handle_key(&key(KeyCode::Enter), settings));
        }
        let game = tidy(o.game);
        let labels: Vec<String> = picks.iter().map(|p| if *p == game { format!("{p} · game") } else { p.clone() }).collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let strip = ui.scope(|ui| widgets::tab_strip(ui, &labels, current.unwrap_or(usize::MAX)));
        if here {
            ui.painter().rect_stroke(strip.response.rect.expand(2.0), 0.0, Stroke::new(1.0, accent_hi()), egui::StrokeKind::Outside);
        }
        if let Some(i) = strip.inner {
            keep(ev, self.commit_value(row, col, picks[i].clone(), settings));
        }
        // The notes beside the picks start at one place down the page.
        let used = strip.response.rect.width();
        if used < CHOICES_W {
            ui.add_space(CHOICES_W - used);
        }
    }

    /// A text cell. Shows the stored value until it is focused (by click, or Space/Enter on
    /// the keyboard cursor); then it edits `edit_buffer` in the view's edit mode.
    #[allow(clippy::too_many_arguments)]
    fn text_cell(&mut self, ui: &mut Ui, row: usize, col: usize, value: &str, width: f32, hint: &str, settings: &mut Settings, ev: &mut ViewEvent) {
        let id = Id::unique(("settings-cell", self.sidebar_focus, row, col));
        let editing_here = self.editing && self.cursor_on(row) && self.col == col;
        let invalid = editing_here && self.feedback.is_some();
        if editing_here {
            let resp = ui.add(text_edit(&mut self.edit_buffer, id, width, hint, invalid));
            let edge = if invalid { RED } else { accent() };
            ui.painter().rect_stroke(resp.rect, 0.0, Stroke::new(1.5, edge), egui::StrokeKind::Outside);
            let armed = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
            if !armed {
                // Started from the keyboard: take focus once, as the view enters edit mode.
                resp.request_focus();
                ui.ctx().data_mut(|d| d.insert_temp(id, true));
            }
            if resp.lost_focus() {
                let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
                keep(ev, self.handle_key(&key(if esc { KeyCode::Esc } else { KeyCode::Enter }), settings));
            }
        } else {
            let mut shown = value.to_owned();
            ui.add(text_edit(&mut shown, id, width, hint, false));
            ui.ctx().data_mut(|d| d.remove::<bool>(id));
            // Memory's focus, not `resp.has_focus()`: that one is also false while the window
            // itself is unfocused.
            if ui.ctx().memory(|m| m.has_focus(id)) {
                keep(ev, self.finish_open_edit(settings));
                self.feedback = None;
                self.select_cell(row, col);
                self.editing = true;
                self.edit_buffer = shown;
                ui.ctx().data_mut(|d| d.insert_temp(id, true));
            }
        }
    }

    /// A colour: the picker's button, then the value as text. A colour picked is shown while
    /// it is dragged and saved when the pointer lets go; `stored` is the colour `value` means.
    #[allow(clippy::too_many_arguments)]
    fn colour_cell(&mut self, ui: &mut Ui, row: usize, col: usize, value: &str, stored: colour_picker::Rgb, width: f32, hint: &str, brightness: bool, note: &str, settings: &mut Settings, ev: &mut ViewEvent) {
        let id = Id::unique(("settings-colour", self.sidebar_focus, row, col));
        let (resp, picked) = colour_picker::button(ui, id, stored, brightness, note);
        if resp.clicked() {
            keep(ev, self.finish_open_edit(settings));
            self.feedback = None;
            self.select_cell(row, col);
        }
        let shown = picked.map_or_else(|| value.to_owned(), |p| colour_picker::hex(p.rgb));
        if let Some(p) = picked.filter(|p| p.done) {
            keep(ev, self.commit_value(row, col, colour_picker::hex(p.rgb), settings));
        }
        self.text_cell(ui, row, col, &shown, width, hint, settings, ev);
    }

    /// As [`Self::colour_cell`] for a colour that can fade into a second one (`to`): the value
    /// is one colour, or `first > second`.
    #[allow(clippy::too_many_arguments)]
    fn fade_cell(&mut self, ui: &mut Ui, row: usize, col: usize, value: &str, stored: colour_picker::Rgb, to: Option<colour_picker::Rgb>, width: f32, hint: &str, note: &str, settings: &mut Settings, ev: &mut ViewEvent) {
        let id = Id::unique(("settings-colour", self.sidebar_focus, row, col));
        let (resp, picked) = colour_picker::fade_button(ui, id, stored, to, note);
        if resp.clicked() {
            keep(ev, self.finish_open_edit(settings));
            self.feedback = None;
            self.select_cell(row, col);
        }
        let text = |p: colour_picker::Picked| p.to.map_or_else(|| colour_picker::hex(p.rgb), |to| format!("{} > {}", colour_picker::hex(p.rgb), colour_picker::hex(to)));
        let shown = picked.map_or_else(|| value.to_owned(), text);
        if let Some(p) = picked.filter(|p| p.done) {
            keep(ev, self.commit_value(row, col, text(p), settings));
        }
        self.text_cell(ui, row, col, &shown, width, hint, settings, ev);
    }

    /// A 0–100 % slider, saved when the drag or typing ends.
    fn percent_slider(&mut self, ui: &mut Ui, idx: usize, stored: u8, hover: &str, settings: &mut Settings, ev: &mut ViewEvent) {
        self.ranged_slider(ui, idx, u16::from(stored), 0..=100, " %", hover, settings, ev);
    }

    /// A number in `range` as a slider with a typed-in value, saved when the drag or typing ends.
    #[allow(clippy::too_many_arguments)]
    fn ranged_slider(&mut self, ui: &mut Ui, idx: usize, stored: u16, range: std::ops::RangeInclusive<u16>, suffix: &str, hover: &str, settings: &mut Settings, ev: &mut ViewEvent) {
        let id = Id::unique(("settings-percent-drag", idx));
        let mut v = ui.ctx().data(|d| d.get_temp::<u16>(id)).unwrap_or(stored);
        ui.spacing_mut().slider_width = 220.0;
        let resp = ui.add(egui::Slider::new(&mut v, range).suffix(suffix).trailing_fill(true));
        if resp.dragged() {
            ui.ctx().data_mut(|d| d.insert_temp(id, v));
        } else {
            ui.ctx().data_mut(|d| d.remove::<u16>(id));
        }
        if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
            keep(ev, self.commit_value(idx, 1, v.to_string(), settings));
        }
        resp.on_hover_text(hover);
    }

    /// A threshold as a drag/typed number (0–255), saved when the drag or typing ends.
    fn number_field(&mut self, ui: &mut Ui, idx: usize, label: &str, value: &str, settings: &mut Settings, ev: &mut ViewEvent) {
        let id = Id::unique(("settings-number-drag", label.to_owned()));
        let stored: u8 = value.trim().parse().unwrap_or(0);
        let mut v = ui.ctx().data(|d| d.get_temp::<u8>(id)).unwrap_or(stored);
        let resp = ui.add(egui::DragValue::new(&mut v).range(0..=255).speed(0.1).update_while_editing(false));
        if resp.dragged() {
            ui.ctx().data_mut(|d| d.insert_temp(id, v));
        } else {
            ui.ctx().data_mut(|d| d.remove::<u8>(id));
        }
        if (resp.drag_stopped() || (resp.changed() && !resp.dragged())) && v != stored {
            keep(ev, self.commit_value(idx, 1, v.to_string(), settings));
        }
        resp.on_hover_text("Alert when a body has at least this many signals — drag or click to type");
        widgets::hint(ui, "signals");
    }

    /// Ground tint presets beside the colour field; a click picks one.
    fn swatches(&mut self, ui: &mut Ui, idx: usize, value: &str, presets: &[(&str, &str)], settings: &mut Settings, ev: &mut ViewEvent) {
        ui.spacing_mut().item_spacing.x = 6.0;
        let current = crate::theme::parse_rgb(value);
        for (name, hex) in presets.iter().copied() {
            let Some((r, g, b)) = crate::theme::parse_rgb(hex) else { continue };
            let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
            let p = ui.painter();
            p.rect_filled(rect.shrink(3.0), 0.0, Color32::from_rgb(r, g, b));
            if current == Some((r, g, b)) {
                p.rect_stroke(rect, 0.0, Stroke::new(2.0, TEXT), egui::StrokeKind::Inside);
            } else if resp.hovered() {
                p.rect_stroke(rect, 0.0, Stroke::new(1.0, TEXT_DIM), egui::StrokeKind::Inside);
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.on_hover_text(format!("{name} {hex}")).clicked() {
                keep(ev, self.commit_value(idx, 1, hex.to_string(), settings));
            }
        }
    }

    /// Appearance: preset swatches under the colour field, and the palette it produces.
    fn accent_extras(&mut self, ui: &mut Ui, settings: &mut Settings, ev: &mut ViewEvent) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(LABEL_W);
            ui.spacing_mut().item_spacing.x = 6.0;
            let current = crate::theme::accent_rgb();
            for name in ACCENTS {
                let Some((r, g, b)) = crate::theme::parse_rgb(name) else { continue };
                let col = Color32::from_rgb(r, g, b);
                let (rect, resp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
                let p = ui.painter();
                p.rect_filled(rect.shrink(3.0), 0.0, col);
                let chosen = (r, g, b) == current;
                if chosen {
                    p.rect_stroke(rect, 0.0, Stroke::new(2.0, TEXT), egui::StrokeKind::Inside);
                } else if resp.hovered() {
                    p.rect_stroke(rect, 0.0, Stroke::new(1.0, TEXT_DIM), egui::StrokeKind::Inside);
                }
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.on_hover_text(name).clicked() {
                    keep(ev, self.commit_value(0, 1, name.to_string(), settings));
                }
            }
        });
        ui.add_space(14.0);
        widgets::section(ui, "Preview", None);
        ui.horizontal(|ui| {
            ui.add_space(LABEL_W);
            for (name, col) in [("accent", accent()), ("bright", accent_hi()), ("dim", accent_dim()), ("idle", accent_mute())] {
                ui.vertical(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(72.0, 18.0), Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, col);
                    ui.label(RichText::new(name).color(TEXT_DIM).font(theme::body(13.5)));
                });
            }
        });
        ui.horizontal(|ui| {
            ui.add_space(LABEL_W);
            widgets::badge(ui, "Badge", accent());
            widgets::hud_button(ui, "Button").on_hover_text("Only a preview");
            widgets::bar(ui, 160.0, 0.62, accent());
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(LABEL_W);
            widgets::hint(ui, "Names: orange, purple, white, red, green, blue, yellow, cyan, magenta, teal — or #rrggbb, or r,g,b.");
        });
    }

    // ------------------------------------------------------------ icon sections

    fn icons_ui(&mut self, ui: &mut Ui, settings: &mut Settings, ev: &mut ViewEvent) {
        let grid = self.build_grid(settings);
        let moved = self.cursor_moved(ui);
        let count = grid.len();
        // Stars and planets are drawn as discs: only the icon list has symbols to edit.
        let symbols = self.section == SettingsSection::Icons;
        ui.horizontal(|ui| {
            let what = if symbols { "Symbol and colour save" } else { "Colours save" };
            widgets::hint(ui, &format!("{count} entries. {what} on Enter or when you click elsewhere; the switch saves at once."));
        });
        if let Some(msg) = &self.feedback {
            ui.label(RichText::new(format!("⚠ {msg}")).color(RED).font(theme::body_semi(15.0)));
        }
        ui.add_space(4.0);
        // The keyboard cursor's row is the table's selected row; keep its fill a tint.
        ui.visuals_mut().selection.bg_fill = alpha(accent(), 44);
        let mut table = TableBuilder::new(ui)
            .id_salt(("settings-icons", self.sidebar_focus))
            .striped(true)
            .auto_shrink([false, false])
            .cell_layout(egui::Layout::left_to_right(Align::Center))
            .column(Column::remainder().at_least(150.0).clip(true))
            .columns(Column::exact(90.0), usize::from(symbols))
            .column(Column::exact(190.0))
            .column(Column::exact(100.0))
            .column(Column::exact(64.0));
        if moved && matches!(self.focus, FocusArea::Content) {
            table = table.scroll_to_row(self.row, None);
        }
        table
            .header(24.0, |mut h| {
                for t in ["Key", "Symbol", "Colour", "Shown", "Preview"].into_iter().filter(|&t| symbols || t != "Symbol") {
                    h.col(|ui| {
                        widgets::th(ui, t);
                    });
                }
            })
            .body(|body| {
                body.rows(ROW_H, count, |mut row| {
                    let i = row.index();
                    let cells = grid[i].cells.clone();
                    let cursor = self.cursor_on(i);
                    row.set_selected(cursor);
                    // A row drawn as a shape has no symbol cell: its colour moves up a column.
                    let (name, ch, colour, on) = match cells.as_slice() {
                        [CellType::Label(n), CellType::StringValue(ch), CellType::StringValue(c), CellType::ToggleEnabled(on)] => (n, Some(ch), c, *on),
                        [CellType::Label(n), CellType::StringValue(c), CellType::ToggleEnabled(on)] => (n, None, c, *on),
                        _ => return,
                    };
                    let colour_col = if ch.is_some() { 2 } else { 1 };
                    row.col(|ui| {
                        let col = if cursor && self.col == 0 { accent_hi() } else { CYAN };
                        let resp = ui.add(egui::Label::new(RichText::new(name).color(col).font(theme::body(15.5))).sense(Sense::click()).truncate());
                        if resp.clicked() {
                            keep(ev, self.finish_open_edit(settings));
                            self.feedback = None;
                            self.select_cell(i, 1);
                        }
                    });
                    if symbols {
                        row.col(|ui| match ch {
                            Some(ch) => self.text_cell(ui, i, 1, ch, 64.0, "", settings, ev),
                            None => {
                                ui.label(RichText::new("—").color(GREY).font(theme::body(15.0))).on_hover_text("Drawn as a shape: only its colour is used");
                            }
                        });
                    }
                    row.col(|ui| {
                        let shown = if self.editing && self.cursor_on(i) && self.col == colour_col { self.edit_buffer.clone() } else { colour.clone() };
                        let c = icon_color(&shown);
                        self.colour_cell(ui, i, colour_col, colour, (c.r(), c.g(), c.b()), 120.0, "white", true, "", settings, ev);
                    });
                    row.col(|ui| {
                        if switch(ui, on, cursor && self.col == colour_col + 1).on_hover_text("Show this icon (space)").clicked() {
                            keep(ev, self.press_cell(i, colour_col + 1, settings));
                        }
                    });
                    row.col(|ui| {
                        let col = if on { icon_color(colour) } else { GREY };
                        match ch {
                            Some(ch) => {
                                ui.label(RichText::new(ch).color(col).font(theme::body(19.0)));
                            }
                            None => {
                                let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                                ui.painter().circle_filled(r.center(), 6.5, col);
                            }
                        }
                    });
                });
            });
    }

    // ------------------------------------------------------------ upload progress

    /// The bulk upload's progress, under the content of every section.
    fn progress_ui(&self, ui: &mut Ui) {
        let Some(prog) = &self.bulk_upload_progress else { return };
        ui.add_space(6.0);
        widgets::section(ui, "Journal upload", None);
        let frac = if prog.total_files == 0 { 0.0 } else { prog.current_file as f32 / prog.total_files as f32 };
        let (label, col) = if prog.done {
            (format!("Done — {} files, {} lines uploaded", prog.total_files, prog.lines_done), GREEN)
        } else if let Some(e) = &prog.error {
            (format!("Error: {e}"), RED)
        } else {
            (format!("Uploading file {} of {} — {} lines", prog.current_file, prog.total_files, prog.lines_done), accent())
        };
        ui.horizontal(|ui| {
            widgets::bar(ui, (ui.available_width() * 0.45).max(120.0), if prog.done { 1.0 } else { frac }, col);
            ui.label(RichText::new(label).color(col).font(theme::body_semi(15.0)));
        });
    }
}

// ---------------------------------------------------------------- free helpers

/// A single-line field in the HUD style with an explicit id, so it keeps focus while it
/// switches from showing the stored value to editing the buffer.
/// The least room beside a graphics override's control for its notes; with less they go under it
/// (a window some 1,600 wide and narrower): in a column narrower than this they ran to six lines.
const NOTES_MIN_W: f32 = 420.0;
/// The room a graphics override's picks take: the widest row's, six of them.
const CHOICES_W: f32 = 510.0;

/// What a graphics override does and what it costs, in `width`: two paragraphs that wrap.
fn option_notes(ui: &mut Ui, o: &crate::game_graphics::GraphicsOption, width: f32) {
    ui.allocate_ui_with_layout(vec2(width, ROW_H), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_max_width(width);
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.add_space(4.0);
        ui.add(egui::Label::new(RichText::new(o.help).color(TEXT_DIM).font(theme::body(14.0))).wrap());
        ui.add(egui::Label::new(RichText::new(o.cost).color(accent_mute()).font(theme::body(13.5))).wrap());
        ui.add_space(4.0);
    });
}

fn text_edit<'a>(value: &'a mut String, id: Id, width: f32, hint: &str, invalid: bool) -> egui::TextEdit<'a> {
    let mut te = egui::TextEdit::singleline(value)
        .id(id)
        .hint_text(RichText::new(hint).color(GREY))
        .font(theme::body(16.0))
        .text_color(TEXT)
        .desired_width(width)
        .margin(vec2(6.0, 3.0));
    if invalid {
        te = te.background_color(alpha(RED, 40));
    }
    te
}

/// An on/off switch: a track with a square knob and the state spelled out. Returns the click
/// response; `cursor` outlines it when the keyboard cursor is on it.
fn switch(ui: &mut Ui, on: bool, cursor: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(70.0, 20.0), Sense::click());
    let p = ui.painter();
    let track = Rect::from_min_size(pos2(rect.left(), rect.center().y - 9.0), vec2(36.0, 18.0));
    let acc = accent();
    let (fill, stroke) = if on { (acc, acc) } else { (alpha(GREY, 40), GREY) };
    let stroke = if resp.hovered() { accent_hi() } else { stroke };
    p.rect(track, 0.0, fill, Stroke::new(1.0, stroke), egui::StrokeKind::Inside);
    let knob_x = if on { track.right() - 16.0 } else { track.left() + 2.0 };
    let knob = Rect::from_min_size(pos2(knob_x, track.top() + 2.0), vec2(14.0, 14.0));
    p.rect_filled(knob, 0.0, if on { BG } else { TEXT_DIM });
    let (txt, col) = if on { ("ON", accent_hi()) } else { ("OFF", GREY) };
    widgets::txt(p, pos2(track.right() + 8.0, rect.center().y + 0.5), Align2::LEFT_CENTER, txt, theme::head(9.0), col, 1.2);
    if cursor {
        p.rect_stroke(track.expand(3.0), 0.0, Stroke::new(1.0, accent_hi()), egui::StrokeKind::Outside);
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// The colour an icon is drawn in, from the names and `#rrggbb` the Explorer understands
/// (plus `r,g,b`), mapped onto the HUD palette like other text colours.
fn icon_color(spec: &str) -> Color32 {
    use crate::text::Color as C;
    let c = match spec.trim().to_lowercase().as_str() {
        "white" => C::White,
        "black" => C::Black,
        "red" => C::Red,
        "green" => C::Green,
        "blue" => C::Blue,
        "yellow" => C::Yellow,
        "cyan" => C::Cyan,
        "magenta" => C::Magenta,
        "gray" | "grey" => C::Gray,
        "darkgray" | "darkgrey" => C::DarkGray,
        "lightred" => C::LightRed,
        "lightgreen" => C::LightGreen,
        "lightyellow" => C::LightYellow,
        "lightblue" => C::LightBlue,
        "lightmagenta" => C::LightMagenta,
        "lightcyan" => C::LightCyan,
        s => match crate::theme::parse_rgb(s) {
            Some((r, g, b)) => C::Rgb(r, g, b),
            None => C::White,
        },
    };
    theme::hud_color(c)
}

/// The About page: the logo, then the shared help text with its headings as sections and
/// its key/description lines as a two-column grid.
fn about_ui(ui: &mut Ui) {
    let lines = crate::views::about::build_lines();
    egui::ScrollArea::vertical().id_salt("settings-about").auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(ui.available_width().min(900.0));
        ui.add_space(4.0);
        widgets::logo_header(ui, logo_texture(ui.ctx()).as_ref());
        ui.add_space(8.0);

        // Where this run's log is, for bug reports.
        if let Some(log) = crate::settings::log_file() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Log").color(accent_hi()).font(theme::body_semi(15.0)));
                ui.label(RichText::new(log.display().to_string()).color(TEXT).font(theme::body(15.0)));
                if let Some(dir) = log.parent() {
                    if ui.button("Open folder").clicked() {
                        if let Err(e) = opener::open(dir) {
                            tracing::warn!("Couldn't open the log folder {}: {e}", dir.display());
                        }
                    }
                }
            });
            ui.label(RichText::new("The last run's log is kept next to it as log.previous.log.").color(TEXT_DIM).font(theme::body(13.0)));
            ui.add_space(8.0);
        }

        let mut i = 0;
        let mut grid_n = 0;
        while i < lines.len() {
            let line = &lines[i];
            let is_ctrl = |l: &crate::text::Line| l.spans.len() == 2 && l.spans[0].content.starts_with("    ");
            if is_ctrl(line) {
                let end = (i..lines.len()).find(|&j| !is_ctrl(&lines[j])).unwrap_or(lines.len());
                egui::Grid::new(("about-keys", grid_n)).num_columns(2).spacing([18.0, 4.0]).show(ui, |ui| {
                    for l in &lines[i..end] {
                        ui.horizontal(|ui| {
                            ui.add_space(14.0);
                            ui.set_min_width(150.0);
                            ui.label(RichText::new(l.spans[0].content.trim()).color(accent_hi()).font(theme::body_semi(15.0)));
                        });
                        ui.label(RichText::new(l.spans[1].content.as_str()).color(TEXT).font(theme::body(15.0)));
                        ui.end_row();
                    }
                });
                grid_n += 1;
                i = end;
                continue;
            }
            let text = line.spans.iter().map(|s| s.content.as_str()).collect::<String>();
            let bold = line.spans.first().is_some_and(|s| s.style.bold);
            if text.trim().is_empty() {
                ui.add_space(6.0);
            } else if bold && !text.starts_with(' ') {
                ui.add_space(4.0);
                widgets::section(ui, &text, None);
            } else if bold {
                ui.label(RichText::new(text.trim()).color(CYAN).font(theme::body_semi(15.0)));
            } else {
                ui.label(widgets::line_job(line, &theme::body(15.0)));
            }
            i += 1;
        }
        ui.add_space(8.0);
    });
}

/// The EDCAS logo as a texture, decoded once and kept in egui's memory.
fn logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = Id::unique("settings-about-logo");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    let icon = eframe::icon_data::from_png_bytes(LOGO_PNG).ok()?;
    let img = egui::ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
    let tex = ctx.load_texture("settings-about-logo", img, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}
