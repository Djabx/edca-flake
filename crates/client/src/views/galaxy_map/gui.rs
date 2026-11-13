//! Desktop-window drawing for [`GalaxyMapView`]: search and the picked system on the left,
//! the 3D map in the middle, and the picked system's bodies (the Explorer's tree and
//! details) on the right.
//!
//! The stars are drawn by the GPU through a paint callback ([`super::stars`]); the grid,
//! markers and labels are ordinary egui shapes laid over them, projected by the same camera.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use eframe::egui::{self, pos2, vec2, Align2, Color32, CursorIcon, Key, Painter, PointerButton, Pos2, Rect, Sense, Shape, Stroke, Ui, UiBuilder};
use eframe::egui_glow;
use edcas_common::format::{abbreviate, fmt_ts_opt, group_thousands};

use super::camera::{self, Camera, Projector, V3};
use super::stars::{self, StarRenderer};
use super::{galaxy_view, GalaxyMapView, Load, DETAIL_RANGE, SAMPLE_RANGE};
use crate::api_client::ApiClient;
use crate::gui::theme::{self, accent, accent_dim, accent_hi, alpha, mix, BG, CYAN, GREEN, GREY, RED, TEXT, TEXT_DIM};
use crate::gui::widgets::{self, txt};
use crate::journal_reader::JournalData;
use crate::settings::Settings;
use crate::views::explorer::pretty;
use crate::views::ViewEvent;

/// The map's ground, a touch bluer than the panels.
const SPACE: Color32 = Color32::from_rgb(3, 4, 8);
/// Populated systems (matches the GPU colour).
const POPULATED: Color32 = Color32::from_rgb(255, 163, 71);
/// Unpopulated systems (matches the GPU colour).
const UNPOPULATED: Color32 = Color32::from_rgb(189, 214, 255);
/// Pointer distance, in pixels, within which a star counts as under it.
const PICK_PX: f32 = 9.0;
/// Most system names written next to their stars at once.
const MAX_LABELS: usize = 28;

/// Fixed places to find one's way at galaxy scale.
const LANDMARKS: [(&str, V3); 4] = [
    ("Sol", [0.0, 0.0, 0.0]),
    ("Sagittarius A*", [25.21875, -20.90625, 25899.97]),
    ("Colonia", [-9530.5, -910.28125, 19808.125]),
    ("Beagle Point", [-1111.5625, -134.21875, 65269.75]),
];

#[derive(Default)]
pub(super) struct GuiState {
    renderer: Arc<Mutex<StarRenderer>>,
    /// Index into the nearby systems of the star under the pointer.
    pub(super) hovered: Option<usize>,
}

/// What a click in the side panels asked for, applied once the panel is drawn.
enum Act {
    Pick(usize),
    Submit,
    Edited,
    CenterOnMe,
    WholeGalaxy,
    FlyToSelected,
    Deselect,
    Copy(String),
}

impl GalaxyMapView {
    pub fn ui(&mut self, ui: &mut Ui, api: &ApiClient, settings: &Settings, journal: &JournalData) -> ViewEvent {
        let now = Instant::now();
        self.nearby_limit = settings.galaxy_map.nearby_stars;
        self.sample_limit = settings.galaxy_map.sampled_stars;
        // Until the user moves the camera, it heads for the commander as soon as the journal
        // says where they are.
        if !self.placed {
            if let Some(pos) = Self::commander_pos(journal) {
                self.fly_to(pos, 80.0);
            }
        }
        self.step_flight(now);

        let full = ui.available_rect_before_wrap();
        let gap = 10.0;
        let left_w = (full.width() * 0.22).clamp(250.0, 340.0);
        let right_w = if self.selected.is_some() { (full.width() * 0.26).clamp(280.0, 420.0) } else { 0.0 };
        let left = Rect::from_min_size(full.min, vec2(left_w, full.height()));
        let map_right = if right_w > 0.0 { full.right() - right_w - gap } else { full.right() };
        let map_rect = Rect::from_min_max(pos2(left.right() + gap, full.top()), pos2(map_right, full.bottom()));

        let mut acts = Vec::new();
        self.left_column(ui, left, journal, &mut acts);
        self.map_panel(ui, map_rect, api, journal);
        if right_w > 0.0 {
            let right = Rect::from_min_max(pos2(map_right + gap, full.top()), full.max);
            let split = (right.top() + right.height() * 0.55).round();
            let tree = Rect::from_min_max(right.min, pos2(right.right(), split - gap / 2.0));
            let detail = Rect::from_min_max(pos2(right.left(), split + gap / 2.0), right.max);
            // Its own id, so the Explorer tab's scroll positions stay its own.
            let mut child = ui.new_child(UiBuilder::new().id_salt("galaxy-system-map").max_rect(right));
            self.system_map.system_map_ui(&mut child, tree, detail, settings, journal);
        }
        for act in acts {
            self.apply(act, api, journal);
        }

        let waits = [self.maybe_fetch_around(api), self.maybe_suggest(api)];
        if self.flight.is_some() {
            ui.ctx().request_repaint();
        } else if let Some(wait) = waits.into_iter().flatten().min() {
            ui.ctx().request_repaint_after(wait);
        }
        ui.allocate_rect(full, Sense::hover());
        ViewEvent::None
    }

