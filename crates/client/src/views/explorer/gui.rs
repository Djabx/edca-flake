//! Desktop-window drawing for [`ExplorerView`]: the selected system (overview, powerplay,
//! construction, factions) above the jump history on the left, the collapsible body tree in
//! the middle and the selected row's details on the right.
//!
//! Keys still go through `handle_key`; clicks set the same fields the keys move (focus,
//! `selected_idx`, `selected_faction`, `selected_system_idx`), and the scroll areas follow
//! the selection whenever the keyboard changes it.

use std::cmp::Ordering;

use eframe::egui::{
    self, pos2,
    text::{LayoutJob, TextWrapping},
    vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, TextFormat, Ui,
};

use super::{
    body_signal_icon, capitalise, find_body_in_tree, format_population, format_thousands, exobiology_facts, is_belt_cluster, is_valuable_ring, kind_icon, node_style, parse_color,
    ring_class_name, signal_icon, strip_system_prefix, tree_depth, ExplorerFocus, ExplorerView, FlatNode, NodeType, SystemHistory,
};
use super::influence_chart::{self, Series};
use crate::event_shim::{KeyCode, KeyEvent};
use crate::gui::theme::{self, accent, accent_dim, accent_hi, alpha, mix, BG, CYAN, GREEN, GREY, RED, TEXT, TEXT_DIM, YELLOW};
use crate::gui::genus_art;
use edcas_common::exobiology::{genus_name, possible_species, Possible, Species};
use crate::gui::widgets::{self, txt};
use edcas_common::exploration::FIRST_FOOTFALL;
use edcas_common::format::credits as format_credits;
use crate::alerts::valuable_kind;
use crate::body_finds::{Find, FindKind};
use crate::journal_reader::{BodyScan, GenusSignal, ConflictData, DiscoveredSignal, Discovery, FactionInfo, JournalData, StationData, SystemData, TreeNode};
use crate::settings::Settings;
use crate::views::ViewEvent;

/// Tree row height.
/// m/s² per g: the journal's `SurfaceGravity` is in m/s², the panel shows g as the game does (it showed the
/// raw value as g, 9.8 times too high, reported 2026-11-07).
const STANDARD_GRAVITY: f32 = 9.80665;
const ROW: f32 = 20.0;
/// Body tree row height: room for the largest body disc.
const TREE_ROW: f32 = 26.0;
/// Horizontal step per tree level.
const INDENT: f32 = 24.0;
/// Offset of a root row's icon from the row's left edge (room for its expander).
const BASE: f32 = 28.0;
/// Furthest a body icon reaches from its centre (a ring reaches further, to [`RING_REACH`]).
const RING_MAX: f32 = 11.5;
/// Furthest a planet's ring reaches sideways; the expander's tip is 13.5 px left of centre.
const RING_REACH: f32 = 13.0;
/// Smallest and largest planet disc radius; the radius between them follows the body's
/// radius on a log scale, so bodies keep their size relative to each other.
const DISC_MIN: f32 = 3.5;
const DISC_MAX: f32 = 10.5;
/// Width of the landable marker column at the right of a tree row.
const LAND_W: f32 = 26.0;
/// Scroll distance of one `detail_scroll` step.
const DETAIL_STEP: f32 = 22.0;
/// Colour of rocky composition bars.
const ROCK: Color32 = Color32::from_rgb(186, 146, 104);

/// What the window drew last frame, to tell keyboard moves (scroll to them) from clicks.
#[derive(Default)]
pub(super) struct GuiState {
    tree_sel: Option<usize>,
    faction_sel: Option<usize>,
    /// Faction row under the pointer last frame; its chart line is drawn highlighted.
    faction_hover: Option<usize>,
    system_sel: Option<usize>,
    detail_scroll: Option<usize>,
}

impl ExplorerView {
    pub fn ui(&mut self, ui: &mut Ui, settings: &Settings, journal: &JournalData) -> ViewEvent {
        let full = ui.available_rect_before_wrap();
        let gap = 10.0;
        let left_w = (full.width() * 0.25).clamp(250.0, 380.0);
        let tree_w = ((full.width() - left_w - 2.0 * gap) * 0.52).round();
        let hist_rows = self.visited_systems.len().max(1) as f32;
        let hist_h = (54.0 + hist_rows * 22.0).min((full.height() * 0.3).clamp(140.0, 250.0));
        let sys_rect = Rect::from_min_size(full.min, vec2(left_w, full.height() - hist_h - gap));
        let hist_rect = Rect::from_min_max(pos2(full.left(), sys_rect.bottom() + gap), pos2(sys_rect.right(), full.bottom()));
        let tree_rect = Rect::from_min_size(pos2(sys_rect.right() + gap, full.top()), vec2(tree_w, full.height()));
        let detail_rect = Rect::from_min_max(pos2(tree_rect.right() + gap, full.top()), full.max);

        if clicked_in(ui, sys_rect.union(hist_rect)) {
            self.focus = ExplorerFocus::SystemInfo;
        } else if clicked_in(ui, tree_rect) {
            self.focus = ExplorerFocus::Tree;
        } else if clicked_in(ui, detail_rect) {
            self.focus = ExplorerFocus::Detail;
        }

        let event = self.system_panel(ui, sys_rect, journal);
        self.history_panel(ui, hist_rect);
        self.tree_panel(ui, tree_rect, settings);
        self.detail_panel(ui, detail_rect, settings, journal);

        let focused = match self.focus {
            ExplorerFocus::SystemInfo => sys_rect,
            ExplorerFocus::Tree => tree_rect,
            ExplorerFocus::Detail => detail_rect,
        };
        paint_focus(ui.painter(), focused);
        ui.allocate_rect(full, Sense::hover());
        event
    }

    /// The body tree and detail panels alone, for a system from the galaxy map
    /// ([`ExplorerView::show_remote_system`]).
    pub(crate) fn system_map_ui(&mut self, ui: &mut Ui, tree_rect: Rect, detail_rect: Rect, settings: &Settings, journal: &JournalData) {
        if clicked_in(ui, tree_rect) {
            self.focus = ExplorerFocus::Tree;
        } else if clicked_in(ui, detail_rect) {
            self.focus = ExplorerFocus::Detail;
        }
        self.tree_panel(ui, tree_rect, settings);
        self.detail_panel(ui, detail_rect, settings, journal);
        paint_focus(ui.painter(), if self.focus == ExplorerFocus::Detail { detail_rect } else { tree_rect });
    }

    // ------------------------------------------------------------ system + factions

