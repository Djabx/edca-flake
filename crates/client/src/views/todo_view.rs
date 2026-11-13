mod gui;


use std::collections::HashMap;

use crate::engineering_data::{self, MaterialCost};
use crate::event_shim::{KeyCode, KeyEvent};
use crate::journal_reader::JournalData;
use crate::todo::{ModKind, TodoList};
use crate::views::util::have_count;
use crate::views::ViewEvent;

enum SelectedKind {
    Engineering(usize),
    Construction(usize),
}

#[derive(Clone, Copy, PartialEq)]
enum TodoFocus {
    Left,
    Right,
}

pub struct TodoView {
    pub todo: TodoList,
    selected_idx: usize,
    show_aggregate: bool,
    focus: TodoFocus,
    resource_selected_idx: usize,
}

impl TodoView {
    pub fn new() -> Self {
        Self {
            todo: TodoList::load(),
            selected_idx: 0,
            show_aggregate: false,
            focus: TodoFocus::Left,
            resource_selected_idx: 0,
        }
    }

    fn total_count(&self) -> usize {
        self.todo.items.len() + self.todo.construction_items.len()
    }

    fn selected_kind(&self) -> Option<SelectedKind> {
        let eng = self.todo.items.len();
        let con = self.todo.construction_items.len();
        if eng + con == 0 {
            return None;
        }
        if self.selected_idx < eng {
            Some(SelectedKind::Engineering(self.selected_idx))
        } else {
            let ci = self.selected_idx - eng;
            if ci < con {
                Some(SelectedKind::Construction(ci))
            } else {
                None
            }
        }
    }