    fn apply(&mut self, act: Act, api: &ApiClient, journal: &JournalData) {
        match act {
            Act::Pick(i) => self.pick_suggestion(api, i),
            Act::Submit => self.search_submitted(api),
            Act::Edited => self.search_edited(),
            Act::CenterOnMe => self.center_on_commander(journal),
            Act::WholeGalaxy => self.fly(galaxy_view()),
            Act::FlyToSelected => {
                if let Some(pos) = self.selected.as_ref().and_then(|s| s.pos) {
                    self.fly_to(pos, super::SYSTEM_VIEW_DISTANCE);
                }
            }
            Act::Deselect => self.selected = None,
            Act::Copy(text) => {
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_text(text);
                }
            }
        }
    }

    // ------------------------------------------------------------ left column

    fn left_column(&mut self, ui: &mut Ui, rect: Rect, journal: &JournalData, acts: &mut Vec<Act>) {
        let rows = self.suggestions.len() as f32;
        let status = if self.search_status.is_empty() { 0.0 } else { 22.0 };
        let search_h = (26.0 + 8.0 + 30.0 + 6.0 + 26.0 + rows * 24.0 + status + 14.0).min(rect.height() * 0.6);
        let search = Rect::from_min_size(rect.min, vec2(rect.width(), search_h));
        let below = Rect::from_min_max(pos2(rect.left(), search.bottom() + 10.0), rect.max);

        let me = Self::commander_pos(journal);
        widgets::hud_panel_at(ui, search, "Search", "/", |ui| {
            let resp = widgets::text_field(ui, &mut self.search, "System name…", ui.available_width())
                .on_hover_text("Type a system name; Enter takes the first match (/)");
            if std::mem::take(&mut self.focus_search) {
                resp.request_focus();
            }
            if resp.changed() {
                acts.push(Act::Edited);
            }
            if widgets::submitted(ui, &resp) {
                acts.push(Act::Submit);
            }
            ui.add_space(4.0);
            for (i, s) in self.suggestions.iter().enumerate() {
                let row = widgets::selectable_row(ui, false, 24.0, |ui| {
                    ui.label(egui::RichText::new(&s.name).font(theme::body_semi(15.0)).color(TEXT));
                });
                if row.on_hover_text("Show this system").clicked() {
                    acts.push(Act::Pick(i));
                }
            }
            if !self.search_status.is_empty() {
                let col = if self.search_status.starts_with("Search failed") { RED } else { GREY };
                ui.add(egui::Label::new(egui::RichText::new(&self.search_status).font(theme::body(14.0)).color(col)).truncate());
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if widgets::hud_button_enabled(ui, "Center on me", me.is_some()).on_hover_text("Fly to your current system (c)").clicked() {
                    acts.push(Act::CenterOnMe);
                }
                if widgets::hud_button(ui, "Galaxy").on_hover_text("Pull back to see the whole charted galaxy (g)").clicked() {
                    acts.push(Act::WholeGalaxy);
                }
            });
        });

        if self.selected.is_some() {
            self.system_panel(ui, below, journal, acts);
        } else {
            self.overview_panel(ui, below, journal);
        }
    }

    /// The picked system: where it is, how far, and what the server knows about it.
    fn system_panel(&self, ui: &mut Ui, rect: Rect, journal: &JournalData, acts: &mut Vec<Act>) {
        let Some(sel) = &self.selected else { return };
        let me = Self::commander_pos(journal);
        let here = journal.current_system.as_ref().is_some_and(|s| s.system_address == sel.address);
        widgets::hud_panel_at(ui, rect, "System", "esc · clear", |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(egui::RichText::new(&sel.name).font(theme::body_semi(21.0)).color(CYAN)).truncate());
            });
            ui.horizontal_wrapped(|ui| {
                if here {
                    widgets::badge(ui, "You are here", GREEN);
                }
                match sel.info.as_ref().and_then(|i| i.population) {
                    Some(p) if p > 0 => widgets::badge(ui, "Populated", POPULATED),
                    _ if sel.info.is_some() => widgets::badge(ui, "Unpopulated", TEXT_DIM),
                    _ => ui.label(""),
                };
            });
            ui.horizontal(|ui| {
                if widgets::hud_button_enabled(ui, "Fly to", sel.pos.is_some()).on_hover_text("Center the map on this system").clicked() {
                    acts.push(Act::FlyToSelected);
                }
                if widgets::hud_button(ui, "Copy name").on_hover_text("Copy the system name to the clipboard").clicked() {
                    acts.push(Act::Copy(sel.name.clone()));
                }
                if widgets::hud_button(ui, "✕").on_hover_text("Clear the selection (esc)").clicked() {
                    acts.push(Act::Deselect);
                }
            });
            ui.add_space(4.0);
            if sel.loading {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(13.0).color(accent()));
                    ui.label(egui::RichText::new("Loading system…").font(theme::body(14.0)).color(GREY));
                });
            }
            if let Some(e) = &sel.error {
                ui.label(egui::RichText::new(format!("Couldn't load this system: {e}")).font(theme::body(14.0)).color(RED));
            }

            egui::ScrollArea::vertical().id_salt("galaxy-system").auto_shrink([false, false]).show(ui, |ui| {
                widgets::section(ui, "Position", None);
                egui::Grid::new("galaxy-pos").num_columns(2).spacing(vec2(12.0, 2.0)).show(ui, |ui| {
                    if let Some(p) = sel.pos {
                        widgets::kv(ui, "Coordinates", widgets::value(format!("{:.2} / {:.2} / {:.2}", p[0], p[1], p[2])));
                        if let Some(m) = me {
                            widgets::kv(ui, "From you", widgets::value(format!("{} ly", fmt_ly(camera::dist(p, m)))));
                        }
                        widgets::kv(ui, "From Sol", widgets::value(format!("{} ly", fmt_ly(camera::dist(p, [0.0; 3])))));
                    } else {
                        widgets::kv(ui, "Coordinates", widgets::value("—"));
                    }
                });
                let Some(info) = &sel.info else { return };
                ui.add_space(6.0);
                widgets::section(ui, "Overview", None);
                let opt = |v: &Option<String>| v.as_deref().filter(|s| !s.is_empty()).map(pretty).unwrap_or_else(|| "—".into());
                egui::Grid::new("galaxy-info").num_columns(2).spacing(vec2(12.0, 2.0)).show(ui, |ui| {
                    widgets::kv(ui, "Allegiance", widgets::value(opt(&info.allegiance)));
                    widgets::kv(ui, "Government", widgets::value(opt(&info.government)));
                    let mut economy = opt(&info.economy);
                    let second = opt(&info.second_economy);
                    if second != "—" && second != "None" {
                        economy = format!("{economy} / {second}");
                    }
                    widgets::kv(ui, "Economy", widgets::value(economy));
                    widgets::kv(ui, "Security", widgets::value(opt(&info.security)));
                    widgets::kv(ui, "Population", widgets::value(info.population.map(abbreviate).unwrap_or_else(|| "—".into())));
                    if info.controlling_power.as_deref().is_some_and(|p| !p.is_empty()) {
                        widgets::kv(ui, "Power", widgets::value(opt(&info.controlling_power)));
                    }
                    widgets::kv(ui, "Updated", widgets::value(fmt_ts_opt(info.updated_at)));
                });
                if !info.factions.is_empty() {
                    ui.add_space(6.0);
                    widgets::section(ui, "Factions", Some(info.factions.len()));
                    for f in &info.factions {
                        faction_row(ui, &f.name, f.influence);
                    }
                }
                ui.add_space(8.0);
            });
        });
    }

    /// With nothing picked: what the map shows and how to drive it.
    fn overview_panel(&self, ui: &mut Ui, rect: Rect, journal: &JournalData) {
        widgets::hud_panel_at(ui, rect, "Galaxy", "", |ui| {
            egui::ScrollArea::vertical().id_salt("galaxy-overview").auto_shrink([false, false]).show(ui, |ui| {
                egui::Grid::new("galaxy-stats").num_columns(2).spacing(vec2(12.0, 2.0)).show(ui, |ui| {
                    let known = if self.total_systems > 0 { group_thousands(self.total_systems) } else { "—".into() };
                    widgets::kv(ui, "Known systems", widgets::value(known));
                    widgets::kv(ui, "Drawn singly", widgets::value(group_thousands(self.nearby.systems.len() as i64)));
                    if let Some(sys) = &journal.current_system {
                        widgets::kv(ui, "You are in", widgets::value(&sys.name));
                        if let Some(p) = sys.star_pos {
                            widgets::kv(ui, "Coordinates", widgets::value(format!("{:.1} / {:.1} / {:.1}", p[0], p[1], p[2])));
                        }
                    }
                });
                ui.add_space(6.0);
                widgets::section(ui, "Legend", None);
                legend_row(ui, POPULATED, "Populated system");
                legend_row(ui, UNPOPULATED, "Unpopulated system");
                legend_row(ui, mix(Color32::from_rgb(77, 107, 217), Color32::from_rgb(255, 219, 168), 0.6), "Charted space (density)");
                legend_row(ui, GREEN, "You");
                legend_row(ui, CYAN, "Selected system");
                ui.add_space(6.0);
                widgets::section(ui, "Controls", None);
                egui::Grid::new("galaxy-controls").num_columns(2).spacing(vec2(12.0, 2.0)).show(ui, |ui| {
                    for (k, v) in [
                        ("Drag", "Rotate"),
                        ("Right-drag", "Pan (or shift-drag)"),
                        ("W A S D", "Move (shift: faster)"),
                        ("Wheel", "Zoom"),
                        ("Click", "Select a star"),
                        ("Double-click", "Fly to a star"),
                        ("c / g", "Center on you / whole galaxy"),
                    ] {
                        widgets::kv(ui, k, widgets::value(v));
                    }
                });
                ui.add_space(6.0);
                widgets::hint(ui, "Single systems appear once you zoom in; farther out the cloud shows how densely each region is charted.");
            });
        });
    }

    // ------------------------------------------------------------ map

    fn map_panel(&mut self, ui: &mut Ui, rect: Rect, api: &ApiClient, journal: &JournalData) {
        let caption = if self.total_systems > 0 { format!("{} systems", group_thousands(self.total_systems)) } else { String::new() };
        let canvas = widgets::paint_hud_frame(ui.painter(), rect, "Galaxy Map", &caption).expand2(vec2(6.0, 4.0));
        let resp = ui.interact(canvas, ui.id().with("galaxy-canvas"), Sense::click_and_drag());

        // ---- input
        if resp.dragged() {
            let d = resp.drag_delta();
            let pan = resp.dragged_by(PointerButton::Secondary)
                || resp.dragged_by(PointerButton::Middle)
                || ui.input(|i| i.modifiers.shift);
            if pan {
                self.camera.pan(d.x, d.y, canvas.height());
            } else {
                self.camera.orbit(d.x, d.y);
            }
            self.touched();
        }
        // WASD flies across the galactic plane while held (not while typing in a field):
        // most of a view's width per second, three times that with shift.
        if !ui.ctx().egui_wants_keyboard_input() {
            let (forward, right, dt, fast) = ui.input(|i| {
                let axis = |pos: Key, neg: Key| i.key_down(pos) as i32 as f32 - i.key_down(neg) as i32 as f32;
                (axis(Key::W, Key::S), axis(Key::D, Key::A), i.stable_dt.min(0.1), i.modifiers.shift)
            });
            if forward != 0.0 || right != 0.0 {
                let speed = if fast { 2.4 } else { 0.8 } * dt;
                self.camera.travel(forward * speed, right * speed);
                self.touched();
                ui.ctx().request_repaint();
            }
        }
        if resp.hovered() {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            if scroll != 0.0 || pinch != 1.0 {
                self.camera.zoom((-scroll * 0.003).exp() / pinch);
                self.touched();
            }
        }

        let cam = self.camera;
        let proj = cam.projector(canvas);
        // A layer shows while its sphere fills a good part of the view; seen from farther,
        // its points would crowd into an overexposed ball, and the next layer takes over.
        let coverage = |radius: f32| ((radius / cam.distance - 0.2) / 0.3).clamp(0.0, 1.0);
        let stars_alpha = if self.nearby.systems.is_empty() { 0.0 } else { coverage(self.nearby.radius) };
        let show_stars = stars_alpha > 0.0;

        self.gui.hovered = match resp.hover_pos() {
            Some(p) if show_stars && !resp.dragged() => self.pick(&proj, p),
            _ => None,
        };
        if let Some(i) = self.gui.hovered {
            let s = &self.nearby.systems[i];
            let (address, name, pos) = (s.system_address, s.name.clone(), [s.x, s.y, s.z]);
            if resp.double_clicked() {
                self.select(api, address, name, Some(pos), true);
            } else if resp.clicked() {
                self.select(api, address, name, Some(pos), false);
            }
        }
        let cursor = if resp.dragged() {
            CursorIcon::Grabbing
        } else if self.gui.hovered.is_some() {
            CursorIcon::PointingHand
        } else if resp.hovered() {
            CursorIcon::Grab
        } else {
            CursorIcon::Default
        };
        if cursor != CursorIcon::Default {
            ui.ctx().set_cursor_icon(cursor);
        }

        // ---- drawing: ground and grid, then the GPU stars, then markers on top
        let painter = ui.painter_at(canvas);
        painter.rect_filled(canvas, 0.0, SPACE);
        let grid = paint_grid(&painter, &cam, canvas);

        // The sample is blended additively, so more points would only brighten it; dim each
        // as the count grows past the default and the region keeps its look.
        let thin = (20_000.0 / self.sampled.limit.max(1) as f32).sqrt().min(1.0);
        let sampled_alpha = if self.sampled.cloud.data.is_empty() { 0.0 } else { coverage(self.sampled.radius) * thin };
        let show_sampled = sampled_alpha > 0.0;
        let near_r = if show_stars { self.nearby.radius } else { 0.0 };
        let sample_r = if show_sampled { self.sampled.radius } else { 0.0 };
        let (nc, sc) = (self.nearby.center, self.sampled.center);
        let ppp = ui.ctx().pixels_per_point();
        let frame = stars::Frame {
            view_proj: cam.view_proj(canvas.width() / canvas.height()),
            px_per_ly: cam.px_per_ly(canvas.height()) * ppp,
            focus_depth: cam.distance,
            density: self.density.clone(),
            cell_ly: self.cell_ly,
            max_log: self.max_log,
            density_alpha: 1.0,
            near: [nc[0], nc[1], nc[2], near_r],
            stars: self.nearby.cloud.clone(),
            stars_alpha,
            sampled_sphere: [sc[0], sc[1], sc[2], sample_r],
            sampled: self.sampled.cloud.clone(),
            sampled_alpha,
        };
        let renderer = Arc::clone(&self.gui.renderer);
        painter.add(egui::PaintCallback {
            rect: canvas,
            callback: Arc::new(egui_glow::CallbackFn::new(move |_info, p| {
                renderer.lock().unwrap().paint(p.gl(), &frame);
            })),
        });

        let me = Self::commander_pos(journal);
        let marker_pos: Vec<Pos2> = [me, self.selected.as_ref().and_then(|s| s.pos)].into_iter().flatten().filter_map(|p| proj.project(p)).collect();
        self.paint_landmarks(&painter, &proj, &cam, &marker_pos);
        if show_stars && cam.distance < 180.0 {
            // The commander's and the selected system carry their own marker labels.
            let here = journal.current_system.as_ref().map(|s| s.system_address);
            let marked: Vec<i64> = here.into_iter().chain(self.selected.as_ref().map(|s| s.address)).collect();
            let markers: Vec<Rect> = [me, self.selected.as_ref().and_then(|s| s.pos)]
                .into_iter()
                .flatten()
                .filter_map(|p| proj.project(p))
                .map(|c| Rect::from_min_size(c + vec2(-14.0, -22.0), vec2(170.0, 42.0)))
                .collect();
            self.paint_labels(&painter, &proj, &cam, canvas, &marked, markers);
        }
        let sel = self.selected.as_ref().and_then(|s| s.pos.map(|p| (p, s.name.as_str())));
        if let (Some(a), Some((b, _))) = (me, sel) {
            if let Some([pa, pb]) = cam.project_segment(a, b, canvas) {
                painter.add(Shape::dashed_line(&[pa, pb], Stroke::new(1.0, alpha(CYAN, 150)), 6.0, 4.0));
                let mid = pa + (pb - pa) * 0.5;
                if canvas.contains(mid) && (pb - pa).length() > 90.0 {
                    label(&painter, mid + vec2(0.0, -10.0), Align2::CENTER_BOTTOM, &format!("{} ly", fmt_ly(camera::dist(a, b))), CYAN);
                }
            }
        }
        if let Some(p) = me {
            let name = journal.current_system.as_ref().map(|s| s.name.as_str()).unwrap_or("");
            marker(&painter, &proj, &cam, canvas, p, GREEN, "YOU", name, false);
        }
        if let Some((p, name)) = sel {
            marker(&painter, &proj, &cam, canvas, p, CYAN, "SELECTED", name, true);
        }
        if let Some(i) = self.gui.hovered {
            let s = &self.nearby.systems[i];
            if let Some(pp) = proj.project([s.x, s.y, s.z]) {
                painter.circle_stroke(pp, 7.0, Stroke::new(1.5, accent_hi()));
                let mut text = s.name.clone();
                if s.population > 0 {
                    text.push_str(&format!("  ·  pop {}", abbreviate(s.population)));
                }
                if let Some(m) = me {
                    text.push_str(&format!("  ·  {} ly from you", fmt_ly(camera::dist([s.x, s.y, s.z], m))));
                }
                tag(&painter, pp + vec2(12.0, -12.0), &text, TEXT, canvas);
            }
        }

        self.paint_hud(&painter, &cam, canvas, grid);
    }

    /// The nearby star closest to `pointer`, if within [`PICK_PX`].
    fn pick(&self, proj: &Projector, pointer: Pos2) -> Option<usize> {
        let mut best = None;
        let mut best_d = PICK_PX * PICK_PX;
        for (i, s) in self.nearby.systems.iter().enumerate() {
            if let Some(p) = proj.project([s.x, s.y, s.z]) {
                let d = (p - pointer).length_sq();
                if d < best_d {
                    best_d = d;
                    best = Some(i);
                }
            }
        }
        best
    }

    /// Names of the systems nearest the focus, skipping any that would overlap.
    /// `skip` are systems labelled elsewhere; `taken` is screen space already in use.
    fn paint_labels(&self, painter: &Painter, proj: &Projector, cam: &Camera, canvas: Rect, skip: &[i64], mut taken: Vec<Rect>) {
        let reach = cam.distance * 0.9;
        let mut near: Vec<(f32, usize)> = self
            .nearby
            .systems
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let d = camera::dist([s.x, s.y, s.z], cam.target);
                (d < reach).then_some((d, i))
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut written = 0;
        for (_, i) in near.into_iter().take(MAX_LABELS * 3) {
            if written >= MAX_LABELS {
                break;
            }
            let s = &self.nearby.systems[i];
            if skip.contains(&s.system_address) {
                continue;
            }
            let Some(p) = proj.project([s.x, s.y, s.z]) else { continue };
            if !canvas.shrink(20.0).contains(p) {
                continue;
            }
            let font = theme::body(13.0);
            let size = text_size(painter, &s.name, font.clone());
            let r = Rect::from_min_size(p + vec2(7.0, -size.y / 2.0), size).expand(2.0);
            if taken.iter().any(|t| t.intersects(r)) {
                continue;
            }
            taken.push(r);
            written += 1;
            let col = if s.population > 0 { mix(POPULATED, TEXT_DIM, 0.35) } else { TEXT_DIM };
            painter.text(p + vec2(7.0, 0.0), Align2::LEFT_CENTER, &s.name, font, alpha(col, 210));
        }
    }

    /// Named reference points at galaxy scale (up close the systems carry their own names).
    /// `markers` are the commander's and the selected system's, whose labels win a collision.
    fn paint_landmarks(&self, painter: &Painter, proj: &Projector, cam: &Camera, markers: &[Pos2]) {
        if cam.distance < 600.0 {
            return;
        }
        for (name, pos) in LANDMARKS {
            let Some(p) = proj.project(pos) else { continue };
            if markers.iter().any(|m| (*m - p).length() < 60.0) {
                continue;
            }
            let s = 4.0;
            let col = accent_hi();
            painter.line_segment([p - vec2(s, 0.0), p + vec2(s, 0.0)], Stroke::new(1.0, col));
            painter.line_segment([p - vec2(0.0, s), p + vec2(0.0, s)], Stroke::new(1.0, col));
            backed_caption(painter, p + vec2(8.0, -6.0), Align2::LEFT_BOTTOM, &name.to_uppercase(), col);
        }
    }

    /// Loading state, focus position and scale in the corners of the map.
    fn paint_hud(&self, painter: &Painter, cam: &Camera, canvas: Rect, grid_ly: f32) {
        let inset = canvas.shrink2(vec2(20.0, 14.0));
        let mut status: Vec<(String, Color32)> = Vec::new();
        match &self.density_load {
            Load::Loading | Load::Idle => status.push(("Loading the charted galaxy…".into(), TEXT_DIM)),
            Load::Failed(e) => status.push((format!("Galaxy overview unavailable: {e}"), RED)),
            Load::Ready => {}
        }
        match (&self.nearby_load, &self.sampled_load) {
            (Load::Loading, _) | (_, Load::Loading) => status.push(("Loading nearby systems…".into(), TEXT_DIM)),
            (Load::Failed(e), _) | (_, Load::Failed(e)) if cam.distance <= SAMPLE_RANGE => {
                status.push((format!("Nearby systems unavailable: {e}"), RED))
            }
            _ => {}
        }
        let mut y = inset.top();
        for (text, col) in status {
            let r = txt(painter, pos2(inset.left(), y), Align2::LEFT_TOP, &text, theme::body(14.0), col, 0.0);
            y = r.bottom() + 2.0;
        }

        let t = cam.target;
        let focus = format!("FOCUS  {:.1} / {:.1} / {:.1}    VIEW {} LY    GRID {} LY", t[0], t[1], t[2], fmt_ly(cam.distance), fmt_ly(grid_ly));
        txt(painter, inset.left_bottom(), Align2::LEFT_BOTTOM, &focus, theme::head(8.0), accent_dim(), 1.2);
        if cam.distance > DETAIL_RANGE {
            txt(painter, inset.right_bottom(), Align2::RIGHT_BOTTOM, "ZOOM IN FOR SINGLE SYSTEMS", theme::head(8.0), accent_dim(), 1.2);
        }
    }
}