    /// Overview, powerplay, construction and factions of the system picked in the history.
    fn system_panel(&mut self, ui: &mut Ui, rect: Rect, journal: &JournalData) -> ViewEvent {
        let moved = self.gui.faction_sel != Some(self.selected_faction);
        self.gui.faction_sel = Some(self.selected_faction);
        let mut open_faction = None;
        let mut faction_hover: Option<Option<usize>> = None;
        let mut step: Option<usize> = None;
        let this = &*self;
        widgets::hud_panel_at(ui, rect, "System", "w/s · space", |ui| {
            let Some(system) = this.visited_systems.get(this.selected_system_idx) else {
                widgets::empty_state(ui, "No system data available — ensure journal directory is configured in Settings.");
                return;
            };
            ui.label(egui::RichText::new(&system.name).font(theme::body_semi(21.0)).color(accent_hi()));
            ui.horizontal(|ui| {
                if this.selected_system_idx == 0 {
                    widgets::badge(ui, "Current system", GREEN);
                } else {
                    let ago = this.selected_system_idx;
                    widgets::badge(ui, &format!("{ago} jump{} ago", if ago == 1 { "" } else { "s" }), TEXT_DIM);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let newer = this.selected_system_idx > 0;
                    if widgets::hud_button_enabled(ui, "▶", newer).on_hover_text("Newer system (d)").clicked() {
                        step = Some(this.selected_system_idx - 1);
                    }
                    let older = this.selected_system_idx + 1 < this.visited_systems.len();
                    if widgets::hud_button_enabled(ui, "◀", older).on_hover_text("Older system (a)").clicked() {
                        step = Some(this.selected_system_idx + 1);
                    }
                });
            });
            ui.add_space(4.0);

            egui::ScrollArea::vertical().id_salt("explorer-system").auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                widgets::section(ui, "Overview", None);
                kv_row(ui, 96.0, "Economy", &pretty(&system.economy), "", TEXT);
                if !system.second_economy.is_empty() {
                    kv_row(ui, 96.0, "2nd Economy", &pretty(&system.second_economy), "", TEXT);
                }
                kv_row(ui, 96.0, "Government", &pretty(&system.government), "", TEXT);
                kv_row(ui, 96.0, "Allegiance", &pretty(&system.allegiance), "", TEXT);
                kv_row(ui, 96.0, "Security", &pretty(&system.security), "", TEXT);
                kv_row(ui, 96.0, "Population", &format_population(system.population), "", TEXT);
                if let Some(history) = this.history_of(system.system_address) {
                    tick_rows(ui, history);
                }

                let control = system.controlling_power.as_deref().filter(|p| !p.is_empty());
                if control.is_some() || !system.powers.is_empty() {
                    ui.add_space(6.0);
                    widgets::section(ui, "Powerplay", None);
                    if let Some(power) = control {
                        kv_row(ui, 96.0, "Control", power, "", accent_hi());
                    }
                    if !system.powers.is_empty() {
                        kv_row(ui, 96.0, "Powers", &system.powers.join(", "), "", TEXT);
                    }
                }

                if this.selected_system_idx == 0 {
                    construction(ui, system, journal);
                }

                if !system.factions.is_empty() {
                    let mut sorted = system.factions.clone();
                    sorted.sort_by(|a, b| b.influence.partial_cmp(&a.influence).unwrap_or(Ordering::Equal));
                    let sel = this.selected_faction.min(sorted.len() - 1);
                    let focused = this.focus == ExplorerFocus::SystemInfo;
                    ui.add_space(6.0);
                    widgets::section(ui, "Factions", Some(sorted.len()));

                    // The server's readings, plus the journal's current share for the system
                    // the commander is in, so each line ends at today's value.
                    let now = chrono::Utc::now().timestamp();
                    let entry = this.history.get(&system.system_address);
                    let history = entry.and_then(|e| e.data.as_ref());
                    let series: Vec<Series> = sorted
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            let mut points: Vec<(i64, f32)> = history
                                .map(|h| h.influence.iter().filter(|p| p.faction_name == f.name).map(|p| (p.timestamp.timestamp(), p.influence)).collect())
                                .unwrap_or_default();
                            if this.selected_system_idx == 0 && points.last().is_none_or(|p| p.0 < now) {
                                points.push((now, f.influence));
                            }
                            Series { name: &f.name, colour: influence_chart::colour(i), points }
                        })
                        .collect();
                    let start = series.iter().filter_map(|s| s.points.first().map(|p| p.0)).min().unwrap_or(now).min(now - 86_400);
                    let note = match history {
                        None if entry.is_some() => Some("Loading influence history…"),
                        None => Some("Influence history needs the server"),
                        Some(h) if h.influence_failed => Some("Influence history unavailable"),
                        Some(_) if series.iter().all(|s| s.points.len() < 2) => Some("No influence history yet"),
                        Some(_) => None,
                    };
                    let highlight = this.gui.faction_hover.or((focused).then_some(sel));
                    influence_chart::chart(ui, &series, start, now, highlight, note);
                    ui.add_space(4.0);

                    let mut hovered = None;
                    for (i, faction) in sorted.iter().enumerate() {
                        let is_sel = i == sel && focused;
                        let resp = faction_card(ui, faction, faction.name == system.system_faction, is_sel, series[i].colour, series[i].change())
                            .on_hover_text("Open in Factions (space)");
                        if resp.hovered() {
                            hovered = Some(i);
                        }
                        if resp.clicked() {
                            open_faction = Some(i);
                        }
                        if is_sel && moved {
                            resp.scroll_to_me(None);
                        }
                    }
                    faction_hover = Some(hovered);
                    ui.add_space(4.0);
                    widgets::hint(ui, "Lines show the last 30 days; ▲▼ is the change over that time. Click a faction (or select it and press space) to open it in Factions.");
                }
            });
        });

        if let Some(h) = faction_hover {
            self.gui.faction_hover = h;
        }
        if let Some(idx) = step {
            self.focus = ExplorerFocus::SystemInfo;
            self.select_system(idx);
        }
        if let Some(i) = open_faction {
            self.focus = ExplorerFocus::SystemInfo;
            self.selected_faction = i;
            self.gui.faction_sel = Some(i);
            return self.handle_key(&KeyEvent::from(KeyCode::Char(' ')));
        }
        ViewEvent::None
    }

    /// Every visited system, newest first; click one to show it (what `a`/`d` step through).
    fn history_panel(&mut self, ui: &mut Ui, rect: Rect) {
        let moved = self.gui.system_sel != Some(self.selected_system_idx);
        self.gui.system_sel = Some(self.selected_system_idx);
        let mut pick = None;
        let this = &*self;
        let right = format!("{} · a/d", this.visited_systems.len());
        widgets::hud_panel_at(ui, rect, "Jump history", &right, |ui| {
            if this.visited_systems.is_empty() {
                widgets::empty_state(ui, "No systems visited yet.");
                return;
            }
            egui::ScrollArea::vertical().id_salt("explorer-history").auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let last = this.visited_systems.len() - 1;
                for (i, sys) in this.visited_systems.iter().enumerate() {
                    let sel = i == this.selected_system_idx;
                    let resp = widgets::selectable_row(ui, sel, 22.0, |ui| {
                        let r = ui.max_rect();
                        let p = ui.painter();
                        let cy = r.center().y;
                        let dot = pos2(r.left() + 5.0, cy);
                        let line = Stroke::new(1.0, widgets::row_text_color(sel, alpha(accent(), 70)));
                        let top = if i == 0 { cy } else { r.top() };
                        let bottom = if i == last { cy } else { r.bottom() };
                        p.line_segment([pos2(dot.x, top), pos2(dot.x, bottom)], line);
                        if i == 0 {
                            p.circle_filled(dot, 4.0, widgets::row_text_color(sel, accent()));
                        } else {
                            p.circle_filled(dot, 3.0, if sel { accent() } else { theme::PANEL });
                            p.circle_stroke(dot, 3.0, Stroke::new(1.0, widgets::row_text_color(sel, accent_dim())));
                        }
                        let mut right_x = r.right();
                        if let Some(n) = this.scans_cache.get(&sys.system_address).map(|s| s.bodies.len()).filter(|&n| n > 0) {
                            let label = format!("{n} {}", if n == 1 { "body" } else { "bodies" });
                            let tr = txt(p, pos2(right_x, cy), Align2::RIGHT_CENTER, &label, theme::body(13.5), widgets::row_text_color(sel, TEXT_DIM), 0.0);
                            right_x = tr.left() - 8.0;
                        }
                        if i == 0 {
                            let tr = txt(p, pos2(right_x, cy), Align2::RIGHT_CENTER, "NOW", theme::head(8.0), widgets::row_text_color(sel, GREEN), 1.2);
                            right_x = tr.left() - 8.0;
                        }
                        let col = if i == 0 { accent_hi() } else { TEXT };
                        elide(p, pos2(r.left() + 16.0, cy), Align2::LEFT_CENTER, &sys.name, theme::body_semi(15.0), widgets::row_text_color(sel, col), right_x - r.left() - 16.0);
                    });
                    let resp = resp.on_hover_text("Show this system (a: older, d: newer)");
                    if resp.clicked() {
                        pick = Some(i);
                    }
                    if sel && moved {
                        resp.scroll_to_me(None);
                    }
                }
            });
        });
        if let Some(i) = pick {
            self.focus = ExplorerFocus::SystemInfo;
            self.select_system(i);
        }
    }

    // ------------------------------------------------------------ body tree

    /// The system's bodies, stations and signals as a tree with guide lines and expanders.
    fn tree_panel(&mut self, ui: &mut Ui, rect: Rect, settings: &Settings) {
        let moved = self.gui.tree_sel != Some(self.selected_idx);
        self.gui.tree_sel = Some(self.selected_idx);
        let mut clicked = None;
        let mut toggle = None;
        let this = &*self;
        widgets::hud_panel_at(ui, rect, "Navigation", "w/s · pgup/pgdn", |ui| {
            this.tree_header(ui);
            if this.flat_nodes.is_empty() {
                let msg = if this.remote {
                    "No bodies of this system are known yet."
                } else if this.loading_scans() {
                    "Reading this system's scans from the journals…"
                } else if this.selected_system_idx > 0 {
                    "No scans of this system in the recent journals."
                } else {
                    "No bodies scanned yet.\nUse the FSS scanner to map bodies."
                };
                widgets::empty_state(ui, msg);
                return;
            }
            egui::ScrollArea::vertical().id_salt("explorer-tree").auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let populated = this.shown_populated();
                for (idx, node) in this.flat_nodes.iter().enumerate() {
                    if matches!(node.node_type, NodeType::SectionHeader) {
                        let count = this.flat_nodes[idx + 1..]
                            .iter()
                            .take_while(|n| !matches!(n.node_type, NodeType::SectionHeader))
                            .count();
                        ui.add_space(6.0);
                        widgets::section(ui, node.short_name.split(" (").next().unwrap_or(&node.short_name), Some(count));
                        continue;
                    }
                    let next_depth = this.flat_nodes.get(idx + 1).map(|n| tree_depth(&n.tree_prefix)).unwrap_or(0);
                    let sel = idx == this.selected_idx;
                    let body = matches!(node.node_type, NodeType::Body).then(|| find_body_in_tree(&this.tree, node.body_id)).flatten();
                    let first = body.and_then(|b| b.discovery).is_some_and(|d| charted(&d, populated) == Standing::First);
                    let valuable = first && body.is_some_and(|b| valuable_kind(b).is_some());
                    let (resp, has_kids, on_expander) = tree_row(ui, node, sel, first, valuable, next_depth, &this.system_name, settings);
                    if resp.clicked() {
                        if on_expander {
                            toggle = Some(idx);
                        } else {
                            clicked = Some(idx);
                        }
                    }
                    if resp.double_clicked() && has_kids {
                        toggle = Some(idx);
                    }
                    if sel && moved {
                        resp.scroll_to_me(None);
                    }
                }
                ui.add_space(8.0);
            });
        });
        if let Some(i) = clicked {
            self.focus = ExplorerFocus::Tree;
            if self.selected_idx != i {
                self.selected_idx = i;
                self.detail_scroll = 0;
            }
        }
        if let Some(i) = toggle {
            self.focus = ExplorerFocus::Tree;
            self.toggle_collapsed(i);
        }
    }

    /// System name, body counts and FSS / nav-beacon progress above the tree.
    fn tree_header(&self, ui: &mut Ui) {
        let name = if self.system_name.is_empty() { "Unknown System" } else { &self.system_name };
        let (bodies, stars) = count_bodies(&self.tree);
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
        let p = ui.painter();
        let counts = format!("{bodies} BODIES  ·  {stars} STAR{}", if stars == 1 { "" } else { "S" });
        let cr = txt(p, pos2(r.right(), r.center().y), Align2::RIGHT_CENTER, &counts, theme::head(8.0), accent_dim(), 1.2);
        elide(p, pos2(r.left(), r.center().y), Align2::LEFT_CENTER, name, theme::body_semi(19.0), accent_hi(), cr.left() - r.left() - 10.0);

        if let Some(total) = self.fss_body_count {
            let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
            let p = ui.painter();
            let cy = r.center().y;
            let lr = txt(p, pos2(r.left(), cy), Align2::LEFT_CENTER, "FSS", theme::head(8.5), accent_dim(), 1.4);
            if self.fss_all_bodies_found {
                txt(p, pos2(lr.right() + 10.0, cy), Align2::LEFT_CENTER, &format!("All {total} found ✓"), theme::body_semi(15.0), GREEN, 0.0);
            } else {
                let scanned = fss_bodies(&self.tree).min(total);
                let non_body = self.fss_non_body_count.unwrap_or(0);
                let tr = txt(p, pos2(r.right(), cy), Align2::RIGHT_CENTER, &format!("{scanned}/{total} bodies · signals: {non_body}"), theme::body(14.0), TEXT_DIM, 0.0);
                let bar = Rect::from_min_max(pos2(lr.right() + 10.0, cy - 3.5), pos2(tr.left() - 10.0, cy + 3.5));
                let frac = if total > 0 { scanned as f32 / total as f32 } else { 0.0 };
                widgets::seg_bar(p, bar, frac, accent());
            }
        }
        self.discovery_summary(ui, w);
        self.value_summary(ui, w);
        if let Some(nb) = self.nav_beacon_bodies {
            widgets::hint(ui, &format!("Nav Beacon: {nb} bodies"));
        }
        ui.add_space(4.0);
    }

    /// The shown system's address, when it comes from the journal.
    fn shown_address(&self) -> Option<i64> {
        self.visited_systems.get(self.selected_system_idx).filter(|_| !self.remote).map(|s| s.system_address)
    }

    /// Where the shown system lies, when the journal said.
    fn shown_position(&self) -> Option<[f32; 3]> {
        self.visited_systems.get(self.selected_system_idx).filter(|_| !self.remote).and_then(|s| s.star_pos)
    }

    /// Whether the shown system is populated. There `WasDiscovered:false` marks pre-charted
    /// bodies (Sol, Sedna, …) that never get a discoverer tag, not a first discovery.
    fn shown_populated(&self) -> bool {
        !self.remote && self.visited_systems.get(self.selected_system_idx).is_some_and(|s| s.population > 0)
    }

    /// One line on how much of the system is still undiscovered, from the commander's own
    /// scans (a system from the server has no such line: someone reported it, so it's known).
    fn discovery_summary(&self, ui: &mut Ui, w: f32) {
        let populated = self.shown_populated();
        let (mut scanned, mut first, mut pre) = (0, 0, 0);
        let mut new_system = false;
        walk_bodies(&self.tree, &mut |b| {
            let Some(d) = b.discovery.filter(|d| !d.reported) else { return };
            scanned += 1;
            match charted(&d, populated) {
                Standing::First => {
                    first += 1;
                    new_system |= !b.star_type.is_empty() && b.distance_from_arrival_ls == 0.0;
                }
                Standing::PreCharted => pre += 1,
                Standing::Known => {}
            }
        });
        if scanned == 0 {
            return;
        }
        let (text, col) = if first > 0 && new_system {
            (format!("Undiscovered system · {first} of {scanned} bodies new"), YELLOW)
        } else if first > 0 {
            (format!("{first} of {scanned} bodies undiscovered"), YELLOW)
        } else if pre > 0 {
            ("Pre-charted space · no discoverer tags".to_string(), TEXT_DIM)
        } else if scanned == 1 {
            ("Already discovered".to_string(), TEXT_DIM)
        } else {
            (format!("All {scanned} scanned bodies already discovered"), TEXT_DIM)
        };
        let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
        let p = ui.painter();
        let cy = r.center().y;
        let lr = txt(p, pos2(r.left(), cy), Align2::LEFT_CENTER, "DISCOVERY", theme::head(8.5), accent_dim(), 1.4);
        let mut x = lr.right() + 10.0;
        if first > 0 {
            x = first_mark(p, pos2(x + 5.0, cy), YELLOW) + 6.0;
        }
        elide(p, pos2(x, cy), Align2::LEFT_CENTER, &text, theme::body_semi(14.5), col, r.right() - x);
    }

    /// Whether organic data from this body pays the first-footfall bonus: as the unsold data
    /// recorded it when analysed, else as the body's scan says.
    fn first_footfall(&self, body_id: i32) -> bool {
        if let Some(b) = self.shown_address().and_then(|a| self.unsold.bio_on(a, body_id).next()) {
            return b.first_footfall;
        }
        let mut untrodden = false;
        walk_bodies(&self.tree, &mut |b| {
            untrodden |= b.body_id == body_id && b.discovery.as_ref().is_some_and(|d| d.footfalled == Some(false));
        });
        untrodden
    }

    /// What an organic species found on `body_id` pays at Vista Genomics, first-footfall
    /// bonus included.
    fn organic_value(&self, body_id: i32, find: &Find) -> Option<i64> {
        let bonus = self.first_footfall(body_id);
        find.value.map(|v| if bonus { v * FIRST_FOOTFALL } else { v })
    }

    /// What the shown system's scans sell for, what mapping the rest would add, and what
    /// its analysed organic species pay. All unsold data is summed up in the window's footer.
    fn value_summary(&self, ui: &mut Ui, w: f32) {
        let (mut here, mut by_mapping) = (0, 0);
        walk_bodies(&self.tree, &mut |b| {
            let now = b.credits().unwrap_or(0);
            here += now;
            by_mapping += b.credits_if_mapped().map_or(0, |m| m - now);
        });
        let bio: i64 = self.shown_address().map_or(0, |a| {
            self.finds
                .in_system(a)
                .flat_map(|(id, finds)| finds.iter().filter(|f| f.samples >= 3).filter_map(move |f| self.organic_value(id, f)))
                .sum()
        });
        if here == 0 && bio == 0 {
            return;
        }
        let (r, resp) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
        let p = ui.painter();
        let cy = r.center().y;
        let lr = txt(p, pos2(r.left(), cy), Align2::LEFT_CENTER, "VALUE", theme::head(8.5), accent_dim(), 1.4);
        let mut job = LayoutJob::default();
        job.append(&format_credits(here), 0.0, fmt(theme::body_semi(14.5), accent_hi()));
        job.append(" this system", 0.0, fmt(theme::body(14.0), TEXT_DIM));
        if by_mapping > 0 {
            job.append(&format!("  +{} if mapped", format_credits(by_mapping)), 0.0, fmt(theme::body(14.0), GREEN));
        }
        if bio > 0 {
            job.append(&format!("  {} bio", format_credits(bio)), 0.0, fmt(theme::body_semi(14.0), GREEN));
        }
        job.wrap = TextWrapping { max_width: r.right() - lr.right() - 10.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
        let g = p.layout_job(job);
        p.galley(pos2(lr.right() + 10.0, cy - g.size().y / 2.0), g, TEXT);
        resp.on_hover_text(
            "Estimated Universal Cartographics value: what this system's scans sell for, and what \
             surface-mapping the rest (within the probe target) would add. Bio: what the species \
             analysed here pay at Vista Genomics. All unsold data is in the footer.",
        );
    }

    // ------------------------------------------------------------ details

    /// Details of the selected tree row; `w`/`s` (detail focus) and the wheel scroll it.
    fn detail_panel(&mut self, ui: &mut Ui, rect: Rect, settings: &Settings, journal: &JournalData) {
        let kind = match self.flat_nodes.get(self.selected_idx).map(|n| &n.node_type) {
            Some(NodeType::Station(_)) => "station",
            Some(NodeType::Signal(_)) => "signal",
            Some(NodeType::Body | NodeType::BodySignal(_) | NodeType::Ring(_)) => "body scan",
            _ => "",
        };
        let target = (self.gui.detail_scroll != Some(self.detail_scroll)).then_some(self.detail_scroll as f32 * DETAIL_STEP);
        let this = &*self;
        let offset = widgets::hud_panel_at(ui, rect, "Details", kind, |ui| {
            let mut area = egui::ScrollArea::vertical().id_salt("explorer-detail").auto_shrink([false, false]);
            if let Some(y) = target {
                area = area.vertical_scroll_offset(y);
            }
            let out = area.show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                this.detail_contents(ui, settings, journal);
                ui.add_space(8.0);
            });
            let max = (out.content_size.y - out.inner_rect.height()).max(0.0);
            out.state.offset.y.clamp(0.0, max)
        });
        let line = (offset / DETAIL_STEP).round() as usize;
        self.detail_scroll = line;
        self.gui.detail_scroll = Some(line);
    }

    fn detail_contents(&self, ui: &mut Ui, settings: &Settings, journal: &JournalData) {
        match self.flat_nodes.get(self.selected_idx).map(|n| &n.node_type) {
            Some(NodeType::Station(station)) => station_detail(ui, station, &self.system_name),
            Some(NodeType::Signal(sig)) => signal_detail(ui, sig, settings),
            _ => match self.get_selected_body() {
                Some(body) => self.body_detail(ui, body, settings, journal),
                None => widgets::empty_state(ui, "Select a body to see details."),
            },
        }
    }

    fn body_detail(&self, ui: &mut Ui, body: &BodyScan, settings: &Settings, journal: &JournalData) {
        let type_label = if !body.planet_class.is_empty() {
            body.planet_class.clone()
        } else if !body.star_type.is_empty() {
            format!("Star ({})", body.star_type)
        } else {
            "Star / Barycentre".to_string()
        };
        let mut badges: Vec<(String, Color32)> = Vec::new();
        if body.landable {
            badges.push(("Landable".into(), CYAN));
        }
        if !body.terraform_state.is_empty() {
            badges.push((body.terraform_state.clone(), GREEN));
        }
        if body.rings.iter().any(|r| is_valuable_ring(&r.ring_class)) {
            badges.push(("Valuable rings".into(), RED));
        }
        let populated = self.shown_populated();
        if let Some(d) = &body.discovery {
            if charted(d, populated) == Standing::First && valuable_kind(body).is_some() {
                badges.push(("Valuable".into(), RED));
            }
            badges.push(match charted(d, populated) {
                Standing::First => ("Undiscovered".into(), YELLOW),
                Standing::PreCharted => ("Pre-charted".into(), TEXT_DIM),
                Standing::Known => ("Discovered".into(), TEXT_DIM),
            });
        }
        let short = strip_system_prefix(&body.body_name, &self.system_name);
        detail_header(ui, &body.body_name, &short, &type_label, &badges, None);
        ui.horizontal(|ui| {
            readout(ui, "Dist. from arrival", &format!("{:.2}", body.distance_from_arrival_ls), "Ls");
            if let Some(value) = body.credits().filter(|&c| c > 0) {
                ui.add_space(18.0);
                readout(ui, "Est. value", &format_thousands(value), "Cr");
            }
        });
        ui.add_space(6.0);

        let physical = |ui: &mut Ui| {
            widgets::section(ui, "Physical", None);
            kv_row(ui, 100.0, "Body ID", &body.body_id.to_string(), "", TEXT);
            kv_row(ui, 100.0, "Distance", &format!("{:.2}", body.distance_from_arrival_ls), "Ls", TEXT);
            if body.radius > 0.0 {
                kv_row(ui, 100.0, "Radius", &format!("{:.0}", body.radius / 1000.0), "km", TEXT);
            }
            if body.mass_em > 0.0 {
                kv_row(ui, 100.0, "Mass", &format!("{:.4}", body.mass_em), "EM", TEXT);
            }
            if body.surface_gravity > 0.0 {
                kv_row(ui, 100.0, "Gravity", &format!("{:.2}", body.surface_gravity / STANDARD_GRAVITY), "g", TEXT);
            }
            if body.surface_temperature > 0.0 {
                kv_row(ui, 100.0, "Temperature", &format!("{:.0}", body.surface_temperature), "K", TEXT);
            }
        };
        let status = |ui: &mut Ui| {
            widgets::section(ui, "Status", None);
            let (land, land_col) = if body.landable { ("Yes", CYAN) } else { ("No", TEXT) };
            kv_row(ui, 100.0, "Landable", land, "", land_col);
            if body.tidal_lock {
                kv_row(ui, 100.0, "Tidal Lock", "Yes", "", TEXT);
            }
            if !body.terraform_state.is_empty() {
                kv_row(ui, 100.0, "Terraform", &body.terraform_state, "", GREEN);
            }
            if !body.atmosphere.is_empty() {
                kv_row(ui, 100.0, "Atmosphere", &body.atmosphere, "", TEXT);
            }
            if !body.volcanism.is_empty() {
                kv_row(ui, 100.0, "Volcanism", &body.volcanism, "", TEXT);
            }
            if !body.scan_type.is_empty() {
                kv_row(ui, 100.0, "Scan", &body.scan_type, "", TEXT);
            }
        };
        if ui.available_width() >= 460.0 {
            ui.columns(2, |cols| {
                physical(&mut cols[0]);
                status(&mut cols[1]);
            });
        } else {
            physical(ui);
            ui.add_space(6.0);
            status(ui);
        }

        if let Some(d) = &body.discovery {
            ui.add_space(8.0);
            let unsold = self.shown_address().is_some_and(|a| self.unsold.holds(a, body.body_id));
            discovery(ui, body, d, charted(d, populated), unsold);
        }

        self.body_life(ui, body, settings);

        if !body.rings.is_empty() {
            ui.add_space(8.0);
            widgets::section(ui, "Rings", Some(body.rings.len()));
            for ring in &body.rings {
                let short = ring.name.strip_prefix(&body.body_name).unwrap_or(&ring.name).trim();
                let class = ring_class_name(&ring.ring_class);
                let valuable = is_valuable_ring(&ring.ring_class);
                let ring_col = ring_colour(&ring.ring_class, settings);
                let w = ui.available_width();
                let (r, _) = ui.allocate_exact_size(vec2(w, ROW), Sense::hover());
                let p = ui.painter();
                let cy = r.center().y;
                p.add(Shape::ellipse_stroke(pos2(r.left() + 12.0, cy), vec2(8.0, 3.2), Stroke::new(1.4, ring_col)));
                p.circle_filled(pos2(r.left() + 12.0, cy), 2.4, ring_col);
                let mut x = r.right();
                if valuable {
                    x = tag(p, pos2(x, cy), "Valuable", RED).left() - 8.0;
                }
                let cr = txt(p, pos2(x, cy), Align2::RIGHT_CENTER, &class, theme::body_semi(15.0), if valuable { RED } else { TEXT_DIM }, 0.0);
                elide(p, pos2(r.left() + 28.0, cy), Align2::LEFT_CENTER, short, theme::body(15.0), TEXT, cr.left() - r.left() - 36.0);
            }
        }

        if let Some(comp) = &body.composition {
            ui.add_space(8.0);
            widgets::section(ui, "Composition", None);
            for (name, v, col) in [("Ice", comp.ice, CYAN), ("Rock", comp.rock, ROCK), ("Metal", comp.metal, accent())] {
                bar_row(ui, name, v as f64 * 100.0, 100.0, col, None);
            }
        }

        if !body.materials.is_empty() {
            ui.add_space(8.0);
            widgets::section(ui, "Materials", Some(body.materials.len()));
            let mut mats = body.materials.clone();
            mats.sort_by(|a, b| b.percent.partial_cmp(&a.percent).unwrap_or(Ordering::Equal));
            let scale = mats.first().map(|m| m.percent).unwrap_or(0.0).max(25.0);
            for mat in &mats {
                let inv = journal.materials_raw.iter().find(|i| i.name == mat.name).map(|i| i.count).unwrap_or(0);
                bar_row(ui, &capitalise(&mat.name), mat.percent, scale, accent(), Some(inv));
            }
        }

        self.body_signals(ui, body, settings);
    }

    /// Signals (SAA preferred over FSS counts), genera and organic scans of a body.
    fn body_signals(&self, ui: &mut Ui, body: &BodyScan, settings: &Settings) {
        let body_id = body.body_id;
        let signals = self.saa_data.get(&body_id).map(|s| &s.signals).or_else(|| self.fss_signals.get(&body_id));
        match signals {
            Some(sigs) if !sigs.is_empty() || self.saa_data.contains_key(&body_id) => {
                ui.add_space(8.0);
                widgets::section(ui, "Signals", Some(sigs.len()));
                for sig in sigs {
                    let (glyph, col) = body_signal_icon(sig, settings);
                    let col = theme::from_ratatui(col);
                    let name_col = if sig.is_biological() {
                        GREEN
                    } else if sig.is_geological() {
                        YELLOW
                    } else {
                        TEXT
                    };
                    let w = ui.available_width();
                    let (r, _) = ui.allocate_exact_size(vec2(w, ROW), Sense::hover());
                    let p = ui.painter();
                    let cy = r.center().y;
                    txt(p, pos2(r.left() + 12.0, cy), Align2::CENTER_CENTER, &glyph, theme::body(15.0), col, 0.0);
                    let cr = txt(p, pos2(r.right(), cy), Align2::RIGHT_CENTER, &format!("{}×", sig.count), theme::body_semi(15.0), accent_hi(), 0.0);
                    elide(p, pos2(r.left() + 28.0, cy), Align2::LEFT_CENTER, sig.display_type(), theme::body_semi(15.0), name_col, cr.left() - r.left() - 36.0);
                }
            }
            _ if !body.planet_class.is_empty() => {
                ui.add_space(8.0);
                widgets::section(ui, "Signals", None);
                let hint = if body.scan_type == "NavBeaconDetail" {
                    "NavBeacon doesn't include signal data."
                } else {
                    "Use FSS scanner to detect signals."
                };
                widgets::hint(ui, hint);
            }
            _ => {}
        }

        self.body_finds(ui, body_id, settings);
    }

    /// Life on the body in one place, one row per genus: what the commander found of it
    /// (samples, value), else the species the body's atmosphere, temperature, gravity,
    /// volcanism and stars allow (see [`edcas_common::exobiology`]). The genera are the ones
    /// a surface scan named, else the possible ones.
    fn body_life(&self, ui: &mut Ui, body: &BodyScan, settings: &Settings) {
        let body_id = body.body_id;
        let address = self.shown_address();
        let finds: Vec<&Find> = address
            .map_or(&[][..], |a| self.finds.on(a, body_id))
            .iter()
            .filter(|f| f.kind == FindKind::Biological)
            .collect();
        let signals = self.saa_data.get(&body_id).map(|s| &s.signals).or_else(|| self.fss_signals.get(&body_id));
        let bio = signals.map(|sigs| sigs.iter().filter(|s| s.is_biological()).map(|s| s.count).sum::<i32>());
        let scanned: Vec<&GenusSignal> = self.saa_data.get(&body_id).map(|s| s.genuses.iter().collect()).unwrap_or_default();
        // No biological signal on a scanned body, or none after a complete FSS: nothing grows.
        let barren = bio == Some(0) || (bio.is_none() && self.fss_all_bodies_found);
        let mut possible: Vec<Possible> = exobiology_facts(&self.tree, self.shown_position(), body)
            .filter(|_| !barren)
            .map(|facts| possible_species(&facts))
            .unwrap_or_default();
        possible.sort_by_key(|p| std::cmp::Reverse(value(p.species)));

        // Genera: found ones first, then the scan's, else the possible ones.
        let mut rows: Vec<(String, String)> = Vec::new();
        let mut add = |symbol: &str, name: &str| {
            if !rows.iter().any(|(s, _)| s == symbol) {
                rows.push((symbol.to_string(), name.to_string()));
            }
        };
        for f in &finds {
            if let Some(g) = f.genus() {
                let name = scanned.iter().find(|s| s.symbol == g).map(|s| s.name.as_str()).or_else(|| genus_name(g)).unwrap_or(g);
                add(g, name);
            }
        }
        for g in &scanned {
            add(&g.symbol, &g.name);
        }
        if scanned.is_empty() {
            for p in &possible {
                add(&p.species.genus, p.species.genus_name());
            }
        }
        let unplaced: Vec<&&Find> = finds.iter().filter(|f| f.genus().is_none()).collect();
        if rows.is_empty() && unplaced.is_empty() {
            return;
        }

        ui.add_space(8.0);
        widgets::section(ui, "Life", Some(rows.len() + unplaced.len()));
        if scanned.is_empty() && finds.is_empty() {
            widgets::hint(ui, &match bio {
                Some(n) => format!("{n} biological signals. Genera that atmosphere, temperature, gravity and stars allow; a surface scan names the ones here."),
                None => "Genera that atmosphere, temperature, gravity and stars allow. Scan the system to know whether anything grows.".to_string(),
            });
        }
        let mut somewhere = false;
        for (genus, name) in &rows {
            let found: Vec<&&Find> = finds.iter().filter(|f| f.genus() == Some(genus.as_str())).collect();
            let species: Vec<&Possible> = possible.iter().filter(|p| p.species.in_genus(genus)).collect();
            let w = ui.available_width();
            let (r, resp) = ui.allocate_exact_size(vec2(w, ROW + 6.0), Sense::hover());
            let p = ui.painter();
            let cy = r.center().y;
            let art = Rect::from_center_size(pos2(r.left() + 12.0, cy), vec2(24.0, 24.0));
            genus_art::paint(p, art, genus_art::Genus::from_symbol(genus), GREEN);
            let mut right = r.right();
            if found.is_empty() && !species.is_empty() {
                let top = species.iter().map(|s| value(s.species)).max().unwrap_or(0);
                let low = species.iter().map(|s| value(s.species)).min().unwrap_or(0);
                let worth = if low == top { format_credits(top) } else { format!("{} – {}", format_credits(low), format_credits(top)) };
                right = txt(p, pos2(r.right(), cy), Align2::RIGHT_CENTER, &worth, theme::body_semi(14.5), accent_hi(), 0.0).left() - 8.0;
            }
            elide(p, pos2(r.left() + 32.0, cy), Align2::LEFT_CENTER, name, theme::body_semi(15.5), GREEN, right - r.left() - 40.0);
            let mut hover: Vec<String> = genus_art::Genus::from_symbol(genus).map(|g| g.looks().to_string()).into_iter().collect();
            if !found.is_empty() {
                // Scanned: what was found, not what might be.
                for f in found {
                    let short = f.name.strip_prefix(name.as_str()).map(str::trim).filter(|n| !n.is_empty()).unwrap_or(&f.name);
                    self.find_row(ui, address, body_id, f, settings, 32.0, short);
                }
            } else if species.is_empty() {
                wrapped(ui, LayoutJob::single_section("No known species fits these conditions".into(), fmt(theme::body(14.0), TEXT_DIM)), 32.0);
            } else {
                let mut job = LayoutJob::default();
                for (i, s) in species.iter().enumerate() {
                    if i > 0 {
                        job.append("  ·  ", 0.0, fmt(theme::body(14.0), TEXT_DIM));
                    }
                    let short = s.species.name.strip_prefix(name.as_str()).map(str::trim).filter(|n| !n.is_empty()).unwrap_or(&s.species.name);
                    let col = if s.depends_on_location { TEXT_DIM } else { TEXT };
                    job.append(short, 0.0, fmt(theme::body_semi(14.0), col));
                    job.append(&format!(" {}", format_credits(value(s.species))), 0.0, fmt(theme::body(13.0), TEXT_DIM));
                    if s.depends_on_location {
                        job.append("*", 0.0, fmt(theme::body(13.0), YELLOW));
                    }
                }
                wrapped(ui, job, 32.0);
                if species.iter().any(|s| s.depends_on_location) {
                    somewhere = true;
                    hover.push("* only near certain nebulae or in certain galactic regions, which a scan doesn't tell".into());
                }
            }
            if !hover.is_empty() {
                resp.on_hover_text(hover.join("\n"));
            }
        }
        for f in unplaced {
            self.find_row(ui, address, body_id, f, settings, 0.0, &f.name);
        }
        if somewhere {
            widgets::hint(ui, "* also needs a nearby nebula or a certain galactic region");
        }
    }

    /// What the commander found on this body on any visit that isn't life (life is in
    /// [`Self::body_life`]): geological sites.
    fn body_finds(&self, ui: &mut Ui, body_id: i32, settings: &Settings) {
        let Some(address) = self.shown_address() else { return };
        let finds: Vec<&Find> = self.finds.on(address, body_id).iter().filter(|f| f.kind != FindKind::Biological).collect();
        if finds.is_empty() {
            return;
        }
        ui.add_space(8.0);
        widgets::section(ui, "Found here", Some(finds.len()));
        for f in finds {
            self.find_row(ui, Some(address), body_id, f, settings, 0.0, &f.name);
        }
    }

    /// One find: its kind's icon (unless `indent`ed under its genus), `name`, and for
    /// organics the samples taken and what they pay.
    #[allow(clippy::too_many_arguments)]
    fn find_row(&self, ui: &mut Ui, address: Option<i64>, body_id: i32, f: &Find, settings: &Settings, indent: f32, name: &str) {
        let Some(address) = address else { return };
            let bio = f.kind == FindKind::Biological;
        let (glyph, icon_col) = kind_icon(bio, !bio, settings);
        let w = ui.available_width();
        let (r, resp) = ui.allocate_exact_size(vec2(w, ROW), Sense::hover());
        let p = ui.painter();
        let cy = r.center().y;
        if indent == 0.0 {
            txt(p, pos2(r.left() + 12.0, cy), Align2::CENTER_CENTER, &glyph, theme::body(15.0), theme::from_ratatui(icon_col), 0.0);
        }
        let mut x = r.right();
        let mut hover = vec![if bio { "Biological" } else { "Geological" }.to_string()];
        if bio {
            let col = if f.samples >= 3 { GREEN } else { YELLOW };
            for i in 0..3u8 {
                let c = pos2(r.right() - 6.0 - f32::from(2 - i) * 14.0, cy);
                if i < f.samples {
                    p.circle_filled(c, 4.0, col);
                } else {
                    p.circle_stroke(c, 3.5, Stroke::new(1.0, col));
                }
            }
            x -= 50.0;
            hover.push(match f.samples {
                0 => "Not sampled".to_string(),
                3 => "Analysed (3/3 samples)".to_string(),
                n => format!("{n}/3 samples"),
            });
            let value = self.organic_value(body_id, f);
            // Analysed data the commander no longer carries was sold (or lost).
            let unsold = f.species.as_deref().is_some_and(|s| self.unsold.holds_bio(address, body_id, s));
            let sold = f.samples >= 3 && self.unsold.loaded && !unsold;
            // Known from a ship or SRV scan before any sample: shown dim, like sold data.
            if f.samples > 0 || value.is_some() {
                let text = value.map_or("?".to_string(), format_credits);
                let col = if f.samples >= 3 && !sold { accent_hi() } else { TEXT_DIM };
                x = txt(p, pos2(x, cy), Align2::RIGHT_CENTER, &text, theme::body_semi(15.0), col, 0.0).left() - 8.0;
            }
            match value {
                Some(v) => {
                    let mut line = format!("Vista Genomics: {} Cr", format_credits(v));
                    if self.first_footfall(body_id) {
                        line += " (first-footfall bonus ×5 included)";
                    }
                    hover.push(line);
                }
                None if f.samples > 0 => hover.push("Value unknown for this species".into()),
                None => {}
            }
            if sold {
                hover.push("Sold, or lost".into());
            } else if unsold {
                hover.push("Unsold".into());
            }
        }
        if f.new_entry {
            x = tag(p, pos2(x, cy), "New", CYAN).left() - 8.0;
            hover.push("New to your codex".into());
        }
        if let Some(day) = f.first_seen.get(..10) {
            hover.push(format!("First found {day}"));
        }
        let name_col = if bio { GREEN } else { YELLOW };
        let left = if indent > 0.0 { indent } else { 28.0 };
        elide(p, pos2(r.left() + left, cy), Align2::LEFT_CENTER, name, theme::body_semi(15.0), name_col, x - r.left() - left - 8.0);
        resp.on_hover_text(hover.join("\n"));
    }
}

