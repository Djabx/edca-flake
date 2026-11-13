//! Desktop-window drawing for [`InventoryView`]: engineering materials against their storage
//! caps, the cargo hold, and the on-foot backpack and ship locker, one sub-tab each.

use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, Layout, Rect, Sense, Ui};
use egui_extras::{Column, TableBuilder};

use super::{material_cap, InventoryView, Tab};
use crate::gui::theme::{self, alpha, accent, CYAN, GREEN, GREY, MAGENTA, RED, TEXT, TEXT_DIM, YELLOW};
use crate::gui::widgets;
use crate::journal_reader::{InventoryItem, JournalData, OnFootInventory};
use crate::views::pilot::gui::key_scroll;
use crate::views::ViewEvent;

const TABS: [Tab; 4] = [Tab::Materials, Tab::Cargo, Tab::Backpack, Tab::ShipLocker];
/// Row height of every list here; also one step of the w/s keys.
const ROW_H: f32 = 24.0;
/// Ship-locker capacity per category.
const LOCKER_CAP: i32 = 1000;
/// Room kept free at the right of list rows for the scroll bar.
const SCROLLBAR_GAP: f32 = 8.0;

impl InventoryView {
    pub fn ui(&mut self, ui: &mut Ui, journal: &JournalData) -> ViewEvent {
        // Labels must not take clicks meant for the rows they sit on.
        ui.style_mut().interaction.selectable_labels = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for t in TABS {
                let resp = widgets::tab_button(ui, t.label(), t == self.tab, 24.0).on_hover_text("Previous / next: a / d");
                if resp.clicked() && t != self.tab {
                    self.tab = t;
                    self.scroll = 0;
                }
            }
            if matches!(self.tab, Tab::Materials) {
                ui.add_space(14.0);
                legend(ui);
            }
        });
        ui.add_space(8.0);

        match self.tab {
            Tab::Materials => {
                let cols = [
                    ("Raw", &journal.materials_raw),
                    ("Manufactured", &journal.materials_manufactured),
                    ("Encoded", &journal.materials_encoded),
                ];
                let max = columns(ui, cols.len(), |i, ui, scroll| {
                    let (label, items) = cols[i];
                    let total: i32 = items.iter().map(|it| it.count).sum();
                    let right = format!("{} types · {total}", items.len());
                    widgets::hud_panel(ui, label, &right, |ui| materials_list(ui, label, items, scroll))
                }, &mut self.scroll);
                self.scroll = self.scroll.min(max);
            }
            Tab::ShipLocker => {
                let max = self.onfoot_columns(ui, &journal.shiplocker, true);
                self.scroll = self.scroll.min(max);
            }
            Tab::Backpack => {
                let max = self.onfoot_columns(ui, &journal.backpack, false);
                self.scroll = self.scroll.min(max);
            }
            Tab::Cargo => self.cargo(ui, journal),
        }
        ViewEvent::None
    }

    /// Items / Components / Consumables / Data side by side; the ship locker also shows how
    /// full each category is.
    fn onfoot_columns(&mut self, ui: &mut Ui, inv: &OnFootInventory, capacity: bool) -> usize {
        let cols = [
            ("Items", &inv.items, GREEN),
            ("Components", &inv.components, YELLOW),
            ("Consumables", &inv.consumables, MAGENTA),
            ("Data", &inv.data, CYAN),
        ];
        columns(ui, cols.len(), |i, ui, scroll| {
            let (label, items, col) = cols[i];
            let used: i32 = items.iter().map(|it| it.count).sum();
            let right = if capacity { format!("{used} / {LOCKER_CAP}") } else { format!("{used}") };
            widgets::hud_panel(ui, label, &right, |ui| {
                if capacity {
                    let frac = (used as f32 / LOCKER_CAP as f32).min(1.0);
                    let bar_col = if frac > 0.9 { RED } else if frac > 0.7 { YELLOW } else { col };
                    widgets::bar(ui, ui.available_width(), frac, bar_col)
                        .on_hover_text(format!("{label}: {used} of {LOCKER_CAP} used"));
                    ui.add_space(4.0);
                }
                onfoot_list(ui, label, items, scroll)
            })
        }, &mut self.scroll)
    }

    fn cargo(&mut self, ui: &mut Ui, journal: &JournalData) {
        let total: i32 = journal.cargo.iter().map(|c| c.count).sum();
        let cap = journal.pilot.cargo_capacity;
        let right = if cap > 0 { format!("{total} / {cap} t") } else { format!("{total} t") };
        widgets::hud_panel(ui, "Cargo hold", &right, |ui| {
            if journal.cargo.is_empty() {
                widgets::empty_state(ui, "Cargo hold is empty.");
                return;
            }
            if cap > 0 {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Used").color(CYAN).font(theme::body(15.0)));
                    widgets::bar(ui, (ui.available_width() - 120.0).clamp(60.0, 360.0), total as f32 / cap as f32, accent());
                    ui.label(widgets::value(format!("{total} / {cap} t")));
                });
                ui.add_space(4.0);
            }
            let body_h = ui.available_height() - 30.0;
            let rows = journal.cargo.len();
            let mut lines = self.scroll;
            let max = key_scroll(ui, "inv-cargo", &mut lines, ROW_H, |ui, offset| {
                let mut t = TableBuilder::new(ui)
                    .id_salt("inv-cargo-table")
                    .striped(true)
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .column(Column::remainder().at_least(160.0).clip(true))
                    .column(Column::exact(90.0))
                    .column(Column::exact(90.0))
                    .max_scroll_height(body_h)
                    .auto_shrink([false, true]);
                if let Some(o) = offset {
                    t = t.vertical_scroll_offset(o);
                }
                t.header(22.0, |mut h| {
                    h.col(|ui| {
                        widgets::th(ui, "Item");
                    });
                    h.col(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| widgets::th(ui, "Count"));
                    });
                    h.col(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| widgets::th(ui, "Stolen"));
                    });
                })
                .body(|body| {
                    body.rows(ROW_H, rows, |mut row| {
                        let item = &journal.cargo[row.index()];
                        let name = if item.localised.is_empty() { &item.name } else { &item.localised };
                        row.col(|ui| {
                            ui.label(egui::RichText::new(name).font(theme::body(15.0)).color(TEXT));
                        });
                        row.col(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(widgets::value(item.count.to_string()));
                            });
                        });
                        row.col(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if item.stolen > 0 {
                                    ui.label(widgets::value(item.stolen.to_string()).color(RED));
                                } else {
                                    ui.label(egui::RichText::new("—").color(GREY));
                                }
                            });
                        });
                    });
                })
            });
            self.scroll = lines.min(max);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Total").color(CYAN).font(theme::body(15.0)));
                ui.label(widgets::value(format!("{total} t")));
            });
        });
    }
}

