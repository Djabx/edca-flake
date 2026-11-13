mod gui;

use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::JournalData;
use crate::views::modules::ModulesView;
use crate::views::pilot::PilotView;
use crate::views::stats::StatsView;
use crate::views::ViewEvent;

#[derive(Clone, Copy, PartialEq)]
enum SubTab { Pilot, Ship, Stats }

pub struct CommanderView {
    tab: SubTab,
    pilot: PilotView,
    modules: ModulesView,
    stats: StatsView,
}

impl CommanderView {
    pub fn new() -> Self {
        Self {
            tab: SubTab::Pilot,
            pilot: PilotView::new(),
            modules: ModulesView::new(),
            stats: StatsView::new(),
        }
    }

    pub fn set_unsold(&mut self, unsold: std::sync::Arc<crate::cartography::UnsoldData>) {
        self.stats.unsold = unsold;
    }

    pub fn handle_key(&mut self, key: &KeyEvent, journal: &JournalData) -> ViewEvent {
        if key.code == KeyCode::Tab {
            self.tab = match self.tab {
                SubTab::Pilot => SubTab::Ship,
                SubTab::Ship => SubTab::Stats,
                SubTab::Stats => SubTab::Pilot,
            };
            return ViewEvent::Consumed;
        }
        match self.tab {
            SubTab::Pilot => self.pilot.handle_key(key),
            SubTab::Ship => self.modules.handle_key(key, journal),
            SubTab::Stats => ViewEvent::None,
        }
    }
}
