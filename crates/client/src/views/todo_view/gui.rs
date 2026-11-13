//! Desktop-window drawing for [`TodoView`]: the todo list (engineering mods and construction
//! sites by system) beside the selected item's materials or commodities, or the summed
//! materials of every engineering todo.
//!
//! Clicks set the same fields the keys move (`selected_idx`, `resource_selected_idx`,
//! `focus`, `show_aggregate`); remove and search-nearest run through `handle_key` so they do
//! exactly what `r` and `f` do.

use std::collections::HashMap;

use eframe::egui::{self, vec2, Align, Color32, Id, Layout, RichText, Sense, Stroke, Ui};
use egui_extras::{Column, Size, StripBuilder, TableBuilder};

use super::{SelectedKind, TodoFocus, TodoView};
use crate::event_shim::{KeyCode, KeyEvent};
use crate::gui::theme::{self, accent, accent_hi, alpha, mix, BG, CYAN, GREEN, GREY, RED, TEXT, TEXT_DIM, YELLOW};
use crate::gui::widgets;
use crate::journal_reader::JournalData;
use crate::todo::ModKind;
use crate::views::util::{have_count, normalize_commodity_name};
use crate::views::ViewEvent;

const ROW_H: f32 = 30.0;

/// Whether `value` changed since the last frame, per `key` (to scroll a keyboard move into view).
fn moved(ui: &Ui, key: &str, value: usize) -> bool {
    let id = Id::new(("todo-moved", key));
    let before = ui.ctx().data_mut(|d| {
        let before = d.get_temp::<usize>(id);
        d.insert_temp(id, value);
        before
    });
    before != Some(value)
}

fn kind_tag(kind: &ModKind) -> &'static str {
    match kind {
        ModKind::Ship => "Ship",
        ModKind::OnFoot => "On foot",
    }
}

