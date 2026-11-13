//! Desktop-window drawing for [`FactionsView`]: a search box and faction list (pinned first)
//! beside a details panel with Info / Systems / History sub-tabs. History is a line chart of
//! the faction's influence in one system.

use eframe::egui::{
    self, pos2, text::LayoutJob, text::TextWrapping, vec2, Align, Align2, Color32, FontId, Painter, Rect,
    Sense, Shape, Stroke, TextFormat, Ui,
};

use super::{allegiance_info, government_info, DetailTab, FactionsView, SortDir, SystemSort};
use crate::api_client::ApiClient;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::gui::theme::{self, accent, accent_dim, accent_hi, alpha, BG, GREEN, GREY, PANEL, RED, TEXT, TEXT_DIM, YELLOW};
use crate::gui::widgets;
use crate::views::util::{fmt_ts_short, status_style, FocusArea, SearchState};
use crate::views::ViewEvent;

const DETAIL_TABS: [DetailTab; 3] = [DetailTab::Info, DetailTab::Systems, DetailTab::History];
const SORT_COLUMNS: [(SystemSort, &str, char); 5] = [
    (SystemSort::Name, "System", '1'),
    (SystemSort::Influence, "Influence", '2'),
    (SystemSort::ActiveState, "Active state", '3'),
    (SystemSort::Pending, "Pending", '4'),
    (SystemSort::Updated, "Updated", '5'),
];
/// Height of one faction / system row.
const ROW_H: f32 = 26.0;
/// Extra height of a system row that carries a conflict line.
const CONFLICT_H: f32 = 20.0;

/// Something clicked in the systems table, applied once the faction borrow has ended.
enum SystemAction {
    Select(usize),
    Copy(usize),
    History(usize),
}

impl FactionsView {
    pub fn ui(&mut self, ui: &mut Ui, api: &ApiClient) -> ViewEvent {
        let full = ui.available_rect_before_wrap();
        let list_w = (full.width() * 0.34).clamp(300.0, 470.0);
        let left = Rect::from_min_size(full.min, vec2(list_w, full.height()));
        let right = Rect::from_min_max(pos2(left.right() + 10.0, full.top()), full.max);

        let shown = self.display_count();
        let caption = if shown == 0 { String::new() } else { format!("{shown} listed") };
        let mut event = widgets::hud_panel_at(ui, left, "Factions", &caption, |ui| self.list_panel(ui, api));
        let detail_event = widgets::hud_panel_at(ui, right, "Faction details", "a/d · sub-tabs", |ui| self.detail_panel(ui, api));
        if matches!(event, ViewEvent::None) {
            event = detail_event;
        }
        focus_outline(ui.painter(), if self.focus == FocusArea::List { left } else { right });
        event
    }

    // ---------------------------------------------------------------- list

