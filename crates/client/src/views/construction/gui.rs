//! Desktop-window drawing for construction sites: overall progress and the resource table,
//! shown inside the Stations detail panel.

use std::collections::HashMap;

use eframe::egui::{self, pos2, vec2, Align, Color32, Layout, Rect, Sense, Ui};
use edcas_common::api::ConstructionDepotResponse;
use edcas_common::format::group_thousands;
use egui_extras::{Column, TableBuilder};

use crate::gui::theme::{self, accent, alpha, GREEN, GREY, TEXT, TEXT_DIM, YELLOW};
use crate::gui::widgets;
use crate::journal_reader::ConstructionDepotData;
use crate::views::util::normalize_commodity_name;

/// One commodity a depot needs.
pub(in crate::views) struct ResourceRow {
    name: String,
    display_name: String,
    provided: i32,
    required: i32,
    payment: i64,
}

/// A construction depot as shown in the window, from live journal data or the API.
pub(in crate::views) struct DepotSummary {
    station_name: String,
    system_name: Option<String>,
    updated: Option<String>,
    progress: f32,
    complete: bool,
    failed: bool,
    /// Unfinished resources first, then by name — the order `f` and the row index refer to.
    resources: Vec<ResourceRow>,
}

/// What the user did with a resource row.
pub(in crate::views) enum ResourceAction {
    Select(usize),
    /// Find the nearest station selling this resource (double-click or context menu).
    Nearest(usize),
}

impl DepotSummary {
    /// From the depot the journal reported this session (most up-to-date counts).
    pub(in crate::views) fn from_local(depot: &ConstructionDepotData) -> Self {
        let s = &depot.submission;
        Self::new(
            &s.station_name,
            None,
            None,
            (s.progress, s.construction_complete, s.construction_failed),
            s.resources.iter().map(|r| ResourceRow {
                name: r.name.clone(),
                display_name: r.display_name.clone(),
                provided: r.provided_amount,
                required: r.required_amount,
                payment: r.payment,
            }),
        )
    }

    /// From the server's copy (cross-session data).
    pub(in crate::views) fn from_response(depot: &ConstructionDepotResponse) -> Self {
        Self::new(
            &depot.station_name,
            Some(depot.system_name.clone()).filter(|s| !s.is_empty()),
            depot.last_updated.map(|ts| ts.format("%Y-%m-%dT%H:%M:%S").to_string()),
            (depot.progress, depot.construction_complete, depot.construction_failed),
            depot.resources.iter().map(|r| ResourceRow {
                name: r.name.clone(),
                display_name: r.display_name.clone(),
                provided: r.provided_amount,
                required: r.required_amount,
                payment: r.payment,
            }),
        )
    }

    /// Display name of the resource at row `idx`, in display order.
    pub(in crate::views) fn resource_name(&self, idx: usize) -> Option<&str> {
        self.resources.get(idx).map(|r| r.display_name.as_str())
    }

    fn new(
        station_name: &str,
        system_name: Option<String>,
        updated: Option<String>,
        (progress, complete, failed): (f32, bool, bool),
        resources: impl Iterator<Item = ResourceRow>,
    ) -> Self {
        let mut resources: Vec<ResourceRow> = resources.collect();
        resources.sort_by(|a, b| {
            let done_a = a.provided >= a.required;
            let done_b = b.provided >= b.required;
            done_a.cmp(&done_b).then(a.display_name.cmp(&b.display_name))
        });
        Self { station_name: station_name.to_string(), system_name, updated, progress, complete, failed, resources }
    }
}

/// Table row highlights quiet enough that coloured cell text stays readable on them.
pub(in crate::views) fn soften_table_highlights(ui: &mut Ui) {
    let v = ui.visuals_mut();
    v.selection.bg_fill = alpha(accent(), 56);
    v.widgets.hovered.bg_fill = alpha(accent(), 26);
}

/// An accent bar at the left edge of a table cell, marking the selected row.
pub(in crate::views) fn selection_marker(ui: &Ui) {
    let r = ui.max_rect();
    let mut clip = ui.clip_rect();
    clip.min.x -= 6.0;
    let p = egui::Painter::new(ui.ctx().clone(), ui.layer_id(), clip);
    p.rect_filled(Rect::from_min_size(pos2(r.left() - 4.0, r.top()), vec2(3.0, r.height())), 0.0, accent());
}

/// Colour of the overall progress: green when done, yellow past half, accent before.
fn progress_color(frac: f32) -> Color32 {
    if frac >= 1.0 {
        GREEN
    } else if frac >= 0.5 {
        YELLOW
    } else {
        accent()
    }
}

