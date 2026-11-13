mod gui;


use crate::engineering_data::{self, EngineerInfo, Modification};
use crate::event_shim::{KeyCode, KeyEvent};
use crate::todo::{ModKind, TodoItem, TodoList};
use crate::views::ViewEvent;

#[derive(Clone, PartialEq)]
enum Panel {
    Engineers,
    Mods,
}

#[derive(Clone, PartialEq)]
enum Kind {
    Ship,
    OnFoot,
}

pub struct EngineersView {
    kind: Kind,
    engineer_idx: usize,
    /// Flat index into all (mod, grade) pairs for the selected engineer.
    mod_grade_idx: usize,
    panel: Panel,
    pub todo: TodoList,
    status: String,
}

impl EngineersView {
    pub fn new() -> Self {
        Self {
            kind: Kind::Ship,
            engineer_idx: 0,
            mod_grade_idx: 0,
            panel: Panel::Engineers,
            todo: TodoList::load(),
            status: String::new(),
        }
    }

    fn engineers(&self) -> &'static [EngineerInfo] {
        match self.kind {
            Kind::Ship => &engineering_data::engineers().ship,
            Kind::OnFoot => &engineering_data::engineers().onfoot,
        }
    }

    fn selected_engineer(&self) -> Option<&'static EngineerInfo> {
        self.engineers().get(self.engineer_idx)
    }

    fn mods_for_selected(&self) -> Vec<&'static Modification> {
        let eng = match self.selected_engineer() {
            Some(e) => e,
            None => return vec![],
        };
        let all_mods = match self.kind {
            Kind::Ship => &engineering_data::modifications().ship,
            Kind::OnFoot => &engineering_data::modifications().onfoot,
        };
        let mut mods: Vec<&'static Modification> = all_mods
            .iter()
            .filter(|m| m.engineer_ids.contains(&eng.id))
            .collect();
        // Alphabetic within each module-type group, so both the section
        // headers and the mods listed under them read top-to-bottom sorted.
        mods.sort_by(|a, b| a.module_type.cmp(&b.module_type).then_with(|| a.name.cmp(&b.name)));
        mods
    }

    /// Index into `self.todo.items` of the queued entry for this mod+grade, if any.
    fn todo_idx(&self, mod_id: &str, grade: u8) -> Option<usize> {
        self.todo.items.iter().position(|i| i.mod_id == mod_id && i.grade == grade)
    }

    fn flat_mod_grades(&self) -> Vec<(&'static Modification, u8)> {
        self.mods_for_selected()
            .into_iter()
            .flat_map(|m| {
                // On-foot mods are always applied at max grade — no intermediate grades.
                let grades: Box<dyn Iterator<Item = u8>> = match self.kind {
                    Kind::OnFoot => Box::new(std::iter::once(m.max_grade)),
                    Kind::Ship => Box::new(1..=m.max_grade),
                };
                grades.map(move |g| (m, g))
            })
            .collect()
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> ViewEvent {
        self.status.clear();
        match key.code {
            KeyCode::Char('t') => {
                self.kind = match self.kind {
                    Kind::Ship => Kind::OnFoot,
                    Kind::OnFoot => Kind::Ship,
                };
                self.engineer_idx = 0;
                self.mod_grade_idx = 0;
            }
            KeyCode::Tab => {
                self.panel = match self.panel {
                    Panel::Engineers => Panel::Mods,
                    Panel::Mods => Panel::Engineers,
                };
            }
            KeyCode::Char('d') | KeyCode::Right => {
                if self.panel == Panel::Engineers {
                    self.panel = Panel::Mods;
                }
            }
            KeyCode::Char('a') | KeyCode::Left => {
                if self.panel == Panel::Mods {
                    self.panel = Panel::Engineers;
                }
            }
            KeyCode::Char('w') | KeyCode::Up => match self.panel {
                Panel::Engineers => {
                    if self.engineer_idx > 0 {
                        self.engineer_idx -= 1;
                        self.mod_grade_idx = 0;
                    }
                }
                Panel::Mods => {
                    if self.mod_grade_idx > 0 {
                        self.mod_grade_idx -= 1;
                    }
                }
            },
            KeyCode::Char('s') | KeyCode::Down => match self.panel {
                Panel::Engineers => {
                    if self.engineer_idx + 1 < self.engineers().len() {
                        self.engineer_idx += 1;
                        self.mod_grade_idx = 0;
                    }
                }
                Panel::Mods => {
                    let total = self.flat_mod_grades().len();
                    if self.mod_grade_idx + 1 < total {
                        self.mod_grade_idx += 1;
                    }
                }
            },
            KeyCode::PageUp => match self.panel {
                Panel::Engineers => {
                    self.engineer_idx = self.engineer_idx.saturating_sub(10);
                    self.mod_grade_idx = 0;
                }
                Panel::Mods => {
                    self.mod_grade_idx = self.mod_grade_idx.saturating_sub(10);
                }
            },
            KeyCode::PageDown => match self.panel {
                Panel::Engineers => {
                    let max = self.engineers().len().saturating_sub(1);
                    self.engineer_idx = (self.engineer_idx + 10).min(max);
                    self.mod_grade_idx = 0;
                }
                Panel::Mods => {
                    let total = self.flat_mod_grades().len();
                    self.mod_grade_idx = (self.mod_grade_idx + 10).min(total.saturating_sub(1));
                }
            },
            KeyCode::Enter => {
                if self.panel == Panel::Mods {
                    let flat = self.flat_mod_grades();
                    if let Some((m, grade)) = flat.get(self.mod_grade_idx) {
                        let kind = match self.kind {
                            Kind::Ship => ModKind::Ship,
                            Kind::OnFoot => ModKind::OnFoot,
                        };
                        self.todo.add(TodoItem {
                            mod_id: m.id.clone(),
                            grade: *grade,
                            mod_name: m.name.clone(),
                            module_type: m.module_type.clone(),
                            kind,
                            count: 1,
                        });
                        self.status = format!("Added {} G{} to todo list", m.name, grade);
                    }
                }
            }
            KeyCode::Delete | KeyCode::Char('r') => {
                if self.panel == Panel::Mods {
                    let flat = self.flat_mod_grades();
                    if let Some((m, grade)) = flat.get(self.mod_grade_idx) {
                        if let Some(idx) = self.todo_idx(&m.id, *grade) {
                            self.todo.decrement_or_remove(idx);
                            self.status = format!("Removed one {} G{} from todo list", m.name, grade);
                        }
                    }
                }
            }
            _ => {}
        }
        ViewEvent::None
    }
}
