//! Desktop-window drawing for [`WorkshopView`]: a sub-tab strip over Inventory / Engineers.

use eframe::egui::Ui;

use super::{SubTab, WorkshopView};
use crate::gui::widgets;
use crate::journal_reader::JournalData;
use crate::views::ViewEvent;

const TABS: [(SubTab, &str); 2] = [(SubTab::Inventory, "Inventory"), (SubTab::Engineers, "Engineers")];

impl WorkshopView {
    pub fn ui(&mut self, ui: &mut Ui, journal: &JournalData) -> ViewEvent {
        let current = TABS.iter().position(|(t, _)| *t == self.tab).unwrap_or(0);
        let labels: Vec<&str> = TABS.iter().map(|(_, l)| *l).collect();
        if let Some(i) = widgets::tab_strip(ui, &labels, current) {
            self.tab = TABS[i].0;
        }
        ui.add_space(8.0);
        match self.tab {
            SubTab::Inventory => self.inventory.ui(ui, journal),
            SubTab::Engineers => self.engineers.ui(ui, journal),
        }
    }
}