    fn list_panel(&mut self, ui: &mut Ui, api: &ApiClient) -> ViewEvent {
        let mut event = ViewEvent::None;
        if !self.embedded {
            ui.horizontal(|ui| {
                let field_w = (ui.available_width() - 86.0).max(80.0);
                let resp = widgets::text_field(ui, &mut self.search_query, "Faction name…", field_w)
                    .on_hover_text("Type a faction name and press Enter (/ or f)");
                if widgets::submitted(ui, &resp) {
                    self.search_state = SearchState::Idle;
                    self.do_search(api);
                } else if resp.lost_focus() {
                    if self.search_state == SearchState::Typing {
                        self.search_state = SearchState::Idle;
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.status_msg = "Search cancelled".into();
                        }
                    }
                } else if resp.gained_focus() {
                    if self.search_state == SearchState::Idle {
                        self.search_state = SearchState::Typing;
                        self.status_msg = "Typing… (Enter to search, Esc to cancel)".into();
                    }
                } else if self.search_state == SearchState::Typing && !resp.has_focus() {
                    resp.request_focus();
                }
                if widgets::hud_button(ui, "Search").on_hover_text("Search the EDCAS database (Enter)").clicked() {
                    self.search_state = SearchState::Idle;
                    self.do_search(api);
                }
            });
        }
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if self.loading || self.loading_pins {
                ui.add(egui::Spinner::new().size(13.0).color(accent()));
            }
            let col = status_style(&self.status_msg).fg.map(theme::from_ratatui).unwrap_or(GREY);
            ui.add(egui::Label::new(egui::RichText::new(&self.status_msg).color(col).font(theme::body(14.0))).truncate());
        });
        ui.add_space(4.0);

        let sel_changed = changed(ui, "list-sel", self.selected_idx);
        let n_pinned = self.pinned_results.len();
        let mut clicked: Option<usize> = None;
        let mut pin: Option<usize> = None;
        egui::ScrollArea::vertical().id_salt("factions-list").auto_shrink([false, false]).show(ui, |ui| {
            let found: Vec<_> = self.results.iter().filter(|f| !self.pinned_names.contains(&f.name)).collect();
            if n_pinned == 0 && found.is_empty() {
                widgets::empty_state(ui, "No results. Type a faction name above and press Enter.");
                return;
            }
            let rows = self.pinned_results.iter().map(|f| (f, true)).chain(found.iter().map(|f| (*f, false)));
            for (i, (faction, pinned)) in rows.enumerate() {
                if i == 0 && pinned {
                    widgets::section(ui, "Pinned", Some(n_pinned));
                }
                if i == n_pinned {
                    if n_pinned > 0 {
                        ui.add_space(6.0);
                    }
                    widgets::section(ui, if n_pinned > 0 { "Search results" } else { "Results" }, Some(found.len()));
                }
                let selected = i == self.selected_idx;
                let mut cut = false;
                let resp = widgets::selectable_row(ui, selected, ROW_H, |ui| {
                    let r = ui.max_rect();
                    let p = ui.painter();
                    if pinned {
                        widgets::txt(p, pos2(r.left() + 7.0, r.center().y), Align2::CENTER_CENTER, "★", theme::body(14.0), widgets::row_text_color(selected, YELLOW), 0.0);
                    }
                    let systems = match faction.presences.len() {
                        1 => "1 system".to_string(),
                        n => format!("{n} systems"),
                    };
                    let name_col = widgets::row_text_color(selected, if pinned { YELLOW } else { TEXT });
                    let font = if selected { theme::body_semi(16.0) } else { theme::body(16.0) };
                    cut = cell(p, Rect::from_min_max(pos2(r.left() + 20.0, r.top()), pos2(r.right() - 82.0, r.bottom())), &faction.name, font, name_col, Align::Min);
                    cell(p, Rect::from_min_max(pos2(r.right() - 78.0, r.top()), r.max), &systems, theme::body(14.0), widgets::row_text_color(selected, TEXT_DIM), Align::Max);
                });
                let resp = if cut { resp.on_hover_text(&faction.name) } else { resp };
                if sel_changed && selected {
                    resp.scroll_to_me(None);
                }
                if resp.clicked() {
                    clicked = Some(i);
                }
                resp.context_menu(|ui| {
                    if ui.button(if pinned { "Unpin  (p)" } else { "Pin to top  (p)" }).clicked() {
                        pin = Some(i);
                        ui.close();
                    }
                });
            }
        });
        if let Some(i) = clicked.or(pin) {
            self.select_faction(i);
        }
        if pin.is_some() {
            event = self.handle_key(&KeyEvent::from(KeyCode::Char('p')), api);
        }
        event
    }

    /// Selects list entry `i` the way moving onto it with w/s does.
    fn select_faction(&mut self, i: usize) {
        self.focus = FocusArea::List;
        if i != self.selected_idx {
            self.selected_idx = i;
            self.selected_system = 0;
            self.detail_scroll = 0;
            self.detail_tab = DetailTab::Info;
        }
    }

    // ---------------------------------------------------------------- details

    fn detail_panel(&mut self, ui: &mut Ui, api: &ApiClient) -> ViewEvent {
        let Some(faction) = self.selected_faction() else {
            widgets::empty_state(ui, "Select a faction from the list.");
            return ViewEvent::None;
        };
        let name = faction.name.clone();
        let government = faction.government.clone();
        let allegiance = faction.allegiance.clone();
        let n_systems = faction.presences.len();
        let pinned = self.selected_idx < self.pinned_results.len();

        let mut event = ViewEvent::None;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&name).font(theme::body_semi(24.0)).color(TEXT));
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let (label, tip) = if pinned {
                    ("Unpin", "Remove this faction from the pinned list (p)")
                } else {
                    ("Pin", "Keep this faction at the top of the list (p)")
                };
                if widgets::hud_button(ui, label).on_hover_text(tip).clicked() {
                    event = self.handle_key(&KeyEvent::from(KeyCode::Char('p')), api);
                }
            });
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if let Some(g) = &government {
                widgets::badge(ui, g, theme::CYAN).on_hover_text("Government");
            }
            if let Some(a) = &allegiance {
                widgets::badge(ui, a, allegiance_color(a)).on_hover_text("Allegiance");
            }
            let systems = if n_systems == 1 { "1 system".to_string() } else { format!("{n_systems} systems") };
            widgets::badge(ui, &systems, accent_dim());
            if pinned {
                widgets::badge(ui, "★ Pinned", YELLOW);
            }
        });
        ui.add_space(8.0);

        let current = DETAIL_TABS.iter().position(|t| *t == self.detail_tab).unwrap_or(0);
        let labels: Vec<&str> = DETAIL_TABS.iter().map(|t| t.label()).collect();
        if let Some(i) = widgets::tab_strip(ui, &labels, current) {
            self.open_tab(DETAIL_TABS[i], api);
        }
        ui.add_space(8.0);

        match self.detail_tab {
            DetailTab::Info => self.info_tab(ui, government.as_deref(), allegiance.as_deref()),
            DetailTab::Systems => {
                if let Some(e) = self.systems_tab(ui, api) {
                    event = e;
                }
            }
            DetailTab::History => self.history_tab(ui, api),
        }
        event
    }

    /// Switches the details sub-tab like a/d do, loading history when History opens.
    fn open_tab(&mut self, tab: DetailTab, api: &ApiClient) {
        self.focus = FocusArea::Detail;
        if tab != self.detail_tab {
            self.detail_tab = tab;
            self.detail_scroll = 0;
            if tab == DetailTab::History {
                self.fetch_history(api);
            }
        }
    }

    fn info_tab(&mut self, ui: &mut Ui, government: Option<&str>, allegiance: Option<&str>) {
        if government.is_none() && allegiance.is_none() {
            widgets::empty_state(ui, "No government / allegiance data available.");
            return;
        }
        line_scroll(ui, "factions-info", &mut self.detail_scroll, 22.0, true, |ui| {
            let government_block = |ui: &mut Ui| {
                let Some(gov) = government else { return };
                widgets::section(ui, "Government", None);
                ui.label(egui::RichText::new(gov).font(theme::body_semi(20.0)).color(accent_hi()));
                let Some((desc, rows)) = government_info(gov) else {
                    prose(ui, &format!("Government type '{gov}' — no detailed information available."));
                    return;
                };
                prose(ui, &desc.join(" "));
                ui.add_space(6.0);
                egui::Grid::new("faction-gov-rows").num_columns(2).spacing([14.0, 5.0]).show(ui, |ui| {
                    for (label, value, color) in rows {
                        ui.label(egui::RichText::new(label.trim_end_matches(':')).color(theme::CYAN).font(theme::body(15.0)));
                        ui.add(egui::Label::new(egui::RichText::new(*value).color(theme::from_ratatui(*color)).font(theme::body_semi(15.0))).wrap());
                        ui.end_row();
                    }
                });
            };
            let allegiance_block = |ui: &mut Ui| {
                let Some(alleg) = allegiance else { return };
                widgets::section(ui, "Allegiance", None);
                ui.label(egui::RichText::new(alleg).font(theme::body_semi(20.0)).color(allegiance_color(alleg)));
                let Some((desc, bullets)) = allegiance_info(alleg) else {
                    prose(ui, &format!("Allegiance '{alleg}' — no detailed information available."));
                    return;
                };
                prose(ui, &desc.join(" "));
                ui.add_space(6.0);
                for b in bullets {
                    ui.horizontal_top(|ui| {
                        ui.label(egui::RichText::new("◆").color(accent()).font(theme::body(11.0)));
                        ui.add(egui::Label::new(egui::RichText::new(*b).color(TEXT).font(theme::body(15.5))).wrap());
                    });
                }
            };
            if ui.available_width() >= 640.0 {
                ui.columns(2, |cols| {
                    let w = cols[0].available_width() - 20.0;
                    cols[0].with_layout(egui::Layout::top_down(Align::Min), |ui| {
                        ui.set_max_width(w);
                        government_block(ui);
                    });
                    cols[1].with_layout(egui::Layout::top_down(Align::Min), allegiance_block);
                });
            } else {
                government_block(ui);
                ui.add_space(14.0);
                allegiance_block(ui);
            }
        });
    }

    fn systems_tab(&mut self, ui: &mut Ui, api: &ApiClient) -> Option<ViewEvent> {
        let faction = self.selected_faction()?;
        if faction.presences.is_empty() {
            widgets::empty_state(ui, "No system presence data available.");
            return None;
        }
        let indices = self.sorted_system_indices(faction);
        let mut action: Option<SystemAction> = None;
        let mut sort: Option<char> = None;

        ui.horizontal(|ui| {
            widgets::hint(ui, "Click to select · double-click for history · right-click for more");
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let sel = self.selected_system.min(indices.len() - 1);
                if widgets::hud_button(ui, "History").on_hover_text("Influence history of the selected system (d)").clicked() {
                    action = Some(SystemAction::History(sel));
                }
                if widgets::hud_button(ui, "Copy system").on_hover_text("Copy the selected system's name to the clipboard (c)").clicked() {
                    action = Some(SystemAction::Copy(sel));
                }
            });
        });
        ui.add_space(4.0);

        let cols = system_columns(ui.available_width() - 22.0);
        let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
        for (k, (col, label, key)) in SORT_COLUMNS.iter().enumerate() {
            let (x0, x1) = cols[k];
            let r = Rect::from_min_max(pos2(head.left() + 6.0 + x0, head.top()), pos2(head.left() + 6.0 + x1, head.bottom()));
            let state = (*col == self.system_sort).then_some(self.system_sort_dir == SortDir::Asc);
            let layout = if k == 1 || k == 4 { egui::Layout::right_to_left(Align::Center) } else { egui::Layout::left_to_right(Align::Center) };
            let resp = ui
                .scope_builder(egui::UiBuilder::new().max_rect(r).layout(layout), |ui| widgets::th_sort(ui, label, state))
                .inner
                .on_hover_text(format!("Sort by {} ({key})", label.to_lowercase()));
            if resp.clicked() {
                sort = Some(*key);
            }
        }
        ui.painter().line_segment([pos2(head.left(), head.bottom()), pos2(head.right(), head.bottom())], Stroke::new(1.0, alpha(accent(), 70)));

        let sel_changed = changed(ui, "system-sel", self.selected_system);
        let selected_system = self.selected_system;
        let mut scroll = self.detail_scroll;
        line_scroll(ui, "factions-systems", &mut scroll, ROW_H, !sel_changed, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            for (row, &pi) in indices.iter().enumerate() {
                let pr = &faction.presences[pi];
                let selected = row == selected_system;
                let states = |list: &[String], pending: bool| {
                    if list.is_empty() {
                        "—".to_string()
                    } else {
                        list.iter().map(|s| crate::views::annotate_faction_state(s, pending)).collect::<Vec<_>>().join(", ")
                    }
                };
                let active = states(&pr.active_states, false);
                let pending = states(&pr.pending_states, true);
                let h = if pr.conflict.is_some() { ROW_H + CONFLICT_H } else { ROW_H };
                let mut cut = false;
                let resp = widgets::selectable_row(ui, selected, h, |ui| {
                    let r = ui.max_rect();
                    let p = ui.painter();
                    let line = Rect::from_min_size(r.min, vec2(r.width(), ROW_H));
                    let span = |k: usize| Rect::from_min_max(pos2(r.left() + cols[k].0, line.top()), pos2(r.left() + cols[k].1, line.bottom()));
                    let c = |col: Color32| widgets::row_text_color(selected, col);
                    let name_font = if selected { theme::body_semi(16.0) } else { theme::body(16.0) };
                    cut |= cell(p, span(0), pr.system_name.trim(), name_font, c(TEXT), Align::Min);

                    let pct = pr.influence * 100.0;
                    let ir = span(1);
                    let bar = Rect::from_min_size(pos2(ir.left(), ir.center().y - 4.0), vec2((ir.width() - 56.0).clamp(0.0, 64.0), 8.0));
                    widgets::seg_bar(p, bar, pct / 100.0, c(influence_color(pct)));
                    cell(p, Rect::from_min_max(pos2(bar.right() + 4.0, ir.top()), ir.max), &format!("{pct:.1}%"), theme::body_semi(15.0), c(influence_color(pct)), Align::Max);

                    cut |= cell(p, span(2), &active, theme::body(15.0), c(if pr.active_states.is_empty() { GREY } else { TEXT }), Align::Min);
                    cut |= cell(p, span(3), &pending, theme::body(15.0), c(if pr.pending_states.is_empty() { GREY } else { YELLOW }), Align::Min);
                    cell(p, span(4), &fmt_ts_short(pr.updated_at.as_ref()), theme::body(14.0), c(TEXT_DIM), Align::Max);

                    if let Some(cf) = &pr.conflict {
                        let (score_col, outcome) = match cf.our_won_days.cmp(&cf.opponent_won_days) {
                            std::cmp::Ordering::Greater => (GREEN, "Winning"),
                            std::cmp::Ordering::Less => (RED, "Losing"),
                            std::cmp::Ordering::Equal => (YELLOW, "Tied"),
                        };
                        let war = match cf.war_type.as_str() {
                            "Election" => "Election",
                            "CivilWar" => "Civil War",
                            _ => "War",
                        };
                        let fmt = |color: Color32, font: FontId| TextFormat { font_id: font, color: c(color), ..Default::default() };
                        let mut job = LayoutJob::default();
                        job.append(&format!("↳ {war} vs "), 0.0, fmt(TEXT_DIM, theme::body(14.5)));
                        job.append(&cf.opponent_name, 0.0, fmt(TEXT, theme::body_semi(14.5)));
                        job.append("   Score ", 0.0, fmt(TEXT_DIM, theme::body(14.5)));
                        job.append(&format!("{}:{}", cf.our_won_days, cf.opponent_won_days), 0.0, fmt(score_col, theme::body_semi(14.5)));
                        job.append(&format!("  ({outcome})"), 0.0, fmt(score_col, theme::body(14.5)));
                        job.wrap = TextWrapping::truncate_at_width((r.width() - 16.0).max(1.0));
                        let g = p.layout_job(job);
                        p.galley(pos2(r.left() + 14.0, line.bottom() + (CONFLICT_H - g.size().y) / 2.0 - 2.0), g, TEXT);
                    }
                });
                let resp = if cut {
                    resp.on_hover_text(format!("{}\nActive: {active}\nPending: {pending}", pr.system_name.trim()))
                } else {
                    resp
                };
                if sel_changed && selected {
                    resp.scroll_to_me(None);
                }
                if resp.double_clicked() {
                    action = Some(SystemAction::History(row));
                } else if resp.clicked() {
                    action = Some(SystemAction::Select(row));
                }
                resp.context_menu(|ui| {
                    if ui.button("Copy system name  (c)").clicked() {
                        action = Some(SystemAction::Copy(row));
                        ui.close();
                    }
                    if ui.button("Influence history  (d)").clicked() {
                        action = Some(SystemAction::History(row));
                        ui.close();
                    }
                });
            }
        });
        self.detail_scroll = scroll;

        let mut event = None;
        if let Some(key) = sort {
            self.focus = FocusArea::Detail;
            event = Some(self.handle_key(&KeyEvent::from(KeyCode::Char(key)), api));
        }
        match action {
            Some(SystemAction::Select(row)) => {
                self.focus = FocusArea::Detail;
                self.selected_system = row;
            }
            Some(SystemAction::Copy(row)) => {
                self.focus = FocusArea::Detail;
                self.selected_system = row;
                event = Some(self.handle_key(&KeyEvent::from(KeyCode::Char('c')), api));
            }
            Some(SystemAction::History(row)) => {
                self.selected_system = row;
                self.open_tab(DetailTab::History, api);
            }
            None => {}
        }
        event
    }

    fn history_tab(&mut self, ui: &mut Ui, api: &ApiClient) {
        let Some(faction) = self.selected_faction() else { return };
        let indices = self.sorted_system_indices(faction);
        let systems: Vec<String> = indices.iter().map(|&i| faction.presences[i].system_name.trim().to_string()).collect();
        let mut pick: Option<usize> = None;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("System").color(theme::CYAN).font(theme::body(15.0)));
            let current = self.history_system_name.trim();
            let shown = if current.is_empty() { "—" } else { current };
            egui::ComboBox::from_id_salt("faction-history-system")
                .selected_text(egui::RichText::new(shown).font(theme::body_semi(15.0)).color(TEXT))
                .width(260.0)
                .height(420.0)
                .show_ui(ui, |ui| {
                    for (i, name) in systems.iter().enumerate() {
                        if ui.selectable_label(name == current, name).clicked() {
                            pick = Some(i);
                        }
                    }
                })
                .response
                .on_hover_text("Show the influence history in another of the faction's systems");
            if !self.history_faction_name.is_empty() && !self.loading_history {
                widgets::hint(ui, "last 90 days");
            }
        });
        if let Some(i) = pick {
            self.focus = FocusArea::Detail;
            self.selected_system = i;
            self.fetch_history(api);
        }
        ui.add_space(6.0);

        if self.loading_history {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(14.0).color(accent()));
                ui.label(egui::RichText::new("Loading influence history…").color(TEXT_DIM).font(theme::body(15.0)));
            });
            return;
        }
        if self.history_data.len() < 2 {
            if let Some(e) = self.history_x_labels.first().filter(|l| l.starts_with("Error:")) {
                ui.add_space(12.0);
                ui.label(egui::RichText::new(e).color(RED).font(theme::body_semi(15.0)));
            } else {
                let msg = if self.history_faction_name.is_empty() {
                    "Pick a system above (or select one in Systems and press d) to load its influence history."
                } else if self.history_data.is_empty() {
                    "No history data recorded yet for this faction-system pair."
                } else {
                    "Not enough data points to draw a chart yet."
                };
                widgets::empty_state(ui, msg);
            }
            return;
        }

        self.history_stats(ui);
        ui.add_space(8.0);
        let legend = format!("{} in {}", self.history_faction_name, self.history_system_name.trim());
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(18.0, 12.0), Sense::hover());
            ui.painter().line_segment([r.left_center(), r.right_center()], Stroke::new(2.0, accent()));
            ui.label(egui::RichText::new(legend).color(TEXT_DIM).font(theme::body(14.0)));
        });
        let size = vec2(ui.available_width(), ui.available_height().max(160.0));
        let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
        self.paint_history_chart(ui.painter(), rect, resp.hover_pos());
    }

    /// Latest value, change over the period and range, above the chart.
    fn history_stats(&self, ui: &mut Ui) {
        let first = self.history_data.first().map(|p| p.1).unwrap_or(0.0);
        let last = self.history_data.last().map(|p| p.1).unwrap_or(0.0);
        let lo = self.history_data.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let hi = self.history_data.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        let delta = last - first;
        let delta_col = if delta > 0.05 { GREEN } else if delta < -0.05 { RED } else { TEXT_DIM };
        let days = self.history_time_bounds[1];
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 28.0;
            stat(ui, "Latest", &format!("{last:.1}%"), influence_color(last as f32));
            stat(ui, "Change", &format!("{delta:+.1} pts"), delta_col);
            stat(ui, "Low", &format!("{lo:.1}%"), TEXT);
            stat(ui, "High", &format!("{hi:.1}%"), TEXT);
            stat(ui, "Period", &format!("{days:.1} days"), TEXT);
            stat(ui, "Samples", &self.history_data.len().to_string(), TEXT);
        });
    }

    /// The influence line chart: grid, axis labels, filled line and a hover read-out.
    fn paint_history_chart(&self, painter: &Painter, rect: Rect, hover: Option<egui::Pos2>) {
        let p = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        let plot = Rect::from_min_max(pos2(rect.left() + 52.0, rect.top() + 22.0), pos2(rect.right() - 14.0, rect.bottom() - 30.0));
        if plot.width() < 40.0 || plot.height() < 40.0 {
            return;
        }
        p.rect_filled(plot, 0.0, alpha(BG, 160));
        let [x0, x1] = self.history_time_bounds;
        let [y0, y1] = self.history_y_bounds;
        let (xs, ys) = ((x1 - x0).max(1e-9), (y1 - y0).max(1e-9));
        let to_screen = |x: f64, y: f64| {
            pos2(
                plot.left() + ((x - x0) / xs) as f32 * plot.width(),
                plot.bottom() - ((y - y0) / ys) as f32 * plot.height(),
            )
        };
        let grid = Stroke::new(1.0, alpha(accent(), 26));
        let label_font = theme::body(13.5);

        let step = nice_step((y1 - y0) / (plot.height() / 48.0).max(2.0) as f64);
        let mut v = (y0 / step).ceil() * step;
        while v <= y1 + 1e-9 {
            let y = to_screen(x0, v).y;
            p.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], grid);
            let text = if step < 1.0 { format!("{v:.1}%") } else { format!("{v:.0}%") };
            widgets::txt(&p, pos2(plot.left() - 8.0, y), Align2::RIGHT_CENTER, &text, label_font.clone(), TEXT_DIM, 0.0);
            v += step;
        }

        let start = self.history_start_ts;
        let span_secs = x1 * 86400.0;
        let max_ticks = (plot.width() / 84.0).max(2.0) as f64;
        let (tick_secs, fmt) = [1.0, 3.0, 6.0, 12.0]
            .iter()
            .map(|h| (h * 3600.0, "%H:%M"))
            .chain([1.0, 2.0, 3.0, 7.0, 14.0, 30.0].iter().map(|d| (d * 86400.0, "%b %d")))
            .find(|(s, _)| span_secs / s <= max_ticks)
            .unwrap_or((30.0 * 86400.0, "%b %d"));
        let mut t = ((start as f64 / tick_secs).ceil() * tick_secs) as i64;
        while (t - start) as f64 <= span_secs {
            let x = to_screen((t - start) as f64 / 86400.0, y0).x;
            p.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], grid);
            if let Some(dt) = chrono::DateTime::from_timestamp(t, 0) {
                let fmt = if t % 86400 == 0 { "%b %d" } else { fmt };
                widgets::txt(&p, pos2(x, plot.bottom() + 6.0), Align2::CENTER_TOP, &dt.format(fmt).to_string(), label_font.clone(), TEXT_DIM, 0.0);
            }
            t += tick_secs as i64;
        }
        widgets::txt(&p, pos2(rect.left(), rect.top() + 6.0), Align2::LEFT_CENTER, "INFLUENCE %", theme::head(8.0), accent_dim(), 1.2);
        widgets::txt(&p, pos2(plot.right(), rect.top() + 6.0), Align2::RIGHT_CENTER, "DATE (UTC)", theme::head(8.0), accent_dim(), 1.2);

        let pts: Vec<egui::Pos2> = self.history_data.iter().map(|&(x, y)| to_screen(x, y)).collect();
        let acc = accent();
        let mut mesh = egui::Mesh::default();
        for (i, pt) in pts.iter().enumerate() {
            mesh.colored_vertex(*pt, alpha(acc, 70));
            mesh.colored_vertex(pos2(pt.x, plot.bottom()), alpha(acc, 4));
            if i > 0 {
                let k = (i as u32) * 2;
                mesh.add_triangle(k - 2, k - 1, k);
                mesh.add_triangle(k - 1, k + 1, k);
            }
        }
        p.add(Shape::mesh(mesh));
        p.add(Shape::line(pts.clone(), Stroke::new(2.0, acc)));
        if pts.len() <= 120 {
            for pt in &pts {
                p.circle_filled(*pt, 2.5, accent_hi());
            }
        }
        p.rect_stroke(plot, 0.0, Stroke::new(1.0, alpha(acc, 70)), egui::StrokeKind::Inside);

        let Some(hp) = hover.filter(|h| plot.expand(4.0).contains(*h)) else { return };
        let Some((i, pt)) = pts.iter().enumerate().min_by(|a, b| (a.1.x - hp.x).abs().total_cmp(&(b.1.x - hp.x).abs())) else { return };
        p.line_segment([pos2(pt.x, plot.top()), pos2(pt.x, plot.bottom())], Stroke::new(1.0, alpha(TEXT, 90)));
        p.circle(*pt, 4.5, BG, Stroke::new(2.0, accent_hi()));
        let (x, y) = self.history_data[i];
        let when = chrono::DateTime::from_timestamp(start + (x * 86400.0) as i64, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_default();
        let g1 = p.layout_no_wrap(format!("{y:.1}%"), theme::body_semi(17.0), influence_color(y as f32));
        let g2 = p.layout_no_wrap(when, theme::body(13.5), TEXT_DIM);
        let size = vec2(g1.size().x.max(g2.size().x) + 16.0, g1.size().y + g2.size().y + 10.0);
        let mut at = pt.to_vec2() + vec2(12.0, -size.y - 8.0);
        if at.x + size.x > plot.right() {
            at.x = pt.x - size.x - 12.0;
        }
        at.y = at.y.max(plot.top() + 2.0);
        let tip = Rect::from_min_size(at.to_pos2(), size);
        p.rect(tip, 0.0, PANEL, Stroke::new(1.0, acc), egui::StrokeKind::Inside);
        let h1 = g1.size().y;
        p.galley(tip.min + vec2(8.0, 5.0), g1, TEXT);
        p.galley(tip.min + vec2(8.0, 5.0 + h1), g2, TEXT);
    }
}