/// How a scanned body's discovery stands for the commander.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Standing {
    /// Nobody has discovered it: selling the data first earns the discoverer tag.
    First,
    Known,
    /// Reports itself undiscovered, but lies in populated space or is already mapped:
    /// charted before discovery tags existed, so it never gets one.
    PreCharted,
}

fn charted(d: &Discovery, populated: bool) -> Standing {
    if d.discovered || d.reported {
        // A body someone reported is known, whatever their scan said.
        Standing::Known
    } else if populated || d.mapped {
        Standing::PreCharted
    } else {
        Standing::First
    }
}

/// The "Discovery" section of a body's details: discovered, mapped and footfall in a word
/// or two, then what the data sells for scanned and mapped. `unsold` says the commander
/// still carries the body's data.
fn discovery(ui: &mut Ui, body: &BodyScan, d: &Discovery, state: Standing, unsold: bool) {
    widgets::section(ui, "Discovery", None);
    let yours = if unsold { "By you, unsold" } else { "By you" };
    let (found, col) = match state {
        Standing::First => (yours, YELLOW),
        Standing::PreCharted => ("Pre-charted", TEXT_DIM),
        Standing::Known => ("Yes", TEXT),
    };
    kv_row(ui, 100.0, "Discovered", found, "", col);
    // Stars and belt clusters can't be mapped.
    if !body.planet_class.is_empty() {
        let (mapped, col) = match body.mapped {
            _ if d.mapped => ("Yes", TEXT),
            Some(_) => (yours, YELLOW),
            None if d.reported => ("No", TEXT_DIM),
            None => ("No", GREEN),
        };
        kv_row(ui, 100.0, "Mapped", mapped, "", col);
    }
    if let Some(landed) = d.footfalled.filter(|_| body.landable) {
        let (foot, col) = match landed {
            true => ("Yes", TEXT),
            // The commander's own first footfall (Disembark), like their own map above.
            false if body.footfall => ("By you, first", YELLOW),
            false if d.reported => ("No", TEXT_DIM),
            false => ("No", CYAN),
        };
        kv_row(ui, 100.0, "Footfall", foot, "", col);
    }
    if let Some(v) = body.value {
        kv_row(ui, 100.0, "Scan value", &format_thousands(v.credits(None)), "Cr", accent_hi());
        if v.mappable() {
            // What the commander's map made it, or what an efficient one would.
            let (mapped, unit) = match body.mapped {
                Some(_) => (v.credits(body.mapped), "Cr"),
                None => (v.credits(Some(true)), "Cr if mapped"),
            };
            kv_row(ui, 100.0, "Map value", &format_thousands(mapped), unit, GREEN);
        }
    }
    if d.reported {
        widgets::hint(ui, "As of the newest scan reported via EDDN.");
    }
}

