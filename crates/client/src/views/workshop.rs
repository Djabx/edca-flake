mod gui;

use crate::event_shim::{KeyCode, KeyEvent};
use crate::views::engineers::EngineersView;
use crate::views::inventory::InventoryView;
use crate::views::ViewEvent;

#[derive(Clone, Copy, PartialEq)]
enum SubTab { Inventory, Engineers }

pub struct WorkshopView {
    tab: SubTab,
    pub inventory: InventoryView,
    pub engineers: EngineersView,
}

impl WorkshopView {
    pub fn new() -> Self {
        Self {
            tab: SubTab::Inventory,
            inventory: InventoryView::new(),
            engineers: EngineersView::new(),
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> ViewEvent {
        if key.code == KeyCode::Tab {
            self.tab = match self.tab {
                SubTab::Inventory => SubTab::Engineers,
                SubTab::Engineers => SubTab::Inventory,
            };
            return ViewEvent::Consumed;
        }
        match self.tab {
            SubTab::Inventory => self.inventory.handle_key(key),
            SubTab::Engineers => self.engineers.handle_key(key),
        }
    }
}