/// Lays out `n` equal panels side by side; each gets the shared w/s line offset. Returns the
/// largest line offset any of them allows.
fn columns(ui: &mut Ui, n: usize, mut add: impl FnMut(usize, &mut Ui, &mut usize) -> usize, scroll: &mut usize) -> usize {
    let full = ui.available_rect_before_wrap();
    let gap = 10.0;
    let w = (full.width() - gap * (n as f32 - 1.0)) / n as f32;
    let mut max = 0;
    for i in 0..n {
        let r = Rect::from_min_size(pos2(full.left() + i as f32 * (w + gap), full.top()), vec2(w, full.height()));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(Layout::top_down(Align::Min)));
        max = max.max(add(i, &mut child, scroll));
    }
    ui.allocate_rect(full, Sense::hover());
    max
}

/// The cap colour key shown beside the sub-tabs.
fn legend(ui: &mut Ui) {
    ui.label(egui::RichText::new("Bar = share of the storage cap:").color(TEXT_DIM).font(theme::body(14.0)));
    for (text, col) in [("under 25 %", RED), ("under 75 %", YELLOW), ("75 % and more", GREEN)] {
        let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
        ui.painter().rect_filled(r, 0.0, col);
        ui.label(egui::RichText::new(text).color(TEXT_DIM).font(theme::body(14.0)));
    }
}

/// Fill colour of a material bar by how full its storage is.
fn cap_color(ratio: f32) -> Color32 {
    if ratio < 0.25 { RED } else if ratio < 0.75 { YELLOW } else { GREEN }
}

