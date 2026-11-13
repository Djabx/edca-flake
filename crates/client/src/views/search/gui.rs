//! Desktop-window drawing for [`SearchView`]: the sub-tab strip and shared field over
//! All's groups, a one-kind list, or the embedded Stations / Factions view.

use eframe::egui::{self, pos2, vec2, Align, Id, Layout, Rect, RichText, Ui};

use super::{SearchTab, SearchView, ALL_PER_KIND, TAB_ROWS};
use crate::api_client::ApiClient;
use crate::gui::theme::{self, accent_hi, GREY, TEXT, TEXT_DIM};
use crate::gui::widgets;
use crate::journal_reader::JournalData;
use crate::todo::TodoList;
use crate::views::{FactionsView, StationsView, ViewEvent};

const FIELD_ID: &str = "search-field";
const ROW: f32 = 34.0;

/// What a click on a row asks for; applied once drawing is done.
enum Pick {
    Tab(SearchTab),
    System(i64, String),
    Station(i64, String),
    Faction(String),
    Commodity(String),
    Module(String),
    /// Row `i` of a one-kind tab's list.
    Row(usize),
    Submit,
}

impl SearchView {
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        api: &ApiClient,
        journal: &JournalData,
        todo: &mut TodoList,
        stations: &mut StationsView,
        factions: &mut FactionsView,
    ) -> ViewEvent {
        stations.embedded = true;
        factions.embedded = true;
        let mut picks = Vec::new();
        self.top_bar(ui, &mut picks);
        ui.add_space(8.0);

        let mut event = ViewEvent::None;
        match self.tab {
            SearchTab::All => self.all_ui(ui, &mut picks),
            SearchTab::Systems => self.systems_ui(ui, &mut picks),
            SearchTab::Stations => event = stations.ui(ui, api, journal, todo),
            SearchTab::Factions => event = factions.ui(ui, api),
            SearchTab::Commodities => self.commodities_ui(ui, journal, &mut picks),
            SearchTab::Modules => self.modules_ui(ui, journal, &mut picks),
        }
        if matches!(event, ViewEvent::FocusSearch) {
            self.focus_field = true;
            event = ViewEvent::Consumed;
        }

        for pick in picks {
            let ev = match pick {
                Pick::Tab(tab) => {
                    self.set_tab(tab, api, journal, stations, factions);
                    ViewEvent::Consumed
                }
                Pick::System(address, name) => ViewEvent::OpenSystem { address, name },
                Pick::Station(market_id, name) => {
                    self.open_station(market_id, &name, api, journal, stations, factions);
                    ViewEvent::Consumed
                }
                Pick::Faction(name) => {
                    self.open_faction_tab(&name, api, journal, stations, factions);
                    ViewEvent::Consumed
                }
                Pick::Commodity(name) => {
                    self.open_commodity(&name, api, journal, stations, factions);
                    ViewEvent::Consumed
                }
                Pick::Module(name) => {
                    self.open_module(&name, api, journal, stations, factions);
                    ViewEvent::Consumed
                }
                Pick::Row(i) => self.open_row(i, api, journal, stations, factions),
                Pick::Submit => {
                    self.submit(api, journal, stations, factions);
                    ViewEvent::Consumed
                }
            };
            if matches!(event, ViewEvent::None | ViewEvent::Consumed) {
                event = ev;
            }
        }
        event
    }

    /// The sub-tab strip, the field and the status line.
    fn top_bar(&mut self, ui: &mut Ui, picks: &mut Vec<Pick>) {
        let labels: Vec<&str> = SearchTab::TABS.iter().map(|t| t.label()).collect();
        if let Some(i) = widgets::tab_strip(ui, &labels, self.tab.index()) {
            picks.push(Pick::Tab(SearchTab::TABS[i]));
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 96.0).max(120.0);
            let id = Id::new(FIELD_ID);
            let replaced = self.query != self.field_text;
            let resp = ui.add_sized(
                vec2(w, 30.0),
                egui::TextEdit::singleline(&mut self.query)
                    .id(id)
                    .hint_text(self.tab.placeholder())
                    .font(theme::body(17.0))
                    .margin(vec2(8.0, 5.0)),
            );
            if std::mem::take(&mut self.focus_field) {
                resp.request_focus();
            }
            if replaced {
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                    let end = egui::text::CCursor::new(self.query.chars().count());
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                    state.store(ui.ctx(), id);
                }
            }
            if resp.changed() {
                self.edited_now();
                // Typing a new name drops the picked commodity / module's list.
                if self.commodity.as_deref() != Some(self.query.as_str()) {
                    self.commodity = None;
                    self.commodity_rows.clear();
                }
                if self.module.as_deref() != Some(self.query.as_str()) {
                    self.module = None;
                    self.module_rows.clear();
                }
            }
            if widgets::submitted(ui, &resp) {
                picks.push(Pick::Submit);
                resp.request_focus();
            }
            if widgets::hud_button(ui, "Search").on_hover_text("Search  (Enter)").clicked() {
                picks.push(Pick::Submit);
            }
            self.field_text = self.query.clone();
        });
        let note = if self.loading {
            "Searching…".to_string()
        } else if !self.status.is_empty() {
            self.status.clone()
        } else if self.tab.live() {
            "Results appear as you type · / to search · [ ] to change kind".to_string()
        } else {
            "Enter to search · / to search · [ ] to change kind".to_string()
        };
        ui.add_space(2.0);
        ui.label(RichText::new(note).font(theme::body(14.0)).color(GREY));
    }

    // ------------------------------------------------------------------- All

    fn all_ui(&mut self, ui: &mut Ui, picks: &mut Vec<Pick>) {
        let full = ui.available_rect_before_wrap();
        if self.query.trim().is_empty() {
            let Some(o) = self.overview.clone() else {
                widgets::empty_state(ui, "Loading what commanders visit most…");
                return;
            };
            let rects = columns(full, 3);
            widgets::hud_panel_at(ui, rects[0], "Popular systems", "most visited", |ui| {
                for s in o.systems.iter().take(12) {
                    if row(ui, &s.name, &format!("{} visits", group(s.hits))) {
                        picks.push(Pick::System(s.system_address, s.name.clone()));
                    }
                }
            });
            widgets::hud_panel_at(ui, rects[1], "Popular stations", "most docked at", |ui| {
                for s in o.stations.iter().take(12) {
                    if row(ui, &s.name, &s.system_name) {
                        picks.push(Pick::Station(s.market_id, s.name.clone()));
                    }
                }
            });
            widgets::hud_panel_at(ui, rects[2], "Widest factions", "systems present in", |ui| {
                for f in o.factions.iter().take(12) {
                    if row(ui, &f.name, &format!("{} systems", group(f.system_count))) {
                        picks.push(Pick::Faction(f.name.clone()));
                    }
                }
            });
            return;
        }

        let commodities = self.commodity_matches(ALL_PER_KIND);
        let modules: Vec<(String, i64)> = self.module_matches(ALL_PER_KIND).iter().map(|m| (m.name.clone(), m.market_count)).collect();
        let m = &self.matches;
        let fresh = self.searched == self.query.trim();
        if fresh
            && !self.loading
            && m.systems.is_empty()
            && m.stations.is_empty()
            && m.factions.is_empty()
            && commodities.is_empty()
            && modules.is_empty()
        {
            widgets::empty_state(ui, &format!("Nothing matches '{}'.", self.query.trim()));
            return;
        }
        let top = Rect::from_min_max(full.min, pos2(full.right(), full.top() + full.height() * 0.5 - 4.0));
        let bottom = Rect::from_min_max(pos2(full.left(), top.bottom() + 8.0), full.max);
        let up = columns(top, 3);
        let down = columns(bottom, 2);
        let shown = |n: usize| if n == 0 { String::new() } else { format!("{n} shown") };

        widgets::hud_panel_at(ui, up[0], "Systems", &shown(m.systems.len()), |ui| {
            for s in &m.systems {
                if row(ui, &s.name, "") {
                    picks.push(Pick::System(s.system_address, s.name.clone()));
                }
            }
            group_footer(ui, m.systems.is_empty(), SearchTab::Systems, picks);
        });
        widgets::hud_panel_at(ui, up[1], "Stations", &shown(m.stations.len()), |ui| {
            for s in &m.stations {
                if row(ui, &s.name, &s.system_name) {
                    picks.push(Pick::Station(s.market_id, s.name.clone()));
                }
            }
            group_footer(ui, m.stations.is_empty(), SearchTab::Stations, picks);
        });
        widgets::hud_panel_at(ui, up[2], "Factions", &shown(m.factions.len()), |ui| {
            for f in &m.factions {
                let sub = format!("{} · {} systems", f.allegiance.as_deref().unwrap_or("Independent"), f.system_count);
                if row(ui, &f.name, &sub) {
                    picks.push(Pick::Faction(f.name.clone()));
                }
            }
            group_footer(ui, m.factions.is_empty(), SearchTab::Factions, picks);
        });
        widgets::hud_panel_at(ui, down[0], "Commodities", "nearest markets", |ui| {
            for c in &commodities {
                if row(ui, c, "") {
                    picks.push(Pick::Commodity(c.to_string()));
                }
            }
            group_footer(ui, commodities.is_empty(), SearchTab::Commodities, picks);
        });
        widgets::hud_panel_at(ui, down[1], "Modules", "nearest sellers", |ui| {
            for (name, count) in &modules {
                if row(ui, name, &format!("{} stations", group(*count))) {
                    picks.push(Pick::Module(name.clone()));
                }
            }
            group_footer(ui, modules.is_empty(), SearchTab::Modules, picks);
        });
    }

    // --------------------------------------------------------------- Systems

    fn systems_ui(&mut self, ui: &mut Ui, picks: &mut Vec<Pick>) {
        let full = ui.available_rect_before_wrap();
        let caption = if self.systems.is_empty() { String::new() } else { format!("{} found · opens on the galaxy map", self.systems.len()) };
        widgets::hud_panel_at(ui, full, "Systems", &caption, |ui| {
            if self.systems.is_empty() {
                widgets::empty_state(ui, "Type a system name. Picking one shows it on the galaxy map.");
                return;
            }
            egui::ScrollArea::vertical().id_salt("search-list-1").auto_shrink([false, false]).show(ui, |ui| {
                for (i, s) in self.systems.iter().enumerate() {
                    let sub = match (s.allegiance.as_deref(), s.population) {
                        (Some(a), Some(p)) if p > 0 => format!("{a} · {} people", group(p)),
                        (Some(a), _) => a.to_string(),
                        (None, Some(p)) if p > 0 => format!("{} people", group(p)),
                        _ => "Unpopulated".to_string(),
                    };
                    if list_row(ui, i == self.selected, &s.name, &sub) {
                        picks.push(Pick::Row(i));
                    }
                }
            });
        });
    }

    // ----------------------------------------------------------- Commodities

    fn commodities_ui(&mut self, ui: &mut Ui, journal: &JournalData, picks: &mut Vec<Pick>) {
        let full = ui.available_rect_before_wrap();
        let [left, right] = split(full);
        let matches = self.commodity_matches(TAB_ROWS);
        let picking = self.commodity_rows.is_empty();
        widgets::hud_panel_at(ui, left, "Commodities", "", |ui| {
            if matches.is_empty() {
                widgets::empty_state(ui, "Type a commodity name.");
                return;
            }
            egui::ScrollArea::vertical().id_salt("search-list-2").auto_shrink([false, false]).show(ui, |ui| {
                for (i, c) in matches.iter().enumerate() {
                    let on = self.commodity.as_deref() == Some(*c) || (picking && i == self.selected);
                    if list_row(ui, on, c, "") {
                        picks.push(Pick::Commodity(c.to_string()));
                    }
                }
            });
        });
        let from = journal.current_system.as_ref().map(|s| s.name.clone()).unwrap_or_default();
        let title = match &self.commodity {
            Some(c) => format!("Nearest markets selling {c}"),
            None => "Nearest markets".to_string(),
        };
        let caption = if from.is_empty() { String::new() } else { format!("from {from} · no fleet carriers") };
        widgets::hud_panel_at(ui, right, &title, &caption, |ui| {
            if self.commodity_rows.is_empty() {
                widgets::empty_state(ui, "Pick a commodity to see the markets nearest you that sell it.");
                return;
            }
            egui::ScrollArea::vertical().id_salt("search-list-3").auto_shrink([false, false]).show(ui, |ui| {
                for (i, r) in self.commodity_rows.iter().enumerate() {
                    let sub = format!(
                        "{} · {:.1} ly · {} CR · {} in stock · pad {}",
                        r.system_name,
                        r.distance_ly,
                        group(r.buy_price as i64),
                        group(r.stock as i64),
                        pad(r.has_large_pad, r.has_medium_pad)
                    );
                    if list_row(ui, !picking && i == self.selected, &r.station_name, &sub) {
                        picks.push(Pick::Station(r.market_id, r.station_name.clone()));
                    }
                }
            });
        });
    }

    // --------------------------------------------------------------- Modules

    fn modules_ui(&mut self, ui: &mut Ui, journal: &JournalData, picks: &mut Vec<Pick>) {
        let full = ui.available_rect_before_wrap();
        let [left, right] = split(full);
        let matches: Vec<(String, i64)> = self.module_matches(TAB_ROWS).iter().map(|m| (m.name.clone(), m.market_count)).collect();
        let picking = self.module_rows.is_empty();
        let caption = if self.modules.is_empty() { "loading the list…".to_string() } else { format!("{} known", group(self.modules.len() as i64)) };
        widgets::hud_panel_at(ui, left, "Modules", &caption, |ui| {
            if matches.is_empty() {
                widgets::empty_state(ui, "Type part of a module's name, e.g. railgun, fsd or shield.");
                return;
            }
            egui::ScrollArea::vertical().id_salt("search-list-4").auto_shrink([false, false]).show(ui, |ui| {
                for (i, (name, count)) in matches.iter().enumerate() {
                    let on = self.module.as_deref() == Some(name.as_str()) || (picking && i == self.selected);
                    if list_row(ui, on, name, &format!("{} stations", group(*count))) {
                        picks.push(Pick::Module(name.clone()));
                    }
                }
            });
        });
        let from = journal.current_system.as_ref().map(|s| s.name.clone()).unwrap_or_default();
        let title = match &self.module {
            Some(m) => format!("Nearest stations selling {m}"),
            None => "Nearest stations".to_string(),
        };
        let caption = if from.is_empty() { String::new() } else { format!("from {from}") };
        widgets::hud_panel_at(ui, right, &title, &caption, |ui| {
            if self.module_rows.is_empty() {
                widgets::empty_state(ui, "Pick a module to see the stations nearest you that sell it.");
                return;
            }
            egui::ScrollArea::vertical().id_salt("search-list-5").auto_shrink([false, false]).show(ui, |ui| {
                for (i, r) in self.module_rows.iter().enumerate() {
                    let price = if r.cost > 0 { format!("{} CR", group(r.cost)) } else { "price unknown".to_string() };
                    let sub = format!("{} · {:.1} ly · {price} · pad {}", r.system_name, r.distance_ly, pad(r.has_large_pad, r.has_medium_pad));
                    if list_row(ui, !picking && i == self.selected, &r.station_name, &sub) {
                        picks.push(Pick::Station(r.market_id, r.station_name.clone()));
                    }
                }
            });
        });
    }
}