/// A small filled diamond with its left tip at `left`, the tree's first-discovery mark.
/// Returns its right edge.
fn first_mark(p: &Painter, left: Pos2, col: Color32) -> f32 {
    let c = left + vec2(4.5, 0.0);
    p.add(Shape::convex_polygon(
        vec![c + vec2(0.0, -4.5), c + vec2(4.5, 0.0), c + vec2(0.0, 4.5), c + vec2(-4.5, 0.0)],
        col,
        Stroke::NONE,
    ));
    c.x + 4.5
}

/// Scanned stars and planets: what the FSS's `BodyCount` counts (no belt clusters or
/// barycentres).
fn fss_bodies(nodes: &[TreeNode]) -> u32 {
    let mut n = 0;
    walk_bodies(nodes, &mut |b| n += u32::from(!b.star_type.is_empty() || !b.planet_class.is_empty()));
    n
}

/// Calls `f` for every scanned body in a body tree.
fn walk_bodies(nodes: &[TreeNode], f: &mut impl FnMut(&BodyScan)) {
    for n in nodes {
        if let Some(b) = &n.data {
            f(b);
        }
        walk_bodies(&n.children, f);
    }
}

/// Scanned bodies and stars in a body tree.
fn count_bodies(nodes: &[TreeNode]) -> (usize, usize) {
    nodes.iter().fold((0, 0), |(b, s), n| {
        let (cb, cs) = count_bodies(&n.children);
        let star = n.data.as_ref().is_some_and(|d| !d.star_type.is_empty());
        (b + cb + usize::from(n.data.is_some()), s + cs + usize::from(star))
    })
}