impl TodoView {
    pub fn ui(&mut self, ui: &mut Ui, journal: &JournalData) -> ViewEvent {
        let mut ev = ViewEvent::None;
        let left_w = (ui.available_width() * 0.36).clamp(320.0, 460.0);
        StripBuilder::new(ui)
            .size(Size::exact(left_w))
            .size(Size::exact(10.0))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    let caption = format!("{} items", self.total_count());
                    widgets::hud_panel(ui, "Todo", &caption, |ui| self.list_ui(ui, journal, &mut ev));
                });
                strip.empty();
                strip.cell(|ui| {
                    let (title, caption) = if self.show_aggregate {
                        ("All materials", "g: item view")
                    } else if self.focus == TodoFocus::Right {
                        ("Materials", "w/s: commodity · tab: list")
                    } else {
                        ("Materials", "tab: commodities · g: all")
                    };
                    widgets::hud_panel(ui, title, caption, |ui| self.detail_ui(ui, journal, &mut ev));
                });
            });
        ev
    }

    // ------------------------------------------------------------ left: the list

    fn list_ui(&mut self, ui: &mut Ui, journal: &JournalData, ev: &mut ViewEvent) {
        if self.total_count() == 0 {
            widgets::empty_state(ui, "No items yet.");
            ui.vertical_centered(|ui| {
                if journal.current_system.is_none() {
                    widgets::hint(ui, "Make sure the journal folder is set in Settings,");
                    widgets::hint(ui, "then add mods in the Workshop tab or dock at a construction site.");
                } else {
                    widgets::hint(ui, "Add mods in the Workshop tab or dock at a construction site.");
                }
            });
            return;
        }

        let footer_h = 34.0;
        let list_h = (ui.available_height() - footer_h).max(60.0);
        let scroll_sel = moved(ui, "item", self.selected_idx);
        ui.allocate_ui(vec2(ui.available_width(), list_h), |ui| {
            egui::ScrollArea::vertical().id_salt("todo-list").auto_shrink([false, false]).show(ui, |ui| {
                if !self.todo.items.is_empty() {
                    widgets::section(ui, "Engineering", Some(self.todo.items.len()));
                    for i in 0..self.todo.items.len() {
                        self.engineering_row(ui, i, scroll_sel, journal, ev);
                    }
                    ui.add_space(8.0);
                }
                if !self.todo.construction_items.is_empty() {
                    widgets::section(ui, "Construction", Some(self.todo.construction_items.len()));
                    let offset = self.todo.items.len();
                    let groups: Vec<(String, Vec<usize>)> = self
                        .construction_groups()
                        .into_iter()
                        .map(|(sys, items)| (sys, items.into_iter().map(|(i, _)| i).collect()))
                        .collect();
                    for (sys, items) in groups {
                        ui.add_space(2.0);
                        ui.label(RichText::new(sys).color(CYAN).font(theme::body_semi(15.0)));
                        for ci in items {
                            self.construction_row(ui, offset + ci, ci, scroll_sel, journal, ev);
                        }
                        ui.add_space(4.0);
                    }
                }
            });
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let label = match self.selected_kind() {
                Some(SelectedKind::Engineering(i)) if self.todo.items[i].count > 1 => "Remove one",
                _ => "Remove",
            };
            let can = self.selected_kind().is_some();
            let tip = "Remove the selected item; a mod queued several times loses one (r / Del)";
            if widgets::hud_button_enabled(ui, label, can).on_hover_text(tip).clicked() {
                keep(ev, self.press_left(KeyCode::Char('r'), journal));
            }
        });
    }

    /// Runs a list-panel key (remove, all materials) the way it works with the list focused.
    fn press_left(&mut self, code: KeyCode, journal: &JournalData) -> ViewEvent {
        self.focus = TodoFocus::Left;
        self.handle_key(&KeyEvent::from(code), journal)
    }

    /// Selects list entry `idx` as the arrow keys do.
    fn select_item(&mut self, idx: usize) {
        if idx != self.selected_idx {
            self.selected_idx = idx;
            self.resource_selected_idx = 0;
        }
        self.focus = TodoFocus::Left;
    }

    /// Click and right-click handling shared by both kinds of list row.
    fn list_row_input(&mut self, resp: egui::Response, idx: usize, scroll_sel: bool, journal: &JournalData, ev: &mut ViewEvent) {
        if idx == self.selected_idx && scroll_sel {
            resp.scroll_to_me(None);
        }
        if resp.clicked() {
            self.select_item(idx);
        }
        resp.context_menu(|ui| {
            if ui.button("Remove (r)").clicked() {
                self.select_item(idx);
                keep(ev, self.press_left(KeyCode::Char('r'), journal));
                ui.close();
            }
        });
    }

    fn engineering_row(&mut self, ui: &mut Ui, i: usize, scroll_sel: bool, journal: &JournalData, ev: &mut ViewEvent) {
        let item = &self.todo.items[i];
        let current = i == self.selected_idx;
        let filled = current && self.focus == TodoFocus::Left;
        let (name, grade, count, kind) = (item.mod_name.clone(), item.grade, item.count, item.kind.clone());
        let resp = widgets::selectable_row(ui, filled, ROW_H, |ui| {
            let fg = widgets::row_text_color(filled, TEXT);
            ui.add(egui::Label::new(RichText::new(&name).color(fg).font(theme::body_semi(16.0))).selectable(false).truncate());
            ui.add(egui::Label::new(RichText::new(format!("G{grade}")).color(widgets::row_text_color(filled, accent_hi())).font(theme::head(10.0))).selectable(false));
            if count > 1 {
                ui.add(egui::Label::new(RichText::new(format!("×{count}")).color(widgets::row_text_color(filled, YELLOW)).font(theme::body_semi(15.0))).selectable(false));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(egui::Label::new(widgets::job(&kind_tag(&kind).to_uppercase(), theme::head(8.5), widgets::row_text_color(filled, TEXT_DIM), 1.2)).selectable(false));
            });
        });
        outline_if(ui, current && !filled, resp.rect);
        let resp = resp.on_hover_text("Click to show its materials · right-click to remove");
        self.list_row_input(resp, i, scroll_sel, journal, ev);
    }

    fn construction_row(&mut self, ui: &mut Ui, idx: usize, ci: usize, scroll_sel: bool, journal: &JournalData, ev: &mut ViewEvent) {
        let site = &self.todo.construction_items[ci];
        let current = idx == self.selected_idx;
        let filled = current && self.focus == TodoFocus::Left;
        let station = site.station_name.clone();
        let remaining = site.resources.iter().filter(|r| r.provided_amount < r.required_amount).count();
        let resp = widgets::selectable_row(ui, filled, ROW_H, |ui| {
            ui.add_space(10.0);
            let fg = widgets::row_text_color(filled, TEXT);
            ui.add(egui::Label::new(RichText::new("★").color(widgets::row_text_color(filled, accent()))).selectable(false));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (txt, col) = if remaining == 0 { ("done".to_string(), GREEN) } else { (format!("{remaining} needed"), YELLOW) };
                ui.add(egui::Label::new(RichText::new(txt).color(widgets::row_text_color(filled, col)).font(theme::body(14.5))).selectable(false));
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add(egui::Label::new(RichText::new(&station).color(fg).font(theme::body_semi(16.0))).selectable(false).truncate());
                });
            });
        });
        outline_if(ui, current && !filled, resp.rect);
        let resp = resp.on_hover_text(format!("{station}\nClick to show its commodities · right-click to remove"));
        self.list_row_input(resp, idx, scroll_sel, journal, ev);
    }

    // ------------------------------------------------------------ right: the details

    fn detail_ui(&mut self, ui: &mut Ui, journal: &JournalData, ev: &mut ViewEvent) {
        let current = usize::from(self.show_aggregate);
        let strip = ui.scope(|ui| widgets::tab_strip(ui, &["Selected item", "All materials"], current));
        strip.response.on_hover_text("The selected item, or the summed materials of every engineering todo (g)");
        if let Some(i) = strip.inner {
            if i != current {
                self.show_aggregate = i == 1;
            }
        }
        ui.add_space(8.0);
        if self.show_aggregate {
            self.aggregate_ui(ui, journal);
            return;
        }
        match self.selected_kind() {
            None => widgets::empty_state(ui, "Select an item to see details."),
            Some(SelectedKind::Engineering(idx)) => self.engineering_detail(ui, idx, journal),
            Some(SelectedKind::Construction(idx)) => self.construction_detail(ui, idx, journal, ev),
        }
    }

    fn engineering_detail(&self, ui: &mut Ui, idx: usize, journal: &JournalData) {
        let item = &self.todo.items[idx];
        ui.horizontal(|ui| {
            let count = if item.count > 1 { format!("  ×{}", item.count) } else { String::new() };
            ui.label(RichText::new(format!("{}  G{}{count}", item.mod_name, item.grade)).color(YELLOW).font(theme::body_semi(21.0)));
            widgets::badge(ui, kind_tag(&item.kind), CYAN);
        });
        ui.label(RichText::new(format!("Module: {}", item.module_type)).color(TEXT_DIM).font(theme::body(15.0)));
        ui.add_space(8.0);

        let costs = Self::grade_costs(&item.mod_id, item.grade, &item.kind);
        widgets::section(ui, "Materials required", Some(costs.len()));
        if costs.is_empty() {
            widgets::empty_state(ui, "No material data found.");
            return;
        }
        let rows: Vec<(String, i32, i32)> = costs
            .iter()
            .map(|c| (c.name.clone(), have_count(&c.name, &item.kind, journal), c.count as i32 * item.count as i32))
            .collect();
        let missing = rows.iter().filter(|(_, have, need)| have < need).count();
        materials_table(ui, "todo-eng-materials", &rows, footer_height(), |ui| {
            if missing == 0 {
                ui.label(RichText::new("✓ Ready to engineer!").color(GREEN).font(theme::body_semi(16.0)));
            } else {
                ui.label(RichText::new(format!("{missing} material(s) still needed.")).color(YELLOW).font(theme::body_semi(16.0)));
            }
        });
    }

    fn aggregate_ui(&self, ui: &mut Ui, journal: &JournalData) {
        if self.todo.items.is_empty() {
            widgets::empty_state(ui, "No engineering todos.");
            return;
        }
        ui.label(RichText::new(format!("All materials for {} mod(s)", self.todo.items.len())).color(YELLOW).font(theme::body_semi(21.0)));
        ui.add_space(8.0);
        let rows: Vec<(String, i32, i32)> = self
            .aggregate_materials(journal)
            .into_iter()
            .map(|(name, need, kind)| {
                let have = have_count(&name, &kind, journal);
                (name, have, need)
            })
            .collect();
        widgets::section(ui, "Materials", Some(rows.len()));
        let missing = rows.iter().filter(|(_, have, need)| have < need).count();
        materials_table(ui, "todo-all-materials", &rows, footer_height(), |ui| {
            if missing == 0 {
                ui.label(RichText::new("✓ All materials ready!").color(GREEN).font(theme::body_semi(16.0)));
            } else {
                ui.label(RichText::new(format!("{missing} material(s) still needed.")).color(YELLOW).font(theme::body_semi(16.0)));
            }
        });
    }

    fn construction_detail(&mut self, ui: &mut Ui, idx: usize, journal: &JournalData, ev: &mut ViewEvent) {
        let site = &self.todo.construction_items[idx];
        let live = journal.construction_depots.contains_key(&site.market_id);
        let station = site.station_name.clone();
        let system = site.system_name.clone();
        let resources = Self::site_resources(site, journal);
        let ship_cargo: HashMap<String, i32> = journal.cargo.iter().map(|c| (normalize_commodity_name(&c.name), c.count)).collect();

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("★ {station}")).color(accent_hi()).font(theme::body_semi(21.0)));
            if live {
                widgets::badge(ui, "Live data", GREEN).on_hover_text("Docked here: amounts come straight from the journal");
            }
        });
        ui.horizontal(|ui| {
            if !system.is_empty() {
                ui.label(RichText::new(&system).color(TEXT_DIM).font(theme::body(15.0)));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let name = resources.get(self.resource_selected_idx).map(|r| r.1.clone()).unwrap_or_default();
                let tip = format!("Find the nearest stations selling {name} (f)");
                if widgets::hud_button_enabled(ui, "Search nearest", !resources.is_empty()).on_hover_text(tip).clicked() {
                    keep(ev, self.search_nearest(self.resource_selected_idx, journal));
                }
            });
        });
        ui.add_space(6.0);

        let remaining = resources.iter().filter(|r| r.2 < r.3).count();
        widgets::section(ui, "Commodities needed", Some(remaining));
        let scroll = moved(ui, "resource", self.resource_selected_idx + 1000 * idx);
        let table_h = (ui.available_height() - footer_height() - 24.0).max(60.0);
        let mut clicked = None;
        let mut search = None;
        ui.scope(|ui| {
            // The keyboard cursor's row is the table's selected row; keep its fill a tint.
            ui.visuals_mut().selection.bg_fill = alpha(accent(), 44);
            let mut table = TableBuilder::new(ui)
                .id_salt(("todo-commodities", idx))
                .striped(true)
                .sense(Sense::click())
                .auto_shrink([false, true])
                .max_scroll_height(table_h)
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::exact(18.0))
                .column(Column::remainder().at_least(120.0).clip(true))
                .column(Column::exact(72.0))
                .column(Column::exact(96.0))
                .column(Column::exact(78.0))
                .column(Column::exact(58.0));
            if scroll && self.focus == TodoFocus::Right {
                table = table.scroll_to_row(self.resource_selected_idx, None);
            }
            table
                .header(22.0, |mut h| {
                    for t in ["", "Commodity", "Left", "Delivered", "Pays", "Ship"] {
                        h.col(|ui| {
                            widgets::th(ui, t);
                        });
                    }
                })
                .body(|body| {
                    body.rows(26.0, resources.len(), |mut row| {
                        let i = row.index();
                        let (raw, name, provided, required, payment) = &resources[i];
                        let done = provided >= required;
                        let left = (required - provided).max(0);
                        let frac = if *required == 0 { 1.0 } else { *provided as f32 / *required as f32 };
                        let col = if done { GREEN } else if frac > 0.0 { YELLOW } else { GREY };
                        let in_ship = ship_cargo.get(&normalize_commodity_name(raw)).copied().unwrap_or(0);
                        row.set_selected(self.focus == TodoFocus::Right && i == self.resource_selected_idx);
                        row.col(|ui| {
                            ui.label(RichText::new(if done { "✓" } else { "·" }).color(col).font(theme::body_semi(16.0)));
                        });
                        row.col(|ui| {
                            ui.add(egui::Label::new(RichText::new(name).color(if done { GREEN } else { TEXT }).font(theme::body(15.5))).selectable(false).truncate());
                        });
                        row.col(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(group(left as i64)).color(if done { GREEN } else { TEXT }).font(theme::body_semi(15.5)));
                            });
                        });
                        row.col(|ui| {
                            widgets::bar(ui, 88.0, frac.min(1.0), col).on_hover_text(format!("{} of {} delivered", group(*provided as i64), group(*required as i64)));
                        });
                        row.col(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if *payment > 0 {
                                    ui.label(RichText::new(format!("{} cr/t", group(*payment))).color(TEXT_DIM).font(theme::body(14.5)));
                                }
                            });
                        });
                        row.col(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if in_ship > 0 {
                                    ui.label(RichText::new(group(in_ship as i64)).color(mix(YELLOW, BG, 0.25)).font(theme::body_semi(15.0)))
                                        .on_hover_text("Tonnes of it in your ship's hold");
                                }
                            });
                        });
                        let resp = row.response();
                        if resp.clicked() {
                            clicked = Some(i);
                        }
                        if resp.double_clicked() {
                            search = Some(i);
                        }
                        resp.on_hover_text("Click to select · double-click to search nearest (f)");
                    });
                });
        });
        if let Some(i) = clicked {
            self.focus = TodoFocus::Right;
            self.resource_selected_idx = i;
        }
        if let Some(i) = search {
            keep(ev, self.search_nearest(i, journal));
        }
        if remaining == 0 && !resources.is_empty() {
            ui.label(RichText::new("✓ All commodities delivered!").color(GREEN).font(theme::body_semi(16.0)));
        }
    }

    /// Searches the nearest seller of resource `i`, exactly as `f` does on that commodity.
    fn search_nearest(&mut self, i: usize, journal: &JournalData) -> ViewEvent {
        self.focus = TodoFocus::Right;
        self.resource_selected_idx = i;
        self.handle_key(&KeyEvent::from(KeyCode::Char('f')), journal)
    }
}

