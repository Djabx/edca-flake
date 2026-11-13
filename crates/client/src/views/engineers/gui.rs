//! Desktop-window drawing for [`EngineersView`]: the engineer list beside the selected
//! engineer's blueprints, each grade with its material cost against what the commander holds
//! and buttons to queue it on the todo list.

use eframe::egui::{self, pos2, vec2, Align, Align2, Layout, Rect, Sense, Ui, UiBuilder};

use super::{EngineersView, Kind, Panel};
use crate::engineering_data::Modification;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::gui::theme::{self, accent, accent_hi, alpha, CYAN, GREEN, GREY, RED, TEXT, TEXT_DIM, YELLOW};
use crate::gui::widgets;
use crate::journal_reader::JournalData;
use crate::todo::ModKind;
use crate::views::pilot::gui::grade_pips;
use crate::views::util::have_count;
use crate::views::ViewEvent;

const HEAD_H: f32 = 30.0;
const MAT_H: f32 = 22.0;

/// What the mouse asked for on a grade block this frame.
enum Action {
    Select(usize),
    Add(usize),
    Remove(usize),
}

impl EngineersView {
    pub fn ui(&mut self, ui: &mut Ui, journal: &JournalData) -> ViewEvent {
        // Labels must not take clicks meant for the rows they sit on.
        ui.style_mut().interaction.selectable_labels = false;
        let full = ui.available_rect_before_wrap();
        let left_w = (full.width() * 0.22).clamp(220.0, 300.0);
        let left = Rect::from_min_size(full.min, vec2(left_w, full.height()));
        let right = Rect::from_min_max(pos2(left.right() + 10.0, full.top()), full.max);

        let prev_id = ui.id().with("engineers-prev");
        let prev: Option<(usize, usize)> = ui.data(|d| d.get_temp(prev_id));
        let follow_eng = prev.map(|p| p.0) != Some(self.engineer_idx);
        let follow_mod = prev != Some((self.engineer_idx, self.mod_grade_idx));

        let mut event = ViewEvent::None;
        let count = self.engineers().len();
        widgets::hud_panel_at(ui, left, "Engineers", &count.to_string(), |ui| {
            event = self.engineer_list(ui, follow_eng);
        });

        let caption = if self.status.is_empty() { "enter add · r remove".to_string() } else { self.status.clone() };
        let mut action = None;
        widgets::hud_panel_at(ui, right, "Blueprints", &caption, |ui| {
            action = self.mods_panel(ui, journal, follow_mod);
        });

        if let Some(a) = action {
            let (i, key) = match a {
                Action::Select(i) => (i, None),
                Action::Add(i) => (i, Some(KeyCode::Enter)),
                Action::Remove(i) => (i, Some(KeyCode::Char('r'))),
            };
            self.mod_grade_idx = i;
            self.panel = Panel::Mods;
            match key {
                Some(k) => event = self.handle_key(&KeyEvent::from(k)),
                None => self.status.clear(),
            }
        }
        ui.data_mut(|d| d.insert_temp(prev_id, (self.engineer_idx, self.mod_grade_idx)));
        event
    }