// ---------------------------------------------------------------- system panel parts

/// Colonisation status of the current system: architect and construction depots.
fn construction(ui: &mut Ui, system: &SystemData, journal: &JournalData) {
    let depots: Vec<_> = journal.construction_depots.values().filter(|d| d.system_name == system.name).collect();
    let is_architect = journal.claimed_systems.contains_key(&system.system_address);
    if !is_architect && depots.is_empty() {
        return;
    }
    ui.add_space(6.0);
    widgets::section(ui, "Construction", None);
    if is_architect {
        let cmdr = if journal.pilot.name.is_empty() { "You".to_string() } else { format!("CMDR {}", journal.pilot.name) };
        kv_row(ui, 96.0, "Architect", &cmdr, "", accent_hi());
    }
    for depot in depots {
        let s = &depot.submission;
        let pct = s.progress * 100.0;
        let col = if pct >= 100.0 { GREEN } else if pct > 50.0 { YELLOW } else { CYAN };
        let remaining = s.resources.iter().filter(|r| r.provided_amount < r.required_amount).count();
        let w = ui.available_width();
        let (r, _) = ui.allocate_exact_size(vec2(w, ROW), Sense::hover());
        let p = ui.painter();
        let cy = r.center().y;
        let pr = txt(p, pos2(r.right(), cy), Align2::RIGHT_CENTER, &format!("{pct:.1}%"), theme::body_semi(15.0), col, 0.0);
        let sr = txt(p, pos2(r.left() + 2.0, cy), Align2::LEFT_CENTER, "★", theme::body(14.0), accent(), 0.0);
        elide(p, pos2(sr.right() + 6.0, cy), Align2::LEFT_CENTER, &s.station_name, theme::body_semi(15.0), TEXT, pr.left() - sr.right() - 14.0);
        let (r, resp) = ui.allocate_exact_size(vec2(w, 14.0), Sense::hover());
        let p = ui.painter();
        let tr = txt(p, pos2(r.right(), r.center().y), Align2::RIGHT_CENTER, &format!("{remaining} left"), theme::body(13.5), TEXT_DIM, 0.0);
        widgets::seg_bar(p, Rect::from_min_max(pos2(r.left() + 2.0, r.top() + 3.0), pos2(tr.left() - 10.0, r.bottom() - 3.0)), s.progress, col);
        resp.on_hover_text(format!("{remaining} commodities still needed"));
    }
}