    fn grade_costs(mod_id: &str, grade: u8, kind: &ModKind) -> Vec<MaterialCost> {
        let all_mods = match kind {
            ModKind::Ship => &engineering_data::modifications().ship,
            ModKind::OnFoot => &engineering_data::modifications().onfoot,
        };
        all_mods
            .iter()
            .find(|m| m.id == mod_id)
            .and_then(|m| m.grades.get(&grade.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    /// Construction sites grouped by system name, in first-seen order, as indices into
    /// `todo.construction_items`.
    fn construction_groups(&self) -> Vec<(String, Vec<(usize, &crate::todo::ConstructionTodoItem)>)> {
        let mut groups: Vec<(String, Vec<(usize, &crate::todo::ConstructionTodoItem)>)> = Vec::new();
        for (i, site) in self.todo.construction_items.iter().enumerate() {
            let sys = if site.system_name.is_empty() { "Unknown System".to_string() } else { site.system_name.clone() };
            if let Some(g) = groups.iter_mut().find(|(s, _)| s == &sys) {
                g.1.push((i, site));
            } else {
                groups.push((sys, vec![(i, site)]));
            }
        }
        groups
    }

    /// A site's resources as (raw name, display name, provided, required, payment), from live
    /// journal data when docked there, else from the saved todo.
    fn site_resources(site: &crate::todo::ConstructionTodoItem, journal: &JournalData) -> Vec<(String, String, i32, i32, i64)> {
        if let Some(depot) = journal.construction_depots.get(&site.market_id) {
            depot.submission.resources.iter().map(|r| {
                (r.name.clone(), r.display_name.clone(), r.provided_amount, r.required_amount, r.payment)
            }).collect()
        } else {
            site.resources.iter().map(|r| {
                (r.commodity_name.clone(), r.display_name.clone(), r.provided_amount, r.required_amount, r.payment)
            }).collect()
        }
    }

    /// Every engineering todo's materials summed as (name, needed, kind), missing ones first,
    /// then by name.
    fn aggregate_materials(&self, journal: &JournalData) -> Vec<(String, i32, ModKind)> {
        let mut totals: HashMap<String, (i32, ModKind)> = HashMap::new();
        for item in &self.todo.items {
            for cost in Self::grade_costs(&item.mod_id, item.grade, &item.kind) {
                let entry = totals.entry(cost.name.clone()).or_insert((0, item.kind.clone()));
                entry.0 += cost.count as i32 * item.count as i32;
            }
        }
        let mut sorted: Vec<(String, i32, ModKind)> = totals
            .into_iter()
            .map(|(name, (needed, kind))| (name, needed, kind))
            .collect();
        sorted.sort_by(|a, b| {
            let ok_a = have_count(&a.0, &a.2, journal) >= a.1;
            let ok_b = have_count(&b.0, &b.2, journal) >= b.1;
            ok_a.cmp(&ok_b).then(a.0.cmp(&b.0))
        });
        sorted
    }

    pub fn handle_key(&mut self, key: &KeyEvent, journal: &JournalData) -> ViewEvent {
        let total = self.total_count();

        // Tab switches focus between left panel and right panel
        if key.code == KeyCode::Tab {
            match self.focus {
                TodoFocus::Left => {
                    if matches!(self.selected_kind(), Some(SelectedKind::Construction(_))) {
                        self.focus = TodoFocus::Right;
                        self.resource_selected_idx = 0;
                    }
                }
                TodoFocus::Right => {
                    self.focus = TodoFocus::Left;
                }
            }
            return ViewEvent::Consumed;
        }

        if self.focus == TodoFocus::Right {
            return match key.code {
                KeyCode::Char('w') | KeyCode::Up => {
                    self.resource_selected_idx = self.resource_selected_idx.saturating_sub(1);
                    ViewEvent::Consumed
                }
                KeyCode::Char('s') | KeyCode::Down => {
                    let resource_count = match self.selected_kind() {
                        Some(SelectedKind::Construction(idx)) => {
                            let site = &self.todo.construction_items[idx];
                            let live = journal.construction_depots.get(&site.market_id);
                            if let Some(depot) = live {
                                depot.submission.resources.len()
                            } else {
                                site.resources.len()
                            }
                        }
                        _ => 0,
                    };
                    if self.resource_selected_idx + 1 < resource_count {
                        self.resource_selected_idx += 1;
                    }
                    ViewEvent::Consumed
                }
                KeyCode::Char('f') => {
                    if let Some(SelectedKind::Construction(idx)) = self.selected_kind() {
                        let site = &self.todo.construction_items[idx];
                        let live = journal.construction_depots.get(&site.market_id);
                        let names = if let Some(depot) = live {
                            depot.submission.resources
                                .get(self.resource_selected_idx)
                                .map(|r| (r.display_name.clone(), r.name.clone()))
                        } else {
                            site.resources
                                .get(self.resource_selected_idx)
                                .map(|r| (r.display_name.clone(), r.commodity_name.clone()))
                        };
                        if let Some((commodity, raw_name)) = names {
                            let canonical_name = super::search_nearest::resolve_commodity_canonical(&raw_name);
                            let system = journal.current_system
                                .as_ref()
                                .map(|s| s.name.clone())
                                .unwrap_or_default();
                            let ship_pad_size = journal.pilot.ship_pad_size;
                            return ViewEvent::OpenSearchNearest { commodity, canonical_name, system, ship_pad_size };
                        }
                    }
                    ViewEvent::Consumed
                }
                _ => ViewEvent::None,
            };
        }

        // Left panel navigation
        match key.code {
            KeyCode::Char('w') | KeyCode::Up => {
                if self.selected_idx > 0 {
                    self.selected_idx -= 1;
                    self.resource_selected_idx = 0;
                }
            }
            KeyCode::Char('s') | KeyCode::Down => {
                if self.selected_idx + 1 < total {
                    self.selected_idx += 1;
                    self.resource_selected_idx = 0;
                }
            }
            KeyCode::PageUp => {
                self.selected_idx = self.selected_idx.saturating_sub(10);
                self.resource_selected_idx = 0;
            }
            KeyCode::PageDown => {
                if self.selected_idx + 10 < total {
                    self.selected_idx += 10;
                } else if total > 0 {
                    self.selected_idx = total - 1;
                }
                self.resource_selected_idx = 0;
            }
            KeyCode::Char('g') => {
                self.show_aggregate = !self.show_aggregate;
            }
            KeyCode::Delete | KeyCode::Char('r') => {
                match self.selected_kind() {
                    Some(SelectedKind::Engineering(idx)) => {
                        let removed = self.todo.decrement_or_remove(idx);
                        if removed && self.selected_idx > 0 && self.selected_idx >= self.total_count() {
                            self.selected_idx -= 1;
                        }
                    }
                    Some(SelectedKind::Construction(idx)) => {
                        let mid = self.todo.construction_items[idx].market_id;
                        self.todo.remove_construction_item(mid);
                        if self.selected_idx > 0 && self.selected_idx >= self.total_count() {
                            self.selected_idx -= 1;
                        }
                    }
                    None => {}
                }
            }
            _ => {}
        }
        ViewEvent::None
    }
}

