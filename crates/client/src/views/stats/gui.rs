//! Desktop-window drawing for [`StatsView`]: session tallies and unsold data value.

use eframe::egui::{self, Ui};

use super::{format_credits, group_thousands, StatsView};
use crate::gui::{theme, widgets};
use crate::journal_reader::JournalData;

impl StatsView {
    pub fn ui(&mut self, ui: &mut Ui, journal: &JournalData) {
        let s = &journal.session;
        widgets::hud_panel(ui, "Session statistics", "since client start", |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_max_width(560.0);
                widgets::section(ui, "Session", None);
                egui::Grid::new("stats-session").num_columns(2).min_col_width(180.0).spacing([16.0, 6.0]).show(ui, |ui| {
                    widgets::kv(ui, "Jumps", widgets::value(group_thousands(s.jumps as u64)));
                    widgets::kv(ui, "Distance", widgets::value(format!("{:.1} ly", s.light_years)));
                    widgets::kv(ui, "Systems visited", widgets::value(group_thousands(s.systems_visited as u64)));
                    widgets::kv(ui, "Bodies scanned", widgets::value(group_thousands(s.bodies_scanned as u64)));
                    widgets::kv(ui, "Bodies mapped", widgets::value(group_thousands(s.bodies_mapped as u64)));
                    widgets::kv(ui, "Dockings", widgets::value(group_thousands(s.dockings as u64)));
                });

                ui.add_space(10.0);
                widgets::section(ui, "Exploration data", None);
                egui::Grid::new("stats-explo").num_columns(2).min_col_width(180.0).spacing([16.0, 6.0]).show(ui, |ui| {
                    let unsold = if self.unsold.loaded { format_credits(self.unsold.total()) } else { "counting…".into() };
                    widgets::kv(ui, "Unsold (est.)", widgets::value(unsold).color(theme::accent_hi()));
                    widgets::kv(ui, "Sold this session", widgets::value(format_credits(s.exploration_earned)));
                });

                ui.add_space(10.0);
                widgets::section(ui, "Exobiology", None);
                egui::Grid::new("stats-exobio").num_columns(2).min_col_width(180.0).spacing([16.0, 6.0]).show(ui, |ui| {
                    widgets::kv(ui, "Samples taken", widgets::value(group_thousands(s.bio_samples as u64)));
                    widgets::kv(ui, "Species completed", widgets::value(group_thousands(s.bio_species_complete as u64)));
                    let unsold = if self.unsold.loaded { format_credits(self.unsold.bio_total()) } else { "counting…".into() };
                    widgets::kv(ui, "Unsold (est.)", widgets::value(unsold).color(theme::accent_hi()));
                    widgets::kv(ui, "Sold this session", widgets::value(format_credits(s.exobio_earned)));
                });

                ui.add_space(12.0);
                widgets::hint(ui, "Values are estimates — actual payouts vary with mapping and first-discovery bonuses.");
            });
        });
    }
}