/// One faction under the influence chart: its line's colour, name (★ when it controls the
/// system), change over the chart's period, share, states and conflict. The whole card is
/// clickable.
fn faction_card(ui: &mut Ui, f: &FactionInfo, controlling: bool, selected: bool, colour: Color32, change: Option<f32>) -> Response {
    let bg = ui.painter().add(Shape::Noop);
    let top = ui.cursor().min;
    let w = ui.available_width();
    ui.add_space(3.0);

    let pct = f.influence * 100.0;
    let bar_col = if pct < 15.0 { RED } else if pct < 40.0 { YELLOW } else { GREEN };
    let (r, _) = ui.allocate_exact_size(vec2(w, ROW), Sense::hover());
    let p = ui.painter();
    let cy = r.center().y;
    let pr = txt(p, pos2(r.right() - 6.0, cy), Align2::RIGHT_CENTER, &format!("{pct:.1}%"), theme::body_semi(15.5), bar_col, 0.0);
    let mut right = pr.left();
    if let Some(d) = change {
        let (text, col) = if d >= 0.05 {
            (format!("▲ {d:.1}"), GREEN)
        } else if d <= -0.05 {
            (format!("▼ {:.1}", -d), RED)
        } else {
            ("±0".to_string(), TEXT_DIM)
        };
        right = txt(p, pos2(right - 10.0, cy), Align2::RIGHT_CENTER, &text, theme::body(14.0), col, 0.0).left();
    }
    p.rect_filled(Rect::from_center_size(pos2(r.left() + 12.0, cy), vec2(8.0, 8.0)), 0.0, colour);
    let name_x = r.left() + 22.0;
    let name_col = if controlling { accent_hi() } else { TEXT };
    let nr = elide(p, pos2(name_x, cy), Align2::LEFT_CENTER, &f.name, theme::body_semi(15.5), name_col, right - name_x - 24.0);
    if controlling {
        txt(p, pos2(nr.right() + 5.0, cy), Align2::LEFT_CENTER, "★", theme::body(14.0), accent(), 0.0);
    }

    for (label, states, pending) in [("Active", &f.active_states, false), ("Pending", &f.pending_states, true), ("Recovering", &f.recovering_states, false)] {
        if states.is_empty() {
            continue;
        }
        let list: Vec<String> = states.iter().map(|s| crate::views::annotate_faction_state(s, pending)).collect();
        let mut job = LayoutJob::default();
        job.append(&format!("{label}  "), 0.0, fmt(theme::body(13.5), CYAN));
        job.append(&list.join(", "), 0.0, fmt(theme::body(14.0), TEXT_DIM));
        wrapped(ui, job, 8.0);
    }
    if let Some(c) = &f.conflict {
        conflict(ui, c);
    }
    ui.add_space(4.0);

    let rect = Rect::from_min_max(top, pos2(top.x + w, ui.cursor().min.y));
    let resp = ui.interact(rect, ui.id().with(("faction", &f.name)), Sense::click());
    let acc = accent();
    let shape = if selected {
        Shape::Vec(vec![
            Shape::rect_filled(rect, 0.0, alpha(acc, 40)),
            Shape::rect_filled(Rect::from_min_size(rect.min, vec2(3.0, rect.height())), 0.0, acc),
        ])
    } else if resp.hovered() {
        Shape::Vec(vec![
            Shape::rect_filled(rect, 0.0, alpha(acc, 18)),
            Shape::rect_filled(Rect::from_min_size(rect.min, vec2(2.0, rect.height())), 0.0, alpha(acc, 160)),
        ])
    } else {
        Shape::Noop
    };
    ui.painter().set(bg, shape);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Next and last BGS tick rows of the overview, from the server's tick detection.
fn tick_rows(ui: &mut Ui, h: &SystemHistory) {
    let now = chrono::Utc::now();
    match &h.next_tick {
        Some(p) => {
            let window = chrono::Duration::minutes(p.window_minutes as i64);
            let (value, col) = if now > p.next_tick + window {
                ("overdue".to_string(), TEXT_DIM)
            } else if now >= p.next_tick - window {
                ("due now".to_string(), YELLOW)
            } else {
                (format!("in {}", span(p.next_tick - now)), GREEN)
            };
            let unit = format!("{} UTC ±{}m", p.next_tick.format("%H:%M"), p.window_minutes);
            kv_row(ui, 96.0, "Next tick", &value, &unit, col);
        }
        None => kv_row(ui, 96.0, "Next tick", "unknown", "too few ticks seen", TEXT_DIM),
    }
    if let Some(t) = h.last_tick {
        kv_row(ui, 96.0, "Last tick", &format!("{} ago", span(now - t)), &t.format("%b %d %H:%M UTC").to_string(), TEXT);
    }
}

/// A duration as "42m", "5h 07m" or "3d 4h".
fn span(d: chrono::Duration) -> String {
    let m = d.num_minutes().max(0);
    if m < 60 {
        format!("{m}m")
    } else if m < 48 * 60 {
        format!("{}h {:02}m", m / 60, m % 60)
    } else {
        format!("{}d {}h", m / 1440, (m % 1440) / 60)
    }
}

/// War / election lines under a faction.
fn conflict(ui: &mut Ui, c: &ConflictData) {
    let type_label = match c.war_type.to_lowercase().as_str() {
        "war" => "War",
        "civilwar" => "Civil War",
        "election" => "Election",
        _ => "Conflict",
    };
    let status = if c.status.is_empty() { "active" } else { &c.status };
    let mut job = LayoutJob::default();
    job.append(&format!("{type_label} ({status})"), 0.0, fmt(theme::body_semi(14.5), accent()));
    job.append("  vs  ", 0.0, fmt(theme::body(14.0), GREY));
    job.append(&c.opponent, 0.0, fmt(theme::body_semi(14.5), YELLOW));
    wrapped(ui, job, 8.0);
    let mut job = LayoutJob::default();
    job.append("Score  ", 0.0, fmt(theme::body(13.5), CYAN));
    job.append(&format!("{} – {}", c.our_won_days, c.opponent_won_days), 0.0, fmt(theme::body_semi(14.5), accent()));
    wrapped(ui, job, 8.0);
}

// ---------------------------------------------------------------- tree rows

/// One tree row: guide lines from the row's `├─`/`│ ` prefix, expander, icon, name,
/// distance and markers. Returns the row's response, whether the row has children and
/// whether the click landed on the expander.
/// `first` marks an undiscovered body (a first discovery) with a gold diamond after its name;
/// `valuable` tags one worth finding first (an undiscovered water world, say) "Valuable" beside
/// its distance.
#[allow(clippy::too_many_arguments)]
fn tree_row(ui: &mut Ui, node: &FlatNode, sel: bool, first: bool, valuable: bool, next_depth: usize, system: &str, settings: &Settings) -> (Response, bool, bool) {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, TREE_ROW), Sense::click());
    let depth = tree_depth(&node.tree_prefix);
    let has_kids = matches!(node.node_type, NodeType::Body) && (node.hidden > 0 || (depth > 0 && next_depth > depth));
    let icon_x = rect.left() + BASE + (depth.max(1) - 1) as f32 * INDENT;
    let on_expander = has_kids && resp.interact_pointer_pos().is_some_and(|p| p.x < icon_x - RING_MAX + 1.5);
    if !ui.is_rect_visible(rect) {
        return (resp, has_kids, on_expander);
    }
    let p = ui.painter();
    let acc = accent();
    let cy = rect.center().y;
    let on = |c: Color32| widgets::row_text_color(sel, c);

    let hl = Rect::from_min_max(pos2(icon_x - 23.0, rect.top() + 1.0), pos2(rect.right() - 2.0, rect.bottom() - 1.0));
    if sel {
        p.rect_filled(hl, 0.0, acc);
        let n = hl.left();
        p.add(Shape::convex_polygon(vec![pos2(n - 5.0, cy - 4.0), pos2(n - 1.0, cy), pos2(n - 5.0, cy + 4.0)], accent_hi(), Stroke::NONE));
    } else if resp.hovered() {
        p.rect_filled(hl, 0.0, alpha(acc, 28));
        p.line_segment([hl.left_top(), hl.left_bottom()], Stroke::new(2.0, acc));
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let guide = Stroke::new(1.0, alpha(acc, 70));
    let chunks: Vec<char> = node.tree_prefix.chars().step_by(3).collect();
    for (k, ch) in chunks.iter().enumerate().skip(1) {
        let x = rect.left() + BASE + (k - 1) as f32 * INDENT;
        match ch {
            '│' | '├' => {
                p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], guide);
            }
            '└' => {
                p.line_segment([pos2(x, rect.top()), pos2(x, cy)], guide);
            }
            _ => {}
        }
        if k == chunks.len() - 1 {
            let end = icon_x - if has_kids { 21.0 } else { RING_MAX };
            p.line_segment([pos2(x, cy), pos2(end, cy)], guide);
        }
    }
    // A belt with its clusters below gets the stub down to them, though it doesn't fold.
    let belt_kids = matches!(node.node_type, NodeType::Ring(_)) && next_depth > depth;
    if (has_kids && node.hidden == 0 || belt_kids) && !sel {
        p.line_segment([pos2(icon_x, cy + if belt_kids { 5.0 } else { RING_MAX }), pos2(icon_x, rect.bottom())], guide);
    }
    if has_kids {
        let tc = pos2(icon_x - 17.0, cy);
        let col = if sel { BG } else if on_expander || resp.hovered() { accent_hi() } else { accent_dim() };
        let tri = if node.hidden == 0 {
            vec![tc + vec2(-3.5, -2.0), tc + vec2(3.5, -2.0), tc + vec2(0.0, 2.5)]
        } else {
            vec![tc + vec2(-2.0, -3.5), tc + vec2(2.5, 0.0), tc + vec2(-2.0, 3.5)]
        };
        p.add(Shape::convex_polygon(tri, col, Stroke::NONE));
    }

    // Icon and name.
    let c = pos2(icon_x, cy);
    let (suffix, name_col, font) = match &node.node_type {
        NodeType::Body => {
            let col = paint_body_icon(p, c, node, settings, sel);
            if node.is_barycentre {
                ("", GREY, theme::body(15.0))
            } else if !node.star_type.is_empty() {
                (node.star_type.as_str(), mix(col, TEXT, 0.3), theme::body_semi(15.5))
            } else if node.planet_class.is_empty() {
                ("", col, theme::body(15.0))
            } else {
                ("", mix(col, TEXT, 0.45), theme::body_semi(15.5))
            }
        }
        NodeType::Station(_) => {
            p.circle_stroke(c, 5.0, Stroke::new(1.2, on(CYAN)));
            p.circle_filled(c, 2.0, on(CYAN));
            ("", TEXT, theme::body_semi(15.0))
        }
        NodeType::Signal(sig) => {
            let (glyph, col) = signal_icon(sig, settings);
            txt(p, c, Align2::CENTER_CENTER, &glyph, theme::body(14.0), on(theme::from_ratatui(col)), 0.0);
            ("", TEXT, theme::body(15.0))
        }
        NodeType::BodySignal(sig) => {
            let (glyph, col) = body_signal_icon(sig, settings);
            let col = theme::from_ratatui(col);
            txt(p, c, Align2::CENTER_CENTER, &glyph, theme::body(14.0), on(col), 0.0);
            ("", mix(col, TEXT, 0.5), theme::body(15.0))
        }
        NodeType::Ring(ring) => {
            let col = ring_colour(&ring.ring_class, settings);
            p.add(Shape::ellipse_stroke(c, vec2(7.0, 2.6), Stroke::new(1.4, on(col))));
            ("", mix(col, TEXT, 0.5), theme::body(15.0))
        }
        NodeType::SectionHeader => ("", TEXT, theme::body(15.0)),
    };
    let dist_right = rect.right() - LAND_W;
    // The distance first, and the tag left of it, so the name stops short of both.
    let mut right = dist_right - 70.0;
    if node.distance_ls > 0.0 {
        let ur = txt(p, pos2(dist_right, cy), Align2::RIGHT_CENTER, "Ls", theme::body(12.5), on(widgets::mute()), 0.0);
        let nr = txt(p, pos2(ur.left() - 3.0, cy), Align2::RIGHT_CENTER, &format!("{:.1}", node.distance_ls), theme::body_semi(14.5), on(accent_dim()), 0.0);
        right = right.min(nr.left() - 8.0);
    }
    if valuable {
        right = tag(p, pos2(right.min(dist_right - 70.0), cy), "Valuable", on(RED)).left() - 8.0;
    }
    let name_x = icon_x + RING_REACH + 4.0;
    let name_max = (right - name_x).max(30.0);
    let nr = elide(p, pos2(name_x, cy), Align2::LEFT_CENTER, &node.short_name, font, on(name_col), name_max);
    let mut x = nr.right() + 6.0;
    if !suffix.is_empty() && x < right - 20.0 {
        x = txt(p, pos2(x, cy), Align2::LEFT_CENTER, &format!("({suffix})"), theme::body(13.5), on(TEXT_DIM), 0.0).right() + 6.0;
    }
    if first {
        x = first_mark(p, pos2(x + 4.0, cy), on(YELLOW)) + 6.0;
    }
    if node.hidden > 0 && x < right - 20.0 {
        let r = txt(p, pos2(x + 4.0, cy), Align2::LEFT_CENTER, &format!("+{}", node.hidden), theme::head(8.0), on(TEXT_DIM), 0.6);
        p.rect_stroke(r.expand2(vec2(4.0, 2.0)), 1.0, Stroke::new(1.0, on(GREY)), egui::StrokeKind::Inside);
    }

    // Markers: landable (bodies), threat (signals). Bio and geo counts are rows of their own.
    let mx = dist_right + 10.0;
    if node.landable {
        let br = Rect::from_center_size(pos2(mx + 7.0, cy), vec2(14.0, 14.0));
        p.rect_stroke(br, 1.0, Stroke::new(1.0, on(CYAN)), egui::StrokeKind::Inside);
        txt(p, br.center() + vec2(0.5, 0.5), Align2::CENTER_CENTER, "L", theme::head_bold(8.5), on(CYAN), 0.0);
    }
    if let NodeType::Ring(ring) = &node.node_type {
        let valuable = is_valuable_ring(&ring.ring_class);
        let mut x = dist_right;
        if valuable {
            x = tag(p, pos2(x, cy), "Valuable", on(RED)).left() - 8.0;
        }
        let col = if valuable { RED } else { TEXT_DIM };
        txt(p, pos2(x, cy), Align2::RIGHT_CENTER, &ring_class_name(&ring.ring_class), theme::body_semi(14.5), on(col), 0.0);
    }
    if let NodeType::Signal(sig) = &node.node_type {
        if let Some(t) = sig.threat_level.filter(|&t| t > 0) {
            txt(p, pos2(rect.right() - 6.0, cy), Align2::RIGHT_CENTER, &format!("⚠ {t}"), theme::body_semi(14.5), on(RED), 0.0);
        }
    }

    let resp = resp.on_hover_ui(|ui| row_tooltip(ui, node, system, has_kids, first));
    (resp, has_kids, on_expander)
}