/// `n` equal columns across `r`.
fn columns(r: Rect, n: usize) -> Vec<Rect> {
    let gap = 10.0;
    let w = (r.width() - gap * (n as f32 - 1.0)) / n as f32;
    (0..n)
        .map(|i| Rect::from_min_size(pos2(r.left() + i as f32 * (w + gap), r.top()), vec2(w, r.height())))
        .collect()
}

/// A narrow picking column and a wide results one.
fn split(r: Rect) -> [Rect; 2] {
    let w = (r.width() * 0.34).clamp(260.0, 440.0);
    [
        Rect::from_min_size(r.min, vec2(w, r.height())),
        Rect::from_min_max(pos2(r.left() + w + 10.0, r.top()), r.max),
    ]
}

/// A clickable result: name, with a dim note after it. True when clicked.
fn row(ui: &mut Ui, name: &str, note: &str) -> bool {
    widgets::selectable_row(ui, false, 26.0, |ui| {
        // The note first, from the right, so a long name is cut short instead of running into it.
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if !note.is_empty() {
                ui.label(RichText::new(note).font(theme::body(13.5)).color(TEXT_DIM));
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add(egui::Label::new(RichText::new(name).font(theme::body_semi(15.5)).color(TEXT)).truncate());
            });
        });
    })
    .clicked()
}

