use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModKind {
    Ship,
    OnFoot,
}

fn default_count() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub mod_id: String,
    pub grade: u8,
    pub mod_name: String,
    pub module_type: String,
    pub kind: ModKind,
    #[serde(default = "default_count")]
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructionTodoResource {
    pub commodity_name: String,
    pub display_name: String,
    pub required_amount: i32,
    pub provided_amount: i32,
    pub payment: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructionTodoItem {
    pub market_id: i64,
    pub station_name: String,
    pub system_name: String,
    pub resources: Vec<ConstructionTodoResource>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
    #[serde(default)]
    pub construction_items: Vec<ConstructionTodoItem>,
}

impl TodoList {
    pub fn load() -> Self {
        let mut list: Self = {
            let path = Self::native_path();
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|data| serde_json::from_str(&data).ok())
                .unwrap_or_default()
        };
        list.sort_items();
        list
    }

    /// Alphabetic by mod name (case-insensitive), grade as a tiebreaker — keeps
    /// the Todo tab and the Engineers tab's queued list in the same order.
    fn sort_items(&mut self) {
        self.items.sort_by(|a, b| {
            a.mod_name.to_lowercase().cmp(&b.mod_name.to_lowercase()).then(a.grade.cmp(&b.grade))
        });
    }

    pub fn save(&self) {
        {
            let path = Self::native_path();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(data) = serde_json::to_string_pretty(self) {
                // Write to a temp file in the same directory, then rename atomically.
                // This ensures a crash during write never leaves a corrupt todo.json.
                if let Some(parent) = path.parent() {
                    let tmp = parent.join("todo.json.tmp");
                    if std::fs::write(&tmp, &data).is_ok() {
                        let _ = std::fs::rename(&tmp, &path);
                    }
                }
            }
        }
    }

    pub fn add(&mut self, item: TodoItem) {
        if let Some(existing) = self.items.iter_mut().find(|i| i.mod_id == item.mod_id && i.grade == item.grade) {
            existing.count += 1;
        } else {
            self.items.push(item);
            self.sort_items();
        }
        self.save();
    }

    pub fn decrement_or_remove(&mut self, idx: usize) -> bool {
        if idx >= self.items.len() { return false; }
        if self.items[idx].count > 1 {
            self.items[idx].count -= 1;
            self.save();
            false
        } else {
            self.items.remove(idx);
            self.save();
            true
        }
    }


    pub fn add_construction_item(&mut self, item: ConstructionTodoItem) {
        if !self.construction_items.iter().any(|i| i.market_id == item.market_id) {
            self.construction_items.push(item);
            self.save();
        }
    }

    pub fn remove_construction_item(&mut self, market_id: i64) {
        let before = self.construction_items.len();
        self.construction_items.retain(|i| i.market_id != market_id);
        if self.construction_items.len() != before {
            self.save();
        }
    }

    /// Update a pinned construction item's resource snapshot (e.g. after re-docking).
    pub fn update_construction_item(&mut self, item: ConstructionTodoItem) {
        if let Some(existing) = self.construction_items.iter_mut().find(|i| i.market_id == item.market_id) {
            *existing = item;
            self.save();
        }
    }

    fn native_path() -> std::path::PathBuf {
        crate::settings::config_dir().join("todo.json")
    }
}