/// What a tree row is, in words, plus how to fold it.
fn row_tooltip(ui: &mut Ui, node: &FlatNode, system: &str, has_kids: bool, first: bool) {
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    let mut lines: Vec<String> = Vec::new();
    match &node.node_type {
        NodeType::Body => {
            let lower = node.short_name.to_ascii_lowercase();
            lines.push(if node.is_barycentre {
                "Barycentre".to_string()
            } else if !node.planet_class.is_empty() {
                node.planet_class.clone()
            } else if !node.star_type.is_empty() {
                format!("Star ({})", node.star_type)
            } else if is_belt_cluster(node) {
                "Belt cluster".to_string()
            } else if lower.ends_with("ring") {
                "Ring".to_string()
            } else {
                "Not scanned".to_string()
            });
            if first {
                lines.push("Undiscovered: a first discovery".into());
            }
            if node.landable {
                lines.push("Landable".into());
            }
            if node.has_rings {
                lines.push("Has rings".into());
            }
            if node.bio_signal_count > 0 {
                lines.push(format!("Biological signals: {}", node.bio_signal_count));
            }
            if node.geo_signal_count > 0 {
                lines.push(format!("Geological signals: {}", node.geo_signal_count));
            }
        }
        NodeType::Station(st) => lines.push(if st.station_type.is_empty() { "Station".into() } else { st.station_type.clone() }),
        NodeType::Signal(sig) => lines.push(sig.uss_type.clone().unwrap_or_else(|| "Signal source".into())),
        NodeType::BodySignal(_) => lines.push("Signals on the body above".into()),
        NodeType::Ring(ring) => {
            lines.push(format!("{} ring of the body above", ring_class_name(&ring.ring_class)));
            if is_valuable_ring(&ring.ring_class) {
                lines.push("Valuable for mining".into());
            }
        }
        NodeType::SectionHeader => {}
    }
    let title = match node.node_type {
        NodeType::Body => format!("{system} {}", node.short_name).trim().to_string(),
        _ => node.short_name.clone(),
    };
    ui.label(egui::RichText::new(title).font(theme::body_semi(15.0)).color(accent_hi()));
    for l in lines {
        ui.label(egui::RichText::new(l).font(theme::body(14.5)).color(TEXT));
    }
    if has_kids {
        let action = if node.hidden > 0 { "unfold" } else { "fold" };
        ui.label(egui::RichText::new(format!("Click the arrow or double-click to {action}")).font(theme::body(13.5)).color(TEXT_DIM));
    }
}

/// Draws a body's icon (star glow, planet disc sized by radius, belt, barycentre) in the
/// colour the settings give it, and returns that colour.
fn paint_body_icon(p: &Painter, c: Pos2, node: &FlatNode, settings: &Settings, sel: bool) -> Color32 {
    let style = node_style(node, settings);
    let col = style.fg.map(theme::from_ratatui).unwrap_or(TEXT);
    // The ring stays within RING_REACH of the centre, clear of the expander to its left.
    let ring = |r: f32| {
        if node.has_rings {
            let rc = if sel { BG } else { mix(col, TEXT, 0.3) };
            let rx = (r * 1.5).min(RING_REACH);
            p.add(Shape::ellipse_stroke(c, vec2(rx, rx * 0.36), Stroke::new(1.2, rc)));
        }
    };
    if node.planet_class.is_empty() {
        if node.is_barycentre {
            let s = Stroke::new(1.0, if sel { BG } else { GREY });
            p.circle_stroke(c, 5.5, s);
            p.line_segment([c - vec2(9.0, 0.0), c + vec2(9.0, 0.0)], s);
            p.line_segment([c - vec2(0.0, 9.0), c + vec2(0.0, 9.0)], s);
            return GREY;
        }
        let lower = node.short_name.to_ascii_lowercase();
        if is_belt_cluster(node) {
            for (dx, dy, r) in [(-3.5, 1.5, 1.6), (0.0, -2.5, 1.9), (3.5, 1.8, 1.5), (0.5, 3.6, 1.0)] {
                p.circle_filled(c + vec2(dx, dy) * 1.4, r * 1.4, if sel { BG } else { col });
            }
            return col;
        }
        if node.star_type.is_empty() {
            // A ring or a body known only by name (no scan yet).
            let col = if lower.ends_with("ring") { TEXT_DIM } else { accent_dim() };
            let s = Stroke::new(1.2, if sel { BG } else { col });
            if lower.ends_with("ring") {
                p.add(Shape::ellipse_stroke(c, vec2(9.5, 3.6), s));
                p.add(Shape::ellipse_stroke(c, vec2(6.0, 2.1), s));
            } else {
                p.circle_stroke(c, 5.0, s);
            }
            return col;
        }
        if !sel {
            p.circle_filled(c, 12.0, alpha(col, 26));
            p.circle_filled(c, 9.5, alpha(col, 55));
        }
        p.circle_filled(c, 7.0, if sel { BG } else { col });
        p.circle_filled(c, 3.0, if sel { col } else { mix(col, Color32::WHITE, 0.6) });
        ring(7.0);
        return col;
    }
    let lower = node.planet_class.to_lowercase();
    let gas = lower.contains("gas") || lower.contains("sudarsky") || lower.contains("giant");
    let r = disc_radius(node.radius);
    // A ringed giant's disc shrinks so its (capped) ring still reads as a ring around it.
    let r = if node.has_rings { r.min(RING_REACH / 1.5) } else { r };
    if sel {
        p.circle_filled(c, r + 1.5, BG);
    }
    p.circle_filled(c, r, col);
    if gas {
        let band = Stroke::new(1.0, alpha(Color32::BLACK, 70));
        p.line_segment([c + vec2(-r * 0.8, -r * 0.3), c + vec2(r * 0.8, -r * 0.3)], band);
        p.line_segment([c + vec2(-r * 0.75, r * 0.35), c + vec2(r * 0.75, r * 0.35)], band);
    }
    p.circle_filled(c + vec2(r * 0.35, r * 0.35), r * 0.6, alpha(Color32::BLACK, 70));
    ring(r);
    col
}

/// A planet's disc radius in the tree for its radius in metres (0 when unknown): 300 km
/// moons get [`DISC_MIN`], gas giants of 70 000 km [`DISC_MAX`], on a log scale between.
fn disc_radius(metres: f32) -> f32 {
    if metres <= 0.0 {
        return 5.0;
    }
    let (small, large) = (3.0e5_f32.log10(), 7.0e7_f32.log10());
    let t = ((metres.log10() - small) / (large - small)).clamp(0.0, 1.0);
    DISC_MIN + t * (DISC_MAX - DISC_MIN)
}

// ---------------------------------------------------------------- detail parts

/// Detail heading: a caption, the name large with badges beside it, and a subtitle.
fn detail_header(ui: &mut Ui, caption: &str, title: &str, subtitle: &str, badges: &[(String, Color32)], glyph: Option<(String, Color32)>) {
    ui.label(widgets::job(&caption.to_uppercase(), theme::head(8.5), accent_dim(), 1.4));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 4.0);
        if let Some((g, col)) = glyph {
            ui.label(egui::RichText::new(g).font(theme::body(24.0)).color(col));
        }
        let font = if title.chars().count() <= 14 { theme::head_bold(20.0) } else { theme::body_semi(24.0) };
        ui.label(egui::RichText::new(title).font(font).color(accent_hi()));
        for (text, col) in badges {
            widgets::badge(ui, text, *col);
        }
    });
    if !subtitle.is_empty() {
        ui.label(egui::RichText::new(subtitle).font(theme::body_semi(16.5)).color(TEXT));
    }
    ui.add_space(6.0);
}

/// A large figure with a small caption above and its unit beside it.
fn readout(ui: &mut Ui, label: &str, value: &str, unit: &str) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(widgets::job(&label.to_uppercase(), theme::head(7.5), accent_dim(), 1.4));
        let mut job = LayoutJob::default();
        job.append(value, 0.0, fmt(theme::body_semi(26.0), TEXT));
        job.append(&format!(" {}", unit.to_uppercase()), 0.0, fmt(theme::head(9.0), accent_dim()));
        ui.label(job);
    });
}