    fn engineer_list(&mut self, ui: &mut Ui, follow: bool) -> ViewEvent {
        let mut event = ViewEvent::None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (k, label) in [(Kind::Ship, "Ship"), (Kind::OnFoot, "On foot")] {
                let on = self.kind == k;
                let resp = widgets::tab_button(ui, label, on, 24.0).on_hover_text("Switch ship / on-foot engineers (t)");
                if resp.clicked() && !on {
                    event = self.handle_key(&KeyEvent::from(KeyCode::Char('t')));
                }
            }
        });
        ui.add_space(6.0);

        let focused = self.panel == Panel::Engineers;
        let mut clicked = None;
        egui::ScrollArea::vertical().id_salt("engineers-list").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (i, eng) in self.engineers().iter().enumerate() {
                let sel = i == self.engineer_idx;
                let solid = sel && focused;
                let resp = widgets::selectable_row(ui, solid, 26.0, |ui| {
                    let col = if sel && !focused { accent_hi() } else { widgets::row_text_color(solid, TEXT) };
                    let font = if sel { theme::body_semi(15.5) } else { theme::body(15.5) };
                    ui.label(egui::RichText::new(&eng.name).font(font).color(col));
                });
                if sel && !focused {
                    let r = resp.rect;
                    ui.painter().rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 0.0, accent());
                }
                let resp = resp.on_hover_text(format!("{} / {}", eng.system, eng.station));
                if sel && follow {
                    resp.scroll_to_me(Some(Align::Center));
                }
                if resp.clicked() {
                    clicked = Some(i);
                }
            }
        });
        if let Some(i) = clicked {
            if i != self.engineer_idx {
                self.engineer_idx = i;
                self.mod_grade_idx = 0;
            }
            self.panel = Panel::Engineers;
            self.status.clear();
        }
        event
    }

    fn mods_panel(&self, ui: &mut Ui, journal: &JournalData, follow: bool) -> Option<Action> {
        let Some(eng) = self.selected_engineer() else {
            widgets::empty_state(ui, "No engineer selected.");
            return None;
        };
        ui.label(widgets::job(&eng.name.to_uppercase(), theme::head_bold(14.0), accent_hi(), 1.6));
        ui.add_space(2.0);
        egui::Grid::new("engineer-info").num_columns(2).spacing([12.0, 3.0]).show(ui, |ui| {
            widgets::kv(ui, "Workshop", widgets::value(format!("{}  ·  {}", eng.station, eng.system)));
            ui.label(egui::RichText::new("Unlock").color(CYAN).font(theme::body(15.0)));
            ui.add(egui::Label::new(egui::RichText::new(&eng.unlock).color(TEXT_DIM).font(theme::body(14.5))).wrap());
            ui.end_row();
        });
        ui.add_space(6.0);

        let flat = self.flat_mod_grades();
        if flat.is_empty() {
            widgets::empty_state(ui, "No modifications listed for this engineer.");
            return None;
        }

        let kind = match self.kind {
            Kind::Ship => ModKind::Ship,
            Kind::OnFoot => ModKind::OnFoot,
        };
        let focused = self.panel == Panel::Mods;
        let mut action = None;
        egui::ScrollArea::vertical().id_salt(("engineer-mods", self.engineer_idx)).auto_shrink([false, false]).show(ui, |ui| {
            let mut current_type: Option<&str> = None;
            let mut current_mod: Option<&str> = None;
            for (i, (m, grade)) in flat.iter().enumerate() {
                if current_type != Some(m.module_type.as_str()) {
                    if current_type.is_some() {
                        ui.add_space(8.0);
                    }
                    current_type = Some(&m.module_type);
                    let n = flat.iter().filter(|(x, _)| x.module_type == m.module_type).map(|(x, _)| &x.id).collect::<std::collections::HashSet<_>>().len();
                    widgets::section(ui, &m.module_type, Some(n));
                }
                if current_mod != Some(m.id.as_str()) {
                    current_mod = Some(&m.id);
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(&m.name).font(theme::body_semi(16.5)).color(YELLOW));
                        ui.label(egui::RichText::new(&m.effect).font(theme::body(14.5)).color(TEXT_DIM));
                    });
                }
                let sel = i == self.mod_grade_idx;
                if let Some(a) = self.grade_block(ui, i, m, *grade, &kind, journal, sel, focused, sel && follow) {
                    action = Some(a);
                }
            }
        });
        action
    }

    /// One grade of a blueprint: a clickable block with the grade, how many are queued, the
    /// add / remove buttons and each material needed against the commander's stock.
    #[allow(clippy::too_many_arguments)]
    fn grade_block(
        &self,
        ui: &mut Ui,
        i: usize,
        m: &Modification,
        grade: u8,
        kind: &ModKind,
        journal: &JournalData,
        sel: bool,
        focused: bool,
        follow: bool,
    ) -> Option<Action> {
        let costs = m.grades.get(&grade.to_string()).map(Vec::as_slice).unwrap_or(&[]);
        let h = HEAD_H + costs.len() as f32 * MAT_H + 6.0;
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
        if follow {
            resp.scroll_to_me(Some(Align::Center));
        }
        if !ui.is_rect_visible(rect) {
            return None;
        }
        let acc = accent();
        let p = ui.painter();
        if sel {
            p.rect_filled(rect, 0.0, alpha(acc, if focused { 46 } else { 22 }));
            p.rect_filled(Rect::from_min_size(rect.min, vec2(3.0, rect.height())), 0.0, if focused { acc } else { alpha(acc, 140) });
        } else if resp.hovered() {
            p.rect_filled(rect, 0.0, alpha(acc, 18));
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let queued = self.todo_idx(&m.id, grade).map(|idx| self.todo.items[idx].count).unwrap_or(0);
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), HEAD_H)).shrink2(vec2(12.0, 0.0));
        let mut action = None;
        let mut left = ui.new_child(UiBuilder::new().max_rect(head).layout(Layout::left_to_right(Align::Center)));
        match self.kind {
            Kind::Ship => {
                left.label(widgets::job(&format!("G{grade}"), theme::head_bold(11.0), if sel { accent_hi() } else { TEXT }, 1.0));
                grade_pips(&mut left, grade, acc);
            }
            Kind::OnFoot => {
                left.label(widgets::job("MATERIALS", theme::head(9.5), TEXT_DIM, 1.2));
            }
        }
        if queued > 0 {
            left.add_space(6.0);
            widgets::badge(&mut left, &format!("Queued ×{queued}"), GREEN);
        }
        let mut right = ui.new_child(UiBuilder::new().max_rect(head).layout(Layout::right_to_left(Align::Center)));
        if widgets::hud_button(&mut right, "+ Todo").on_hover_text("Add one to the todo list (enter)").clicked() {
            action = Some(Action::Add(i));
        }
        if widgets::hud_button_enabled(&mut right, "− Todo", queued > 0).on_hover_text("Remove one from the todo list (r / del)").clicked() {
            action = Some(Action::Remove(i));
        }

        let name_x = rect.left() + 34.0;
        let have_x = rect.right() - 12.0;
        let bar_w = (rect.width() * 0.14).clamp(50.0, 110.0);
        let bar_r = have_x - 78.0;
        let need_x = bar_r - bar_w - 12.0;
        for (k, cost) in costs.iter().enumerate() {
            let y = rect.top() + HEAD_H + k as f32 * MAT_H + MAT_H / 2.0;
            let have = have_count(&cost.name, kind, journal);
            let enough = have >= cost.count as i32;
            let col = if enough { GREEN } else { RED };
            let p = ui.painter_at(rect);
            p.circle_filled(pos2(name_x - 10.0, y), 2.5, col);
            let np = ui.painter_at(Rect::from_min_max(pos2(name_x, y - MAT_H / 2.0), pos2(need_x - 40.0, y + MAT_H / 2.0)));
            widgets::txt(&np, pos2(name_x, y), Align2::LEFT_CENTER, &cost.name, theme::body(15.0), col, 0.0);
            widgets::txt(&p, pos2(need_x, y), Align2::RIGHT_CENTER, &format!("×{}", cost.count), theme::body_semi(15.0), TEXT, 0.0);
            let bar = Rect::from_min_max(pos2(bar_r - bar_w, y - 3.5), pos2(bar_r, y + 3.5));
            widgets::seg_bar(&p, bar, have as f32 / cost.count.max(1) as f32, col);
            widgets::txt(&p, pos2(have_x, y), Align2::RIGHT_CENTER, &format!("{have} have"), theme::body(14.5), if enough { TEXT_DIM } else { GREY }, 0.0);
        }

        if action.is_none() && resp.clicked() {
            action = Some(Action::Select(i));
        }
        action
    }
}
