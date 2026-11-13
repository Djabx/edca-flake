//! Desktop-window drawing for [`CommanderView`]: a sub-tab strip over Pilot / Ship / Stats.

use eframe::egui::Ui;

use super::{CommanderView, SubTab};
use crate::gui::widgets;
use crate::journal_reader::JournalData;
use crate::views::ViewEvent;

const TABS: [(SubTab, &str); 3] = [(SubTab::Pilot, "Pilot"), (SubTab::Ship, "Ship"), (SubTab::Stats, "Stats")];

impl CommanderView {
    pub fn ui(&mut self, ui: &mut Ui, journal: &JournalData) -> ViewEvent {
        let current = TABS.iter().position(|(t, _)| *t == self.tab).unwrap_or(0);
        let labels: Vec<&str> = TABS.iter().map(|(_, l)| *l).collect();
        if let Some(i) = widgets::tab_strip(ui, &labels, current) {
            self.tab = TABS[i].0;
        }
        ui.add_space(8.0);
        match self.tab {
            SubTab::Pilot => {
                self.pilot.ui(ui, journal);
                ViewEvent::None
            }
            SubTab::Ship => self.modules.ui(ui, journal),
            SubTab::Stats => {
                self.stats.ui(ui, journal);
                ViewEvent::None
            }
        }
    }
}