fn station_detail(ui: &mut Ui, station: &StationData, system: &str) {
    let caption = if station.system_name.is_empty() { system } else { &station.system_name };
    detail_header(ui, caption, &station.name, &station.station_type, &[], None);
    if station.dist_from_star_ls > 0.0 {
        readout(ui, "Dist. from star", &format!("{:.2}", station.dist_from_star_ls), "Ls");
        ui.add_space(6.0);
    }
    widgets::section(ui, "Location", None);
    if station.dist_from_star_ls > 0.0 {
        kv_row(ui, 110.0, "Distance", &format!("{:.2}", station.dist_from_star_ls), "Ls", TEXT);
    }
    if !station.allegiance.is_empty() {
        kv_row(ui, 110.0, "Allegiance", &station.allegiance, "", TEXT);
    }
    if !station.faction.is_empty() {
        kv_row(ui, 110.0, "Faction", &station.faction, "", GREEN);
    }
    if !station.government.is_empty() {
        kv_row(ui, 110.0, "Government", &station.government, "", TEXT);
    }
    if !station.economy.is_empty() {
        let econ = if station.secondary_economies.is_empty() {
            station.economy.clone()
        } else {
            let secs: Vec<String> = station.secondary_economies.iter().map(|(n, p)| format!("{} ({:.0}%)", pretty(n), p * 100.0)).collect();
            format!("{} / {}", station.economy, secs.join(", "))
        };
        kv_row(ui, 110.0, "Economy", &econ, "", TEXT);
    }
    if let Some((s, m, l)) = station.landing_pads {
        kv_row(ui, 110.0, "Landing Pads", &format!("S {s}  ·  M {m}  ·  L {l}"), "", TEXT);
    }
    if !station.services.is_empty() {
        ui.add_space(8.0);
        widgets::section(ui, "Services", Some(station.services.len()));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
            for service in &station.services {
                let g = ui.painter().layout_no_wrap(service.clone(), theme::body(14.5), TEXT);
                let (r, _) = ui.allocate_exact_size(g.size() + vec2(12.0, 4.0), Sense::hover());
                let p = ui.painter();
                p.rect(r, 0.0, alpha(accent(), 14), Stroke::new(1.0, alpha(accent(), 70)), egui::StrokeKind::Inside);
                p.galley(r.min + vec2(6.0, 2.0), g, TEXT);
            }
        });
    }
}

fn signal_detail(ui: &mut Ui, sig: &DiscoveredSignal, settings: &Settings) {
    let (glyph, col) = signal_icon(sig, settings);
    let mut badges = Vec::new();
    if sig.is_station {
        badges.push(("Station".to_string(), CYAN));
    }
    let subtitle = sig.signal_type.as_deref().map(words).unwrap_or_default();
    detail_header(ui, "Signal source", &sig.display_name, &subtitle, &badges, Some((glyph, theme::from_ratatui(col))));
    if let Some(uss) = &sig.uss_type {
        widgets::section(ui, "USS", None);
        kv_row(ui, 110.0, "Type", uss, "", TEXT);
        if let Some(threat) = sig.threat_level {
            let tc = if threat >= 4 { RED } else if threat >= 2 { accent() } else { YELLOW };
            kv_row(ui, 110.0, "Threat", &threat.to_string(), "", tc);
        }
        ui.add_space(8.0);
    }
    if sig.spawning_faction.is_some() || sig.spawning_state.is_some() {
        widgets::section(ui, "Origin", None);
        if let Some(f) = &sig.spawning_faction {
            kv_row(ui, 110.0, "Faction", f, "", GREEN);
        }
        if let Some(s) = &sig.spawning_state {
            kv_row(ui, 110.0, "State", s, "", TEXT);
        }
        ui.add_space(8.0);
    }
    if sig.is_station {
        kv_row(ui, 110.0, "Type", "Station / Installation", "", TEXT);
    }
    if let Some(remaining) = sig.time_remaining {
        let (m, s) = ((remaining / 60.0) as u32, (remaining % 60.0) as u32);
        kv_row(ui, 110.0, "Expires in", &format!("{m}m {s}s"), "", YELLOW);
    }
}

// ---------------------------------------------------------------- small helpers

/// A label/value row whose value wraps under itself; `unit` is drawn small after the value.
fn kv_row(ui: &mut Ui, key_w: f32, key: &str, value: &str, unit: &str, color: Color32) {
    let w = ui.available_width();
    let value = if value.is_empty() { "—" } else { value };
    let mut job = LayoutJob::default();
    job.append(value, 0.0, fmt(theme::body_semi(15.5), color));
    if !unit.is_empty() {
        job.append(&format!(" {unit}"), 0.0, fmt(theme::body(13.0), TEXT_DIM));
    }
    job.wrap.max_width = (w - key_w).max(40.0);
    let galley = ui.painter().layout_job(job);
    let h = galley.size().y.max(18.0) + 3.0;
    let (r, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter();
    let kr = txt(p, pos2(r.left() + 4.0, r.top() + 1.0), Align2::LEFT_TOP, key, theme::body(15.0), CYAN, 0.0);
    p.galley(pos2(r.left() + key_w, kr.top()), galley, color);
    p.line_segment([pos2(r.left() + 4.0, r.bottom() - 0.5), pos2(r.right(), r.bottom() - 0.5)], Stroke::new(1.0, alpha(accent(), 14)));
}

/// A named percentage with a segmented bar; `inv` adds the inventory count (×n) at the end,
/// orange when you have none and yellow when few.
fn bar_row(ui: &mut Ui, name: &str, percent: f64, scale: f64, col: Color32, inv: Option<i32>) {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, ROW), Sense::hover());
    let p = ui.painter();
    let cy = r.center().y;
    let mut right = r.right();
    if let Some(n) = inv {
        let ic = if n == 0 { accent() } else if n <= 5 { YELLOW } else { GREY };
        txt(p, pos2(right, cy), Align2::RIGHT_CENTER, &format!("×{n}"), theme::body_semi(14.5), ic, 0.0);
        right -= 40.0;
        resp.on_hover_text(format!("{n} in your inventory"));
    }
    let pr = txt(p, pos2(right, cy), Align2::RIGHT_CENTER, "%", theme::body(12.5), accent_dim(), 0.0);
    txt(p, pos2(pr.left() - 2.0, cy), Align2::RIGHT_CENTER, &format!("{percent:.1}"), theme::body_semi(15.0), col, 0.0);
    let label_w = 104.0;
    elide(p, pos2(r.left() + 4.0, cy), Align2::LEFT_CENTER, name, theme::body(15.0), TEXT, label_w - 8.0);
    let bar = Rect::from_min_max(pos2(r.left() + label_w, cy - 4.0), pos2(right - 48.0, cy + 4.0));
    if bar.width() > 12.0 {
        widgets::seg_bar(p, bar, (percent / scale) as f32, col);
    }
}

/// A small outlined tag painted right-aligned at `pos`.
/// A ring class's colour from the icon settings (cyan when unset).
fn ring_colour(ring_class: &str, settings: &Settings) -> Color32 {
    settings.icons.get(ring_class).filter(|i| i.enabled)
        .map(|i| theme::from_ratatui(parse_color(&i.color)))
        .unwrap_or(CYAN)
}

fn tag(p: &Painter, pos: Pos2, text: &str, col: Color32) -> Rect {
    let g = p.layout_job(widgets::job(&text.to_uppercase(), theme::head(7.5), col, 1.2));
    let r = Align2::RIGHT_CENTER.anchor_size(pos, g.size() + vec2(10.0, 5.0));
    p.rect(r, 0.0, alpha(col, 22), Stroke::new(1.0, col), egui::StrokeKind::Inside);
    p.galley(r.min + vec2(5.0, 3.0), g, col);
    r
}

/// Paints `text` on one line, cut with "…" when it is wider than `max_w`.
fn elide(p: &Painter, pos: Pos2, align: Align2, text: &str, font: FontId, color: Color32, max_w: f32) -> Rect {
    let mut job = LayoutJob::single_section(text.to_owned(), fmt(font, color));
    job.wrap = TextWrapping { max_width: max_w.max(12.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    let g = p.layout_job(job);
    let r = align.anchor_size(pos, g.size());
    p.galley(r.min, g, color);
    r
}

/// What a species' first sample sells for (0 when the payout table doesn't know it).
fn value(species: &Species) -> i64 {
    edcas_common::exploration::exobio_value(&species.symbol, &species.genus).unwrap_or(0)
}

/// Allocates and paints `job` wrapped to the available width, indented by `indent`.
fn wrapped(ui: &mut Ui, mut job: LayoutJob, indent: f32) {
    let w = ui.available_width();
    job.wrap.max_width = (w - indent - 4.0).max(20.0);
    let g = ui.painter().layout_job(job);
    let (r, _) = ui.allocate_exact_size(vec2(w, g.size().y), Sense::hover());
    ui.painter().galley(pos2(r.left() + indent, r.top()), g, TEXT);
}

fn fmt(font_id: FontId, color: Color32) -> TextFormat {
    TextFormat { font_id, color, ..Default::default() }
}

/// Journal symbols as words: `$economy_HighTech;` → "High Tech", `$SYSTEM_SECURITY_high;`
/// → "High", `economy_Agri` → "Agri". Names that are already plain pass through.
pub(crate) fn pretty(raw: &str) -> String {
    if !raw.starts_with('$') && !raw.starts_with("economy_") {
        return raw.to_owned();
    }
    let inner = raw.trim_start_matches('$').trim_end_matches(';');
    words(inner.rsplit('_').next().unwrap_or(inner))
}

/// `ResourceExtraction` → "Resource Extraction", first letter capitalised.
fn words(word: &str) -> String {
    let mut out = String::new();
    for (i, ch) in word.chars().enumerate() {
        if i == 0 {
            out.extend(ch.to_uppercase());
        } else {
            if ch.is_uppercase() {
                out.push(' ');
            }
            out.push(ch);
        }
    }
    out
}

/// Whether the primary button was clicked inside `r` this frame.
fn clicked_in(ui: &Ui, r: Rect) -> bool {
    ui.input(|i| i.pointer.primary_clicked() && i.pointer.interact_pos().is_some_and(|p| r.contains(p)))
}

/// Marks the panel that has keyboard focus with a full-strength outline.
fn paint_focus(p: &Painter, r: Rect) {
    let c = 12.0;
    let pts = vec![
        r.left_top(),
        pos2(r.right() - c, r.top()),
        pos2(r.right(), r.top() + c),
        r.right_bottom(),
        pos2(r.left() + c, r.bottom()),
        pos2(r.left(), r.bottom() - c),
    ];
    p.add(Shape::closed_line(pts, Stroke::new(1.5, accent_hi())));
}

#[cfg(test)]
mod tick_tests {
    use super::span;

    #[test]
    fn countdowns_read_naturally() {
        assert_eq!(span(chrono::Duration::minutes(42)), "42m");
        assert_eq!(span(chrono::Duration::minutes(5 * 60 + 7)), "5h 07m");
        assert_eq!(span(chrono::Duration::minutes(3 * 1440 + 4 * 60 + 59)), "3d 4h");
        assert_eq!(span(chrono::Duration::minutes(-5)), "0m");
    }
}

#[cfg(test)]
mod discovery_tests {
    use super::{charted, Discovery, Standing};

    fn scan(discovered: bool, mapped: bool) -> Discovery {
        Discovery { discovered, mapped, footfalled: None, reported: false }
    }

    #[test]
    fn only_unpopulated_unmapped_bodies_are_first_discoveries() {
        assert_eq!(charted(&scan(false, false), false), Standing::First);
        assert_eq!(charted(&scan(true, false), false), Standing::Known);
        // Sol and the rest of the bubble report WasDiscovered:false forever.
        assert_eq!(charted(&scan(false, false), true), Standing::PreCharted);
        assert_eq!(charted(&scan(false, true), false), Standing::PreCharted);
        // Someone else's report means someone has been there.
        assert_eq!(charted(&Discovery { reported: true, ..scan(false, false) }, false), Standing::Known);
    }
}
