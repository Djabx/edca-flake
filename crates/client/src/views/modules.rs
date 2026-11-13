mod gui;

use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::JournalData;
use crate::views::ViewEvent;
use ratatui::style::Color;

pub struct ModulesView {
    selected: usize,
}

impl ModulesView {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn handle_key(&mut self, key: &KeyEvent, journal: &JournalData) -> ViewEvent {
        let count = journal.modules.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('w') => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('s') => {
                if count > 0 {
                    self.selected = (self.selected + 1).min(count - 1);
                }
            }
            KeyCode::PageUp => {
                self.selected = self.selected.saturating_sub(10);
            }
            KeyCode::PageDown => {
                if count > 0 {
                    self.selected = (self.selected + 10).min(count - 1);
                }
            }
            _ => return ViewEvent::None,
        }
        ViewEvent::Consumed
    }
}

use edcas_common::format::group_thousands as fmt_credits;

fn condition_color(health: Option<f32>) -> Color {
    match health {
        Some(h) if h > 0.75 => Color::Green,
        Some(h) if h > 0.50 => Color::Yellow,
        Some(h) if h > 0.25 => crate::theme::accent(),
        Some(_) => Color::Red,
        None => Color::DarkGray,
    }
}