// ---------------------------------------------------------------- free helpers

/// Keeps the event that asks for something over `None`/`Consumed`.
fn keep(ev: &mut ViewEvent, new: ViewEvent) {
    if !matches!(new, ViewEvent::None | ViewEvent::Consumed) || matches!(ev, ViewEvent::None) {
        *ev = new;
    }
}

fn footer_height() -> f32 {
    30.0
}

fn group(n: i64) -> String {
    edcas_common::format::group_thousands(n)
}

/// Outlines the current list row while the details panel has the keyboard.
fn outline_if(ui: &Ui, on: bool, rect: egui::Rect) {
    if on {
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, accent()), egui::StrokeKind::Inside);
        ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, vec2(3.0, rect.height())), 0.0, accent());
    }
}

/// Materials as (name, have, need): ✓/✗, name, have/need and a bar, green when enough and
/// red when short; `footer` draws the summary under the table.
fn materials_table(ui: &mut Ui, id: &str, rows: &[(String, i32, i32)], footer_h: f32, footer: impl FnOnce(&mut Ui)) {
    let table_h = (ui.available_height() - footer_h - 24.0).max(60.0);
    ui.scope(|ui| {
        TableBuilder::new(ui)
            .id_salt(id)
            .striped(true)
            .auto_shrink([false, true])
            .max_scroll_height(table_h)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(18.0))
            .column(Column::remainder().at_least(140.0).clip(true))
            .column(Column::exact(80.0))
            .column(Column::exact(130.0))
            .header(22.0, |mut h| {
                for t in ["", "Material", "Have", ""] {
                    h.col(|ui| {
                        widgets::th(ui, t);
                    });
                }
            })
            .body(|body| {
                body.rows(26.0, rows.len(), |mut row| {
                    let (name, have, need) = &rows[row.index()];
                    let ok = have >= need;
                    let col: Color32 = if ok { GREEN } else { RED };
                    let frac = if *need <= 0 { 1.0 } else { (*have as f32 / *need as f32).min(1.0) };
                    row.col(|ui| {
                        ui.label(RichText::new(if ok { "✓" } else { "✗" }).color(col).font(theme::body_semi(16.0)));
                    });
                    row.col(|ui| {
                        ui.add(egui::Label::new(RichText::new(name).color(col).font(theme::body(15.5))).truncate());
                    });
                    row.col(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(format!("{} / {}", (*have).min(*need), need)).color(TEXT).font(theme::body_semi(15.5)));
                        });
                    });
                    row.col(|ui| {
                        widgets::bar(ui, 120.0, frac, col).on_hover_text(format!("You have {have}"));
                    });
                });
            });
    });
    ui.add_space(4.0);
    footer(ui);
}