/// Materials of one category, fullest first, each with count / cap and a bar.
fn materials_list(ui: &mut Ui, label: &str, items: &[InventoryItem], scroll: &mut usize) -> usize {
    if items.is_empty() {
        widgets::empty_state(ui, "Empty.");
        return 0;
    }
    let mut sorted: Vec<&InventoryItem> = items.iter().collect();
    sorted.sort_by_key(|it| std::cmp::Reverse(it.count));
    list_header(ui, "Material", "Count / cap");
    scrolled_rows(ui, label, sorted.len(), scroll, |ui, i| {
        let item = sorted[i];
        let name = if item.localised.is_empty() { &item.name } else { &item.localised };
        let max = material_cap(&item.name);
        let count = item.count.min(max);
        let ratio = count as f32 / max as f32;
        let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::hover());
        if i % 2 == 1 {
            ui.painter().rect_filled(row, 0.0, alpha(accent(), 10));
        }
        let r = row.with_max_x(row.right() - SCROLLBAR_GAP);
        let y = r.center().y;
        let bar_w = (r.width() * 0.24).clamp(40.0, 140.0);
        let bar = Rect::from_min_max(pos2(r.right() - bar_w - 4.0, y - 4.0), pos2(r.right() - 4.0, y + 4.0));
        widgets::seg_bar(ui.painter(), bar, ratio, cap_color(ratio));
        let count_x = bar.left() - 10.0;
        let p = ui.painter();
        let cr = widgets::txt(p, pos2(count_x, y), Align2::RIGHT_CENTER, &format!("/{max}"), theme::body(14.0), TEXT_DIM, 0.0);
        let nr = widgets::txt(p, pos2(cr.left() - 1.0, y), Align2::RIGHT_CENTER, &count.to_string(), theme::body_semi(15.0), TEXT, 0.0);
        let np = ui.painter_at(Rect::from_min_max(r.min, pos2(nr.left() - 8.0, r.bottom())));
        widgets::txt(&np, pos2(r.left() + 4.0, y), Align2::LEFT_CENTER, name, theme::body(15.0), TEXT, 0.0);
        resp.on_hover_text(format!("{name}: {count} of {max} ({:.0}%)", ratio * 100.0));
    })
}

/// On-foot items of one category with their counts.
fn onfoot_list(ui: &mut Ui, label: &str, items: &[InventoryItem], scroll: &mut usize) -> usize {
    if items.is_empty() {
        widgets::empty_state(ui, "Empty.");
        return 0;
    }
    list_header(ui, "Name", "Count");
    scrolled_rows(ui, label, items.len(), scroll, |ui, i| {
        let item = &items[i];
        let name = if item.localised.is_empty() { &item.name } else { &item.localised };
        let (row, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::hover());
        if i % 2 == 1 {
            ui.painter().rect_filled(row, 0.0, alpha(accent(), 10));
        }
        let r = row.with_max_x(row.right() - SCROLLBAR_GAP);
        let y = r.center().y;
        let cr = widgets::txt(ui.painter(), pos2(r.right() - 4.0, y), Align2::RIGHT_CENTER, &item.count.to_string(), theme::body_semi(15.0), TEXT, 0.0);
        let np = ui.painter_at(Rect::from_min_max(r.min, pos2(cr.left() - 8.0, r.bottom())));
        widgets::txt(&np, pos2(r.left() + 4.0, y), Align2::LEFT_CENTER, name, theme::body(15.0), TEXT, 0.0);
        resp.on_hover_text(format!("{name}: {}", item.count));
    })
}

/// Column captions above a list.
fn list_header(ui: &mut Ui, left: &str, right: &str) {
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        widgets::th(ui, left);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(4.0 + SCROLLBAR_GAP);
            widgets::th(ui, right);
        });
    });
}

/// `n` rows of `ROW_H` in a scroll area that follows the shared w/s offset.
fn scrolled_rows(ui: &mut Ui, salt: &str, n: usize, scroll: &mut usize, mut row: impl FnMut(&mut Ui, usize)) -> usize {
    key_scroll(ui, salt, scroll, ROW_H, |ui, offset| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut area = egui::ScrollArea::vertical().id_salt(("inv", salt)).auto_shrink([false, false]);
        if let Some(o) = offset {
            area = area.vertical_scroll_offset(o);
        }
        area.show_rows(ui, ROW_H, n, |ui, range| {
            for i in range {
                row(ui, i);
            }
        })
    })
}