/// A ground grid on the plane through the focus, spaced to the zoom and fading with
/// distance from the focus. Returns the spacing in ly.
fn paint_grid(painter: &Painter, cam: &Camera, canvas: Rect) -> f32 {
    let step = 10f32.powf((cam.distance * 0.35).max(1.0).log10().floor());
    let n = 8;
    let extent = n as f32 * step;
    let (cx, cz) = ((cam.target[0] / step).round() * step, (cam.target[2] / step).round() * step);
    let y = cam.target[1];
    let pieces = 12;
    for i in -n..=n {
        let off = i as f32 * step;
        for along_x in [false, true] {
            for k in 0..pieces {
                let a = -extent + 2.0 * extent * k as f32 / pieces as f32;
                let b = a + 2.0 * extent / pieces as f32;
                let (p, q) = if along_x {
                    ([cx + a, y, cz + off], [cx + b, y, cz + off])
                } else {
                    ([cx + off, y, cz + a], [cx + off, y, cz + b])
                };
                let mid = camera::lerp(p, q, 0.5);
                let fade = 1.0 - (camera::dist(mid, [cam.target[0], y, cam.target[2]]) / extent).powi(2);
                if fade <= 0.0 {
                    continue;
                }
                if let Some(seg) = cam.project_segment(p, q, canvas) {
                    painter.line_segment(seg, Stroke::new(1.0, alpha(accent(), (34.0 * fade) as u8)));
                }
            }
        }
    }
    step
}