// ---------------------------------------------------------------- helpers

/// Brightens the frame of the panel that has keyboard focus (same chamfer as `hud_panel`).
fn focus_outline(p: &Painter, r: Rect) {
    let c = 12.0;
    let pts = vec![
        r.left_top(),
        pos2(r.right() - c, r.top()),
        pos2(r.right(), r.top() + c),
        r.right_bottom(),
        pos2(r.left() + c, r.bottom()),
        pos2(r.left(), r.bottom() - c),
    ];
    p.add(Shape::closed_line(pts, Stroke::new(1.5, accent())));
}

/// Whether `value` differs from what it was on the previous frame (keyed by `key`).
fn changed(ui: &Ui, key: &str, value: usize) -> bool {
    let id = egui::Id::new(("factions-prev", key));
    ui.ctx().data_mut(|d| {
        let prev = d.get_temp::<usize>(id);
        d.insert_temp(id, value);
        prev != Some(value)
    })
}

/// A vertical scroll area tied to a line counter the key handler moves (w/s, PgUp/PgDn):
/// when `line` changed since the last frame (and `jump` allows) it scrolls to `line * step`;
/// mouse scrolling writes the new position back, so keys continue from where the wheel left.
fn line_scroll<R>(ui: &mut Ui, salt: &str, line: &mut usize, step: f32, jump: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    let id = egui::Id::new(("factions-line", salt));
    let prev = ui.ctx().data(|d| d.get_temp::<usize>(id));
    let mut area = egui::ScrollArea::vertical().id_salt(salt).auto_shrink([false, false]);
    if jump && prev != Some(*line) {
        area = area.vertical_scroll_offset(*line as f32 * step);
    }
    let out = area.show(ui, add);
    let max = (out.content_size.y - out.inner_rect.height()).max(0.0);
    *line = (out.state.offset.y.clamp(0.0, max) / step).round() as usize;
    ui.ctx().data_mut(|d| d.insert_temp(id, *line));
    out.inner
}