/// A two-line list row, highlighted when `on`. True when clicked.
fn list_row(ui: &mut Ui, on: bool, name: &str, sub: &str) -> bool {
    widgets::selectable_row(ui, on, ROW + if sub.is_empty() { -8.0 } else { 6.0 }, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add(egui::Label::new(RichText::new(name).font(theme::body_semi(15.5)).color(widgets::row_text_color(on, TEXT))).truncate());
            if !sub.is_empty() {
                ui.add(egui::Label::new(RichText::new(sub).font(theme::body(13.5)).color(widgets::row_text_color(on, TEXT_DIM))).truncate());
            }
        });
    })
    .clicked()
}

/// Under a group on All: "none" or a link to the group's own tab.
fn group_footer(ui: &mut Ui, empty: bool, tab: SearchTab, picks: &mut Vec<Pick>) {
    ui.add_space(4.0);
    if empty {
        ui.label(RichText::new("No matches").font(theme::body(14.0)).color(GREY));
    } else if ui
        .link(RichText::new(format!("All {} →", tab.label().to_lowercase())).font(theme::body(14.0)).color(accent_hi()))
        .clicked()
    {
        picks.push(Pick::Tab(tab));
    }
}

fn pad(large: bool, medium: bool) -> &'static str {
    match (large, medium) {
        (true, _) => "L",
        (false, true) => "M",
        _ => "S",
    }
}

fn group(n: i64) -> String {
    edcas_common::format::group_thousands(n)
}