/// A system marker: a ring with ticks (or corner brackets), a drop line to the grid
/// plane, and a two-line label.
#[allow(clippy::too_many_arguments)]
fn marker(painter: &Painter, proj: &Projector, cam: &Camera, canvas: Rect, pos: V3, col: Color32, caption: &str, name: &str, brackets: bool) {
    let foot = [pos[0], cam.target[1], pos[2]];
    if let Some(seg) = cam.project_segment(pos, foot, canvas) {
        painter.line_segment(seg, Stroke::new(1.0, alpha(col, 90)));
        painter.circle_filled(seg[1], 2.0, alpha(col, 120));
    }
    let Some(c) = proj.project(pos) else { return };
    if brackets {
        let (r, l) = (10.0, 4.0);
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let corner = c + vec2(sx * r, sy * r);
            painter.add(Shape::line(vec![corner - vec2(sx * l, 0.0), corner, corner - vec2(0.0, sy * l)], Stroke::new(1.5, col)));
        }
    } else {
        painter.circle_stroke(c, 8.0, Stroke::new(1.5, col));
        for d in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
            painter.line_segment([c + d * 11.0, c + d * 15.0], Stroke::new(1.5, col));
        }
    }
    let at = c + vec2(16.0, -4.0);
    backed_caption(painter, at, Align2::LEFT_BOTTOM, caption, col);
    if !name.is_empty() {
        label(painter, at + vec2(0.0, 1.0), Align2::LEFT_TOP, name, mix(col, TEXT, 0.4));
    }
}

