//! Desktop-window drawing for [`ModulesView`]: the fitted modules as a selectable table, and
//! the ship plus the selected module's engineering in a detail panel beside it.

use eframe::egui::{self, pos2, vec2, Align, Color32, Layout, Rect, Sense, Ui};
use egui_extras::{Column, TableBuilder};

use super::{condition_color, fmt_credits, ModulesView};
use crate::gui::theme::{self, accent, CYAN, GREEN, GREY, RED, TEXT, TEXT_DIM};
use crate::gui::widgets;
use crate::journal_reader::{EngineeringData, JournalData, ShipModule};
use crate::views::pilot::gui::{grade_pips, kv_grid, PURPLE};
use crate::views::ViewEvent;

/// Experimental-effect amber.
const EXPERIMENTAL: Color32 = Color32::from_rgb(255, 200, 80);

impl ModulesView {
    pub fn ui(&mut self, ui: &mut Ui, journal: &JournalData) -> ViewEvent {
        // Labels must not take clicks meant for the rows they sit on.
        ui.style_mut().interaction.selectable_labels = false;
        if let Some(last) = journal.modules.len().checked_sub(1) {
            self.selected = self.selected.min(last);
        }
        let full = ui.available_rect_before_wrap();
        let list_w = (full.width() * 0.62).floor();
        let list = Rect::from_min_size(full.min, vec2(list_w, full.height()));
        let detail = Rect::from_min_max(pos2(list.right() + 10.0, full.top()), full.max);

        let count = journal.modules.len();
        widgets::hud_panel_at(ui, list, "Modules", &format!("{count} fitted · w/s select"), |ui| {
            if journal.modules.is_empty() {
                widgets::empty_state(ui, "No module data — set the journal folder in Settings; the next Loadout event fills this.");
            } else {
                self.table(ui, journal);
            }
        });
        widgets::hud_panel_at(ui, detail, "Detail", &journal.pilot.ship_type, |ui| {
            egui::ScrollArea::vertical().id_salt("modules-detail").auto_shrink([false, false]).show(ui, |ui| {
                self.detail(ui, journal);
            });
        });
        ViewEvent::None
    }