/// Paints `text` on one line inside `rect`, cut with an ellipsis when it doesn't fit.
/// Returns whether it was cut.
fn cell(p: &Painter, rect: Rect, text: &str, font: FontId, color: Color32, align: Align) -> bool {
    let mut job = LayoutJob::single_section(text.to_owned(), TextFormat { font_id: font, color, ..Default::default() });
    job.wrap = TextWrapping::truncate_at_width(rect.width().max(1.0));
    let g = p.layout_job(job);
    let x = match align {
        Align::Min => rect.left(),
        Align::Center => rect.center().x - g.size().x / 2.0,
        Align::Max => rect.right() - g.size().x,
    };
    let cut = g.elided;
    p.galley(pos2(x, rect.center().y - g.size().y / 2.0), g, color);
    cut
}

/// Column spans (left, right) for System, Influence, Active state, Pending, Updated.
fn system_columns(width: f32) -> [(f32, f32); 5] {
    let (inf_w, upd_w, gap) = if width < 700.0 { (96.0, 80.0, 8.0) } else { (122.0, 84.0, 12.0) };
    let flex = (width - inf_w - upd_w - 4.0 * gap).max(150.0);
    let (name_w, active_w) = (flex * 0.42, flex * 0.33);
    let pending_w = flex - name_w - active_w;
    let mut x = 0.0;
    let mut out = [(0.0, 0.0); 5];
    for (i, w) in [name_w, inf_w, active_w, pending_w, upd_w].into_iter().enumerate() {
        out[i] = (x, x + w);
        x += w + gap;
    }
    out
}