/// A small spaced-out uppercase caption on a dark backing.
fn backed_caption(painter: &Painter, pos: Pos2, align: Align2, text: &str, col: Color32) {
    let galley = painter.layout_job(widgets::job(text, theme::head_bold(8.0), col, 1.4));
    let r = align.anchor_size(pos, galley.size());
    painter.rect_filled(r.expand2(vec2(4.0, 2.0)), 2.0, alpha(BG, 180));
    painter.galley(r.min, galley, col);
}

/// Text with a dark backing so it stays readable over bright stars.
fn label(painter: &Painter, pos: Pos2, align: Align2, text: &str, col: Color32) {
    let font = theme::body_semi(14.0);
    let size = text_size(painter, text, font.clone());
    let r = align.anchor_size(pos, size).expand2(vec2(4.0, 1.0));
    painter.rect_filled(r, 2.0, alpha(BG, 190));
    painter.text(pos, align, text, font, col);
}

/// A boxed tooltip-like caption, kept inside `canvas`.
fn tag(painter: &Painter, pos: Pos2, text: &str, col: Color32, canvas: Rect) {
    let font = theme::body(14.0);
    let size = text_size(painter, text, font.clone()) + vec2(12.0, 6.0);
    let mut r = Rect::from_min_size(pos - vec2(0.0, size.y), size);
    if r.right() > canvas.right() - 4.0 {
        r = r.translate(vec2(canvas.right() - 4.0 - r.right(), 0.0));
    }
    if r.top() < canvas.top() + 4.0 {
        r = r.translate(vec2(0.0, canvas.top() + 4.0 - r.top()));
    }
    painter.rect(r, 0.0, alpha(theme::PANEL, 235), Stroke::new(1.0, alpha(accent(), 140)), egui::StrokeKind::Inside);
    painter.text(r.center(), Align2::CENTER_CENTER, text, font, col);
}