    fn table(&mut self, ui: &mut Ui, journal: &JournalData) {
        let id = ui.id().with("modules-prev-selected");
        let prev: Option<usize> = ui.data(|d| d.get_temp(id));
        let follow = prev != Some(self.selected);

        let mut clicked = None;
        let slot_w = ((ui.available_width() - 290.0) * 0.4).clamp(100.0, 220.0);
        let mut table = TableBuilder::new(ui)
            .id_salt("modules-table")
            .striped(true)
            .sense(Sense::click())
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(slot_w).clip(true))
            .column(Column::remainder().at_least(100.0).clip(true))
            .column(Column::exact(80.0))
            .column(Column::exact(116.0))
            .column(Column::exact(72.0))
            .auto_shrink([false, false]);
        if follow {
            table = table.scroll_to_row(self.selected, None);
        }
        table
            .header(22.0, |mut h| {
                h.col(|ui| {
                    widgets::th(ui, "Slot");
                });
                h.col(|ui| {
                    widgets::th(ui, "Module");
                });
                h.col(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| widgets::th(ui, "Power"));
                });
                h.col(|ui| {
                    widgets::th(ui, "Condition")
                        .on_hover_text("Green above 75 %, yellow above 50 %, orange above 25 %, red below");
                });
                h.col(|ui| {
                    widgets::th(ui, "Eng.");
                });
            })
            .body(|body| {
                body.rows(26.0, journal.modules.len(), |mut row| {
                    let i = row.index();
                    let m = &journal.modules[i];
                    let sel = i == self.selected;
                    row.set_selected(sel);
                    let font = if sel { theme::body_semi(15.0) } else { theme::body(15.0) };
                    row.col(|ui| {
                        ui.label(egui::RichText::new(&m.slot).font(font.clone()).color(if sel { TEXT } else { TEXT_DIM }));
                    });
                    row.col(|ui| {
                        ui.label(egui::RichText::new(&m.item).font(font.clone()).color(TEXT));
                    });
                    row.col(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let (t, c) = if m.power > 0.0 { (format!("{:.2} MW", m.power), TEXT) } else { ("—".into(), GREY) };
                            ui.label(egui::RichText::new(t).font(font.clone()).color(c));
                        });
                    });
                    row.col(|ui| condition_cell(ui, m, sel));
                    row.col(|ui| {
                        if let Some(e) = &m.engineering {
                            let col = if sel { TEXT } else { PURPLE };
                            ui.label(egui::RichText::new(format!("G{}", e.level)).font(theme::body_semi(15.0)).color(col));
                            ui.add_space(-4.0);
                            mini_pips(ui, e.level, col);
                        }
                    });
                    if row.response().clicked() {
                        clicked = Some(i);
                    }
                });
            });
        if let Some(i) = clicked {
            self.selected = i;
        }
        ui.data_mut(|d| d.insert_temp(id, self.selected));
    }

    fn detail(&self, ui: &mut Ui, journal: &JournalData) {
        let p = &journal.pilot;
        if !p.ship_type.is_empty() {
            widgets::section(ui, "Ship", None);
            kv_grid(ui, "modules-ship", |ui| {
                widgets::kv(ui, "Type", widgets::value(&p.ship_type));
                if !p.ship_name.is_empty() {
                    widgets::kv(ui, "Name", widgets::value(&p.ship_name).font(theme::body_semi(16.0)));
                }
                if !p.ship_ident.is_empty() {
                    widgets::kv(ui, "Ident", widgets::value(&p.ship_ident));
                }
                if p.hull_health > 0.0 {
                    let pct = (p.hull_health * 100.0) as u8;
                    let col = if pct > 75 { GREEN } else if pct > 50 { theme::YELLOW } else if pct > 25 { accent() } else { RED };
                    label(ui, "Hull");
                    bar_value(ui, p.hull_health, col, &format!("{pct}%"));
                    ui.end_row();
                }
                if p.max_jump_range > 0.0 {
                    widgets::kv(ui, "Max jump", widgets::value(format!("{:.2} ly", p.max_jump_range)).color(GREEN));
                }
                if p.unladen_mass > 0.0 {
                    widgets::kv(ui, "Unladen", widgets::value(format!("{:.1} t", p.unladen_mass)));
                }
                if p.cargo_capacity > 0 {
                    widgets::kv(ui, "Cargo", widgets::value(format!("{} t", p.cargo_capacity)));
                }
                if p.fuel_capacity > 0.0 {
                    let frac = p.fuel_level / p.fuel_capacity;
                    let pct = (frac * 100.0) as u8;
                    let col = if pct > 25 { GREEN } else { RED };
                    label(ui, "Fuel");
                    ui.vertical(|ui| {
                        bar_value(ui, frac, col, &format!("{pct}%"));
                        let mut t = format!("{:.1} / {:.1} t", p.fuel_level, p.fuel_capacity);
                        if p.reserve_fuel_capacity > 0.0 {
                            t.push_str(&format!("  + {:.2} t reserve", p.reserve_fuel_capacity));
                        }
                        ui.label(egui::RichText::new(t).color(TEXT_DIM).font(theme::body(14.0)));
                    });
                    ui.end_row();
                }
                if p.modules_value > 0 {
                    widgets::kv(ui, "Modules value", widgets::value(format!("{} Cr", fmt_credits(p.modules_value))));
                }
                if p.rebuy > 0 {
                    widgets::kv(ui, "Rebuy", widgets::value(format!("{} Cr", fmt_credits(p.rebuy))).color(RED));
                }
            });
            ui.add_space(10.0);
        }

        let Some(m) = journal.modules.get(self.selected) else {
            widgets::empty_state(ui, "No module selected.");
            return;
        };
        widgets::section(ui, "Module", None);
        kv_grid(ui, "modules-module", |ui| {
            widgets::kv(ui, "Slot", widgets::value(&m.slot));
            label(ui, "Item");
            ui.add(egui::Label::new(widgets::value(&m.item)).wrap());
            ui.end_row();
            let pwr = if m.power > 0.0 { format!("{:.2} MW", m.power) } else { "—".into() };
            widgets::kv(ui, "Power draw", widgets::value(pwr).color(accent()));
            let prio = if m.power > 0.0 { m.priority.to_string() } else { "—".into() };
            widgets::kv(ui, "Priority", widgets::value(prio));
            label(ui, "Condition");
            match m.health {
                Some(h) => bar_value(ui, h, theme::from_ratatui(condition_color(m.health)), &format!("{:.1}%", h * 100.0)),
                None => {
                    ui.label(widgets::value("—").color(GREY));
                }
            }
            ui.end_row();
            if let Some(v) = m.value {
                widgets::kv(ui, "Value", widgets::value(format!("{} Cr", fmt_credits(v))));
            }
        });

        if let Some(eng) = &m.engineering {
            ui.add_space(10.0);
            engineering(ui, eng);
        }
    }
}