/// Prose paragraph: dim, wrapped, comfortable size.
fn prose(ui: &mut Ui, text: &str) {
    ui.add(egui::Label::new(egui::RichText::new(text).color(TEXT_DIM).font(theme::body(16.0))).wrap());
}

/// A small caption over a large value, for the history summary row.
fn stat(ui: &mut Ui, caption: &str, value: &str, color: Color32) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        widgets::caption(ui, caption, accent_dim());
        ui.label(egui::RichText::new(value).font(theme::body_semi(20.0)).color(color));
    });
}

/// Influence colours: red below 15 %, yellow below 40 %, else green.
fn influence_color(pct: f32) -> Color32 {
    if pct < 15.0 {
        RED
    } else if pct < 40.0 {
        YELLOW
    } else {
        GREEN
    }
}

/// Badge colour per allegiance.
fn allegiance_color(alleg: &str) -> Color32 {
    match alleg {
        "Federation" => theme::BLUE,
        "Empire" => YELLOW,
        "Alliance" => GREEN,
        "Independent" => TEXT_DIM,
        _ => GREY,
    }
}

/// A 1/2/5 × 10ⁿ step at least `raw` wide, for axis ticks.
fn nice_step(raw: f64) -> f64 {
    let raw = raw.max(1e-6);
    let mag = 10f64.powf(raw.log10().floor());
    let f = raw / mag;
    let nice = if f <= 1.0 {
        1.0
    } else if f <= 2.0 {
        2.0
    } else if f <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * mag
}