/// Draws the depot header, progress bar and resource table. `selected` highlights a resource
/// row; `scroll_to_selected` brings it into view (after a keyboard move).
pub(in crate::views) fn depot_ui(
    ui: &mut Ui,
    depot: &DepotSummary,
    ship_cargo: &HashMap<String, i32>,
    selected: Option<usize>,
    scroll_to_selected: bool,
) -> Option<ResourceAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&depot.station_name).font(theme::body_semi(18.0)).color(theme::accent_hi()));
        if depot.complete {
            widgets::badge(ui, "Complete", GREEN);
        } else if depot.failed {
            widgets::badge(ui, "Failed", theme::RED);
        }
    });
    ui.horizontal(|ui| {
        let col = progress_color(depot.progress);
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().min(360.0), 18.0), Sense::hover());
        widgets::seg_bar(ui.painter(), r.shrink2(vec2(0.0, 2.0)), depot.progress, col);
        ui.label(egui::RichText::new(format!("{:.1} %", depot.progress * 100.0)).font(theme::body_semi(17.0)).color(col));
    });
    if depot.system_name.is_some() || depot.updated.is_some() {
        egui::Grid::new("depot-meta").num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
            if let Some(sys) = &depot.system_name {
                widgets::kv(ui, "System", widgets::value(sys));
            }
            if let Some(ts) = &depot.updated {
                widgets::kv(ui, "Updated", egui::RichText::new(ts).color(GREY).font(theme::body(15.0)));
            }
        });
    }

    ui.add_space(6.0);
    widgets::section(ui, "Resources", Some(depot.resources.len()));
    if depot.resources.is_empty() {
        widgets::empty_state(ui, "This depot lists no resources.");
        return None;
    }

    let right = Layout::right_to_left(Align::Center);
    soften_table_highlights(ui);
    TableBuilder::new(ui)
        .id_salt("depot-resources")
        .striped(true)
        .cell_layout(Layout::left_to_right(Align::Center))
        .vscroll(false)
        .sense(Sense::click())
        .column(Column::remainder().at_least(130.0).clip(true))
        .column(Column::exact(104.0))
        .column(Column::exact(74.0))
        .column(Column::exact(64.0))
        .column(Column::exact(88.0))
        .column(Column::exact(52.0))
        .header(22.0, |mut h| {
            h.col(|ui| _ = widgets::th(ui, "Commodity"));
            h.col(|ui| _ = ui.with_layout(right, |ui| widgets::th(ui, "Delivered")));
            h.col(|_| {});
            h.col(|ui| _ = ui.with_layout(right, |ui| widgets::th(ui, "Left")));
            h.col(|ui| _ = ui.with_layout(right, |ui| widgets::th(ui, "Pays")));
            h.col(|ui| _ = ui.with_layout(right, |ui| widgets::th(ui, "Ship")).response.on_hover_text("Tonnes in your hold"));
        })
        .body(|mut body| {
            for (i, res) in depot.resources.iter().enumerate() {
                let frac = if res.required == 0 { 1.0 } else { res.provided as f32 / res.required as f32 };
                let remaining = (res.required - res.provided).max(0);
                let in_ship = ship_cargo.get(&normalize_commodity_name(&res.name)).copied().unwrap_or(0);
                let base = if frac >= 1.0 { GREEN } else if frac > 0.0 { YELLOW } else { GREY };
                let is_sel = selected == Some(i);
                body.row(24.0, |mut row| {
                    row.set_selected(is_sel);
                    row.col(|ui| {
                        if is_sel {
                            selection_marker(ui);
                        }
                        ui.label(egui::RichText::new(&res.display_name).font(if is_sel { theme::body_semi(15.5) } else { theme::body(15.5) }).color(TEXT));
                    });
                    row.col(|ui| {
                        ui.with_layout(right, |ui| {
                            ui.label(egui::RichText::new(format!("{} / {}", group_thousands(res.provided as i64), group_thousands(res.required as i64))).font(theme::body_semi(15.0)).color(base));
                        });
                    });
                    row.col(|ui| {
                        widgets::bar(ui, ui.available_width(), frac.clamp(0.0, 1.0), base);
                    });
                    row.col(|ui| {
                        ui.with_layout(right, |ui| {
                            let col = if remaining == 0 { GREEN } else { TEXT_DIM };
                            ui.label(egui::RichText::new(group_thousands(remaining as i64)).font(theme::body(15.0)).color(col));
                        });
                    });
                    row.col(|ui| {
                        ui.with_layout(right, |ui| {
                            if res.payment > 0 {
                                ui.label(egui::RichText::new(format!("{} cr/t", group_thousands(res.payment))).font(theme::body(14.5)).color(GREY));
                            }
                        });
                    });
                    row.col(|ui| {
                        ui.with_layout(right, |ui| {
                            if in_ship > 0 {
                                ui.label(egui::RichText::new(in_ship.to_string()).font(theme::body_semi(15.0)).color(theme::mix(YELLOW, theme::BG, 0.2)));
                            }
                        });
                    });
                    let resp = row.response();
                    if is_sel && scroll_to_selected {
                        resp.scroll_to_me(None);
                    }
                    if resp.double_clicked() {
                        action = Some(ResourceAction::Nearest(i));
                    } else if resp.clicked() || resp.secondary_clicked() {
                        action = Some(ResourceAction::Select(i));
                    }
                    resp.on_hover_text("Click to select · double-click: nearest station selling it (f)").context_menu(|ui| {
                        if ui.button(format!("Nearest station selling {}  (f)", res.display_name)).clicked() {
                            action = Some(ResourceAction::Nearest(i));
                            ui.close();
                        }
                    });
                });
            }
        });
    action
}