/// The blueprint, engineer, experimental effect and each modified stat against its original.
fn engineering(ui: &mut Ui, eng: &EngineeringData) {
    widgets::section(ui, "Engineering", None);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{}  G{}", eng.blueprint, eng.level)).font(theme::body_semi(16.0)).color(PURPLE));
        grade_pips(ui, eng.level, PURPLE);
    });
    kv_grid(ui, "modules-eng", |ui| {
        widgets::kv(ui, "Engineer", widgets::value(&eng.engineer));
        if !eng.experimental.is_empty() {
            widgets::kv(ui, "Experimental", widgets::value(&eng.experimental).color(EXPERIMENTAL));
        }
    });
    if eng.modifiers.is_empty() {
        return;
    }
    ui.add_space(6.0);
    egui::Grid::new("modules-modifiers").num_columns(3).striped(true).spacing([12.0, 5.0]).show(ui, |ui| {
        widgets::th(ui, "Stat");
        widgets::th(ui, "Original → now");
        widgets::th(ui, "Change");
        ui.end_row();
        for m in &eng.modifiers {
            let improved = if m.less_is_good { m.value < m.original_value } else { m.value > m.original_value };
            let col = if improved { GREEN } else { RED };
            let arrow = if m.value > m.original_value { "▲" } else { "▼" };
            let pct = if m.original_value != 0.0 {
                ((m.value - m.original_value) / m.original_value.abs() * 100.0).round() as i32
            } else {
                0
            };
            ui.label(egui::RichText::new(&m.label).color(TEXT_DIM).font(theme::body(15.0)));
            ui.label(egui::RichText::new(format!("{:.2} → {:.2}", m.original_value, m.value)).color(col).font(theme::body_semi(15.0)));
            ui.label(egui::RichText::new(format!("{arrow} {pct:+}%")).color(col).font(theme::body_semi(15.0)));
            ui.end_row();
        }
    });
}

/// A table cell with a short condition bar and the percentage, coloured by wear.
fn condition_cell(ui: &mut Ui, m: &ShipModule, selected: bool) {
    match m.health {
        Some(h) => {
            let col = theme::from_ratatui(condition_color(m.health));
            widgets::bar(ui, 52.0, h, col);
            let text_col = if selected { TEXT } else { col };
            ui.label(egui::RichText::new(format!("{:.1}%", h * 100.0)).font(theme::body(15.0)).color(text_col));
        }
        None => {
            ui.label(egui::RichText::new("—").color(GREY));
        }
    }
}

/// Compact grade pips for a table cell.
fn mini_pips(ui: &mut Ui, level: u8, col: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(5.0 * 6.0, 10.0), Sense::hover());
    for i in 0..5u8 {
        let pr = Rect::from_min_size(pos2(r.left() + i as f32 * 6.0, r.center().y - 3.0), vec2(4.0, 6.0));
        ui.painter().rect_filled(pr, 0.0, if i < level { col } else { theme::alpha(col, 45) });
    }
}

fn label(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(CYAN).font(theme::body(15.0)));
}

/// A bar with its reading beside it, for grid value cells.
fn bar_value(ui: &mut Ui, frac: f32, col: Color32, reading: &str) {
    ui.horizontal(|ui| {
        widgets::bar(ui, 120.0, frac, col);
        ui.label(egui::RichText::new(reading).font(theme::body_semi(15.0)).color(col));
    });
}