fn legend_row(ui: &mut Ui, col: Color32, text: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(16.0, 18.0), Sense::hover());
        ui.painter().circle_filled(r.center(), 4.0, col);
        ui.painter().circle_filled(r.center(), 7.0, alpha(col, 40));
        ui.label(egui::RichText::new(text).font(theme::body(15.0)).color(TEXT));
    });
}

fn faction_row(ui: &mut Ui, name: &str, influence: f32) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
    let p = ui.painter();
    let pct = format!("{:.1}%", influence * 100.0);
    let pr = txt(p, pos2(r.right(), r.center().y), Align2::RIGHT_CENTER, &pct, theme::body_semi(15.0), accent_hi(), 0.0);
    let bar = Rect::from_min_max(pos2(pr.left() - 56.0, r.center().y - 3.0), pos2(pr.left() - 8.0, r.center().y + 3.0));
    widgets::seg_bar(p, bar, influence.clamp(0.0, 1.0), accent());
    let name_rect = Rect::from_min_max(r.min, pos2(bar.left() - 8.0, r.max.y));
    let g = p.layout(name.to_string(), theme::body(15.0), TEXT, f32::INFINITY);
    let clip = p.with_clip_rect(name_rect.intersect(p.clip_rect()));
    clip.galley(pos2(r.left(), r.center().y - g.size().y / 2.0), g, TEXT);
}

fn text_size(painter: &Painter, text: &str, font: egui::FontId) -> egui::Vec2 {
    painter.layout_no_wrap(text.to_string(), font, TEXT).size()
}

/// Light years with sensible precision: `4.37`, `128.4`, `22 000`.
fn fmt_ly(ly: f32) -> String {
    if ly < 10.0 {
        format!("{ly:.2}")
    } else if ly < 1_000.0 {
        format!("{ly:.1}")
    } else {
        group_thousands(ly.round() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::fmt_ly;

    #[test]
    fn light_years_read_well() {
        assert_eq!(fmt_ly(4.366), "4.37");
        assert_eq!(fmt_ly(128.44), "128.4");
        assert!(fmt_ly(22_000.4).starts_with("22"));
    }
}
