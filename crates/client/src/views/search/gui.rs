//! Desktop-window drawing for [`SearchView`]: the sub-tab strip and shared field over
//! All's groups, a one-kind list, the embedded Stations / Factions view, or Stars' and
//! Bodies' picker and results table.

use eframe::egui::{self, pos2, vec2, Align, Color32, Id, Layout, Rect, RichText, Sense, Ui};
use egui_extras::{Column, TableBuilder};

use super::{known_bodies_line, known_line, BodyChoice, SearchTab, SearchView, StarChoice, Tri, ALL_PER_KIND, TAB_ROWS};
use crate::api_client::ApiClient;
use crate::gui::theme::{self, accent, accent_hi, BG, GREY, TEXT, TEXT_DIM};
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
    /// Stars: search for this kind.
    Star(StarChoice),
    /// Stars: search again (the options changed).
    StarSearch,
    /// Stars: highlight result `i`.
    StarSelect(usize),
    /// Stars: show result `i`'s system on the galaxy map.
    StarOpen(usize),
    /// Stars: copy result `i`'s system name.
    StarCopy(usize),
    /// Bodies: search for this kind.
    Body(BodyChoice),
    /// Bodies: search again (a filter changed).
    BodySearch,
    /// Bodies: highlight result `i`.
    BodySelect(usize),
    /// Bodies: show result `i`'s system on the galaxy map.
    BodyOpen(usize),
    /// Bodies: copy result `i`'s system name.
    BodyCopy(usize),
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
            SearchTab::Stars => self.stars_ui(ui, &mut picks),
            SearchTab::Bodies => self.bodies_ui(ui, &mut picks),
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
                Pick::Star(choice) => {
                    self.pick_star(choice, api, journal);
                    ViewEvent::Consumed
                }
                Pick::StarSearch => {
                    if self.star_choice.is_some() {
                        self.search_stars(api, journal);
                    }
                    ViewEvent::Consumed
                }
                Pick::StarSelect(i) => {
                    self.selected = i;
                    ViewEvent::Consumed
                }
                Pick::StarOpen(i) => self.open_star_row(i),
                Pick::StarCopy(i) => {
                    self.selected = i;
                    self.copy_star_system();
                    ViewEvent::Consumed
                }
                Pick::Body(choice) => {
                    self.pick_body(choice, api, journal);
                    ViewEvent::Consumed
                }
                Pick::BodySearch => {
                    if self.body_choice.is_some() {
                        self.search_bodies(api, journal);
                    }
                    ViewEvent::Consumed
                }
                Pick::BodySelect(i) => {
                    self.selected = i;
                    ViewEvent::Consumed
                }
                Pick::BodyOpen(i) => self.open_body_row(i),
                Pick::BodyCopy(i) => {
                    self.selected = i;
                    self.copy_body_system();
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
            let id = Id::unique(FIELD_ID);
            // On Stars and Bodies the field is the reference system, kept apart from the query.
            let reference = self.tab.takes_reference();
            let text = match self.tab {
                SearchTab::Stars => &mut self.star_system,
                SearchTab::Bodies => &mut self.body_system,
                _ => &mut self.query,
            };
            let replaced = *text != self.field_text;
            let resp = ui.add_sized(
                vec2(w, 30.0),
                egui::TextEdit::singleline(text)
                    .id(id)
                    .hint_text(self.tab.placeholder())
                    .font(theme::body(17.0))
                    .margin(vec2(8.0, 5.0)),
            );
            if std::mem::take(&mut self.focus_field) {
                resp.request_focus();
            }
            let shown = text.clone();
            if replaced {
                let len = shown.chars().count();
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                    let end = egui::text::CCursor::new(len);
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                    state.store(ui.ctx(), id);
                }
            }
            if resp.changed() && !reference {
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
            self.field_text = shown;
        });
        let note = if self.loading {
            "Searching…".to_string()
        } else if !self.status.is_empty() {
            self.status.clone()
        } else if self.tab == SearchTab::Stars {
            "Pick a kind of star · Enter searches from the system in the field · / to edit it · [ ] to change kind".to_string()
        } else if self.tab == SearchTab::Bodies {
            "Pick a kind of planet · Enter searches from the system in the field · / to edit it · [ ] to change kind".to_string()
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

impl SearchView {
    // ----------------------------------------------------------------- Stars

    fn stars_ui(&mut self, ui: &mut Ui, picks: &mut Vec<Pick>) {
        let full = ui.available_rect_before_wrap();
        let [left, right] = split(full);
        let choices = self.star_choices();
        let picking = self.stars.as_ref().is_none_or(|s| s.results.is_empty());
        let caption = if self.star_catalog.is_some() { String::new() } else { "groups only".to_string() };
        widgets::hud_panel_at(ui, left, "Kind of star", &caption, |ui| {
            egui::ScrollArea::vertical().id_salt("search-stars-kinds").auto_shrink([false, false]).show(ui, |ui| {
                let mut types_started = false;
                for (i, (choice, count)) in choices.iter().enumerate() {
                    if matches!(choice, StarChoice::Type(_)) && !types_started {
                        types_started = true;
                        ui.add_space(6.0);
                        widgets::section(ui, "Single types", None);
                    }
                    let on = self.star_choice.as_ref() == Some(choice) || (picking && self.star_choice.is_none() && i == self.star_pick);
                    let note = count.map(|n| format!("{} known", group(n))).unwrap_or_default();
                    if kind_row(ui, on, &choice.label(), &note) {
                        picks.push(Pick::Star(choice.clone()));
                    }
                }
            });
        });

        let title = match &self.star_choice {
            Some(c) => format!("Nearest {} stars", c.label()),
            None => "Nearest stars".to_string(),
        };
        let from = self.stars.as_ref().and_then(|s| s.reference.name.clone()).unwrap_or_else(|| self.star_system.trim().to_string());
        let caption = if from.is_empty() { String::new() } else { format!("from {from}") };
        widgets::hud_panel_at(ui, right, &title, &caption, |ui| {
            self.star_options(ui, picks);
            ui.add_space(6.0);
            match &self.stars {
                Some(s) => {
                    ui.label(RichText::new(known_line(s.known_total)).font(theme::body(14.5)).color(TEXT_DIM))
                        .on_hover_text("EDCAS only knows the stars commanders have scanned: the real nearest one may not be listed yet.");
                    ui.add_space(4.0);
                }
                None if self.star_choice.is_none() => {
                    widgets::empty_state(ui, "Pick a kind of star on the left to see the nearest ones EDCAS knows.");
                    return;
                }
                None => {}
            }
            if self.loading {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Spinner::new().size(22.0).color(accent()));
                    widgets::hint(ui, "Searching…");
                });
            } else if !picking {
                self.stars_table(ui, picks);
            } else if !self.status.is_empty() {
                widgets::empty_state(ui, &self.status);
            }
        });
    }

    /// "Main star only", max distance, and Search / Copy system.
    fn star_options(&mut self, ui: &mut Ui, picks: &mut Vec<Pick>) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            let main = ui
                .checkbox(&mut self.star_main_only, RichText::new("Main star only").font(theme::body(15.0)).color(TEXT))
                .on_hover_text("Only a system's arrival star: one to scoop or boost at as you drop in  (m)");
            if main.changed() {
                picks.push(Pick::StarSearch);
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Max distance").font(theme::body(15.0)).color(theme::CYAN));
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.star_max_distance)
                    .id(Id::unique("search-stars-max-distance"))
                    .hint_text(RichText::new("any").color(GREY))
                    .font(theme::body(15.0))
                    .desired_width(70.0)
                    .margin(vec2(6.0, 3.0)),
            );
            ui.label(RichText::new("ly").font(theme::body(15.0)).color(TEXT_DIM));
            if widgets::submitted(ui, &resp) {
                picks.push(Pick::StarSearch);
            }
            let can_search = self.star_choice.is_some() && !self.loading;
            if widgets::hud_button_enabled(ui, "Search", can_search).on_hover_text("Search again  (Enter)").clicked() {
                picks.push(Pick::StarSearch);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let any = self.stars.as_ref().is_some_and(|s| !s.results.is_empty());
                if widgets::hud_button_enabled(ui, "Copy system", any).on_hover_text("Copy the selected star's system name  (c)").clicked() {
                    picks.push(Pick::StarCopy(self.selected));
                }
            });
        });
    }

    /// The results: distance, system, star, type and class, main star, scan date.
    fn stars_table(&mut self, ui: &mut Ui, picks: &mut Vec<Pick>) {
        let Some(stars) = &self.stars else { return };
        let selected = self.selected;
        let moved = {
            let id = Id::unique("search-stars-results").with("last-selected");
            let moved = ui.data(|d| d.get_temp::<usize>(id)) != Some(selected);
            ui.data_mut(|d| d.insert_temp(id, selected));
            moved
        };
        ui.scope(|ui| {
            ui.visuals_mut().selection.bg_fill = accent();
            ui.visuals_mut().selection.stroke.color = BG;
            ui.style_mut().interaction.selectable_labels = false;
            let mut b = TableBuilder::new(ui)
                .id_salt("search-stars-results")
                .striped(true)
                .sense(Sense::click())
                .auto_shrink([false, false])
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::exact(96.0))
                .column(Column::remainder().clip(true))
                .column(Column::remainder().clip(true))
                .column(Column::exact(150.0))
                .column(Column::exact(60.0))
                .column(Column::exact(100.0));
            if moved {
                b = b.scroll_to_row(selected, None);
            }
            b.header(20.0, |mut h| {
                h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Dist")));
                h.col(|ui| _ = widgets::th(ui, "System"));
                h.col(|ui| _ = widgets::th(ui, "Star"));
                h.col(|ui| _ = widgets::th(ui, "Type · class"));
                h.col(|ui| _ = widgets::th(ui, "Main"));
                h.col(|ui| _ = widgets::th(ui, "Scanned"));
            })
            .body(|body| {
                body.rows(34.0, stars.results.len(), |mut row| {
                    let i = row.index();
                    let r = &stars.results[i];
                    let sel = i == selected;
                    let c = |normal: Color32| widgets::row_text_color(sel, normal);
                    row.set_selected(sel);
                    row.col(|ui| right(ui, |ui| _ = ui.label(RichText::new(format!("{} ly", distance(r.distance_ly))).font(theme::body_semi(16.0)).color(c(accent_hi())))));
                    row.col(|ui| _ = ui.add(egui::Label::new(RichText::new(r.system_name.trim()).font(theme::body_semi(16.0)).color(c(TEXT))).truncate()));
                    row.col(|ui| _ = ui.add(egui::Label::new(RichText::new(&r.star_name).font(theme::body(15.5)).color(c(TEXT))).truncate()));
                    let class = match &r.luminosity {
                        Some(l) => format!("{} · {l}", r.star_type),
                        None => r.star_type.clone(),
                    };
                    row.col(|ui| _ = ui.add(egui::Label::new(RichText::new(class).font(theme::body(15.5)).color(c(theme::YELLOW))).truncate()));
                    row.col(|ui| {
                        if r.main_star {
                            widgets::badge(ui, "Main", if sel { BG } else { theme::GREEN }).on_hover_text("The system's arrival star");
                        }
                    });
                    row.col(|ui| _ = ui.label(RichText::new(r.scanned_at.format("%Y-%m-%d").to_string()).font(theme::body(15.0)).color(c(TEXT_DIM))));
                    let resp = row.response();
                    if resp.double_clicked() {
                        picks.push(Pick::StarOpen(i));
                    } else if resp.clicked() {
                        picks.push(Pick::StarSelect(i));
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Copy system name  (c)").clicked() {
                            picks.push(Pick::StarCopy(i));
                            ui.close();
                        }
                        if ui.button("Show on the galaxy map  (Enter)").clicked() {
                            picks.push(Pick::StarOpen(i));
                            ui.close();
                        }
                    });
                });
            });
        });
    }
}

impl SearchView {
    // ---------------------------------------------------------------- Bodies

    fn bodies_ui(&mut self, ui: &mut Ui, picks: &mut Vec<Pick>) {
        let full = ui.available_rect_before_wrap();
        // The kinds are short names; the table needs the room.
        let [left, right] = split_narrow(full);
        let choices = self.body_choices();
        let picking = self.bodies.as_ref().is_none_or(|b| b.results.is_empty());
        let caption = if self.body_catalog.is_some() { String::new() } else { "groups only".to_string() };
        widgets::hud_panel_at(ui, left, "Kind of planet", &caption, |ui| {
            egui::ScrollArea::vertical().id_salt("search-bodies-kinds").auto_shrink([false, false]).show(ui, |ui| {
                let mut classes_started = false;
                for (i, (choice, count)) in choices.iter().enumerate() {
                    if matches!(choice, BodyChoice::Class(_)) && !classes_started {
                        classes_started = true;
                        ui.add_space(6.0);
                        widgets::section(ui, "Single classes", None);
                    }
                    let on = self.body_choice.as_ref() == Some(choice) || (picking && self.body_choice.is_none() && i == self.body_pick);
                    let note = count.map(|n| format!("{} known", group(n))).unwrap_or_default();
                    if kind_row(ui, on, &choice.label(), &note) {
                        picks.push(Pick::Body(choice.clone()));
                    }
                }
            });
        });

        let title = match &self.body_choice {
            Some(BodyChoice::Any) | None => "Nearest planets".to_string(),
            Some(c) => format!("Nearest: {}", c.label()),
        };
        let from = self.bodies.as_ref().and_then(|b| b.reference.name.clone()).unwrap_or_else(|| self.body_system.trim().to_string());
        let caption = if from.is_empty() { String::new() } else { format!("from {from}") };
        widgets::hud_panel_at(ui, right, &title, &caption, |ui| {
            self.body_options(ui, picks);
            ui.add_space(6.0);
            match &self.bodies {
                Some(b) => {
                    ui.label(RichText::new(known_bodies_line(b.known_total)).font(theme::body(14.5)).color(TEXT_DIM))
                        .on_hover_text("Before the filters. EDCAS only knows the planets commanders have scanned: the real nearest one may not be listed yet.");
                    ui.add_space(4.0);
                }
                None if self.body_choice.is_none() => {
                    widgets::empty_state(ui, "Pick a kind of planet on the left to see the nearest ones EDCAS knows.");
                    return;
                }
                None => {}
            }
            if self.loading {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Spinner::new().size(22.0).color(accent()));
                    widgets::hint(ui, "Searching…");
                });
            } else if !picking {
                self.bodies_table(ui, picks);
            } else if !self.status.is_empty() {
                widgets::empty_state(ui, &self.status);
            }
        });
    }

    /// The filters (two rows), Search and Copy system. A changed choice searches again; the
    /// typed fields search on Enter.
    fn body_options(&mut self, ui: &mut Ui, picks: &mut Vec<Pick>) {
        let label = |ui: &mut Ui, text: &str| _ = ui.label(RichText::new(text).font(theme::body(15.0)).color(theme::CYAN));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let f = &mut self.body_filters;
            for (name, tri, tip) in [
                ("Landable", &mut f.landable, "Bodies you can land on"),
                ("Terraformable", &mut f.terraformable, "Terraformable, being terraformed or terraformed"),
                ("Ringed", &mut f.ringed, "Bodies with rings"),
                ("Volcanism", &mut f.volcanism, "Bodies with any volcanism"),
            ] {
                label(ui, name);
                if tri_switch(ui, tri).on_hover_text(tip).changed() {
                    picks.push(Pick::BodySearch);
                }
                ui.add_space(10.0);
            }
        });
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            label(ui, "Atmosphere");
            let mut atmospheres: Vec<(String, i64)> =
                self.body_catalog.as_ref().map(|c| c.atmosphere_types.iter().map(|e| (e.label.clone(), e.count)).collect()).unwrap_or_default();
            // By the name shown; the server sends them by count.
            atmospheres.sort_by_cached_key(|(a, _)| atmosphere_name(a).to_lowercase());
            let current = self.body_filters.atmosphere.as_deref().map(atmosphere_name).unwrap_or_else(|| "any".to_string());
            let mut picked: Option<Option<String>> = None;
            egui::ComboBox::from_id_salt("search-bodies-atmosphere")
                .selected_text(RichText::new(current).font(theme::body_semi(15.0)).color(TEXT))
                .width(150.0)
                .height(420.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.body_filters.atmosphere.is_none(), "any").clicked() {
                        picked = Some(None);
                    }
                    for (a, n) in &atmospheres {
                        let on = self.body_filters.atmosphere.as_deref() == Some(a.as_str());
                        if ui.selectable_label(on, format!("{} · {}", atmosphere_name(a), group(*n))).clicked() {
                            picked = Some(Some(a.clone()));
                        }
                    }
                })
                .response
                .on_hover_text("One atmosphere type, as the scans name it");
            if let Some(a) = picked {
                if a != self.body_filters.atmosphere {
                    self.body_filters.atmosphere = a;
                    picks.push(Pick::BodySearch);
                }
            }
            ui.add_space(10.0);
            for (name, salt, which) in [("Bio ≥", "search-bodies-bio", 0), ("Geo ≥", "search-bodies-geo", 1)] {
                label(ui, name);
                let value = if which == 0 { &mut self.body_filters.min_bio } else { &mut self.body_filters.min_geo };
                if signal_combo(ui, salt, value).on_hover_text(if which == 0 { "At least this many biological signals" } else { "At least this many geological signals" }).changed() {
                    picks.push(Pick::BodySearch);
                }
                ui.add_space(10.0);
            }
            label(ui, "Max gravity");
            let g = ui.add(
                egui::TextEdit::singleline(&mut self.body_filters.max_gravity)
                    .id(Id::unique("search-bodies-max-gravity"))
                    .hint_text(RichText::new("any").color(GREY))
                    .font(theme::body(15.0))
                    .desired_width(50.0)
                    .margin(vec2(6.0, 3.0)),
            );
            ui.label(RichText::new("g").font(theme::body(15.0)).color(TEXT_DIM));
            ui.add_space(10.0);
            label(ui, "Max distance");
            let d = ui.add(
                egui::TextEdit::singleline(&mut self.body_filters.max_distance)
                    .id(Id::unique("search-bodies-max-distance"))
                    .hint_text(RichText::new("any").color(GREY))
                    .font(theme::body(15.0))
                    .desired_width(70.0)
                    .margin(vec2(6.0, 3.0)),
            );
            ui.label(RichText::new("ly").font(theme::body(15.0)).color(TEXT_DIM));
            if widgets::submitted(ui, &g) || widgets::submitted(ui, &d) {
                picks.push(Pick::BodySearch);
            }
            ui.add_space(10.0);
            let can_search = self.body_choice.is_some() && !self.loading;
            if widgets::hud_button_enabled(ui, "Search", can_search).on_hover_text("Search again  (Enter)").clicked() {
                picks.push(Pick::BodySearch);
            }
            let any = self.bodies.as_ref().is_some_and(|b| !b.results.is_empty());
            if widgets::hud_button_enabled(ui, "Copy system", any).on_hover_text("Copy the selected planet's system name  (c)").clicked() {
                picks.push(Pick::BodyCopy(self.selected));
            }
        });
    }

    /// The results: distance, body, system, class, arrival distance, gravity, temperature,
    /// atmosphere, marks, signals.
    fn bodies_table(&mut self, ui: &mut Ui, picks: &mut Vec<Pick>) {
        let Some(bodies) = &self.bodies else { return };
        let selected = self.selected;
        let moved = {
            let id = Id::unique("search-bodies-results").with("last-selected");
            let moved = ui.data(|d| d.get_temp::<usize>(id)) != Some(selected);
            ui.data_mut(|d| d.insert_temp(id, selected));
            moved
        };
        ui.scope(|ui| {
            ui.visuals_mut().selection.bg_fill = accent();
            ui.visuals_mut().selection.stroke.color = BG;
            ui.style_mut().interaction.selectable_labels = false;
            let mut b = TableBuilder::new(ui)
                .id_salt("search-bodies-results")
                .striped(true)
                .sense(Sense::click())
                .auto_shrink([false, false])
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::exact(88.0))
                .column(Column::remainder().at_least(70.0).clip(true))
                .column(Column::remainder().clip(true))
                .column(Column::exact(130.0).clip(true))
                .column(Column::exact(74.0))
                .column(Column::exact(58.0))
                .column(Column::exact(56.0))
                .column(Column::exact(112.0).clip(true))
                .column(Column::exact(80.0))
                .column(Column::exact(30.0))
                // Room on the right for the scroll bar.
                .column(Column::exact(42.0));
            if moved {
                b = b.scroll_to_row(selected, None);
            }
            b.header(20.0, |mut h| {
                h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Dist")));
                h.col(|ui| _ = widgets::th(ui, "Body"));
                h.col(|ui| _ = widgets::th(ui, "System"));
                h.col(|ui| _ = widgets::th(ui, "Class"));
                h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Arrival")));
                h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Grav")));
                h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Temp")));
                h.col(|ui| _ = widgets::th(ui, "Atmosphere"));
                h.col(|ui| _ = widgets::th(ui, "Marks"));
                h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Bio").on_hover_text("Biological signals")));
                h.col(|ui| right(ui, |ui| {
                    ui.add_space(12.0);
                    widgets::th(ui, "Geo").on_hover_text("Geological signals");
                }));
            })
            .body(|body| {
                body.rows(34.0, bodies.results.len(), |mut row| {
                    let i = row.index();
                    let r = &bodies.results[i];
                    let sel = i == selected;
                    let c = |normal: Color32| widgets::row_text_color(sel, normal);
                    let dash = || "—".to_string();
                    row.set_selected(sel);
                    row.col(|ui| right(ui, |ui| _ = ui.label(RichText::new(format!("{} ly", distance(r.distance_ly))).font(theme::body_semi(16.0)).color(c(accent_hi())))));
                    row.col(|ui| {
                        ui.add(egui::Label::new(RichText::new(body_short(&r.body_name, &r.system_name)).font(theme::body_semi(16.0)).color(c(TEXT))).truncate())
                            .on_hover_text(&r.body_name);
                    });
                    row.col(|ui| _ = ui.add(egui::Label::new(RichText::new(r.system_name.trim()).font(theme::body(15.5)).color(c(TEXT))).truncate()));
                    row.col(|ui| {
                        ui.add(egui::Label::new(RichText::new(short_class(&r.planet_class)).font(theme::body(15.5)).color(c(theme::YELLOW))).truncate())
                            .on_hover_text(&r.planet_class);
                    });
                    let arrival = r.distance_from_arrival_ls.map(|ls| format!("{} ls", group(ls.round() as i64))).unwrap_or_else(dash);
                    row.col(|ui| right(ui, |ui| _ = ui.label(RichText::new(arrival).font(theme::body(15.0)).color(c(TEXT_DIM)))));
                    let gravity = r.gravity_g.map(|g| format!("{g:.2} g")).unwrap_or_else(dash);
                    row.col(|ui| right(ui, |ui| _ = ui.label(RichText::new(gravity).font(theme::body(15.0)).color(c(TEXT)))));
                    let temp = r.surface_temperature.map(|t| format!("{} K", group(t.round() as i64))).unwrap_or_else(dash);
                    row.col(|ui| right(ui, |ui| _ = ui.label(RichText::new(temp).font(theme::body(15.0)).color(c(TEXT)))));
                    let (atmo, atmo_tip) = atmosphere_cell(r.atmosphere_type.as_deref(), r.atmosphere.as_deref(), r.pressure_atm);
                    row.col(|ui| {
                        let resp = ui.add(egui::Label::new(RichText::new(atmo).font(theme::body(15.0)).color(c(TEXT))).truncate());
                        if let Some(tip) = &atmo_tip {
                            resp.on_hover_text(tip);
                        }
                    });
                    row.col(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        let mark = |ui: &mut Ui, on: bool, text: &str, col: Color32, tip: &str| {
                            if on {
                                widgets::badge(ui, text, if sel { BG } else { col }).on_hover_text(tip);
                            }
                        };
                        mark(ui, r.landable, "L", theme::GREEN, "Landable");
                        let terraform = r.terraform_state.as_deref().unwrap_or("");
                        mark(ui, !terraform.is_empty(), "T", theme::CYAN, terraform);
                        let rings = format!("Rings: {}", r.ring_classes.join(", "));
                        mark(ui, !r.ring_classes.is_empty(), "R", theme::YELLOW, &rings);
                        let volcanism = r.volcanism.as_deref().unwrap_or("").trim();
                        mark(ui, !volcanism.is_empty(), "V", theme::RED, volcanism);
                    });
                    let signals = |n: i32, col: Color32| RichText::new(if n > 0 { n.to_string() } else { "·".into() }).font(theme::body_semi(15.5)).color(c(if n > 0 { col } else { GREY }));
                    row.col(|ui| right(ui, |ui| _ = ui.label(signals(r.bio_signals, theme::GREEN))));
                    row.col(|ui| right(ui, |ui| {
                        ui.add_space(12.0);
                        ui.label(signals(r.geo_signals, theme::YELLOW));
                    }));
                    let resp = row.response();
                    if resp.double_clicked() {
                        picks.push(Pick::BodyOpen(i));
                    } else if resp.clicked() {
                        picks.push(Pick::BodySelect(i));
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Copy system name  (c)").clicked() {
                            picks.push(Pick::BodyCopy(i));
                            ui.close();
                        }
                        if ui.button("Show on the galaxy map  (Enter)").clicked() {
                            picks.push(Pick::BodyOpen(i));
                            ui.close();
                        }
                    });
                });
            });
        });
    }
}

/// any / yes / no as three joined buttons. The response is `changed` when one was clicked.
fn tri_switch(ui: &mut Ui, value: &mut Tri) -> egui::Response {
    let mut changed = false;
    let mut resp = ui
        .scope(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for t in Tri::ALL {
                let on = *value == t;
                let text = RichText::new(t.label()).font(theme::body_semi(14.0)).color(if on { BG } else { TEXT_DIM });
                let b = egui::Button::new(text).fill(if on { accent() } else { theme::PANEL }).stroke(egui::Stroke::new(1.0, theme::alpha(accent(), 110))).corner_radius(0.0).min_size(vec2(34.0, 22.0));
                if ui.add(b).clicked() && !on {
                    *value = t;
                    changed = true;
                }
            }
        })
        .response;
    if changed {
        resp.mark_changed();
    }
    resp
}

/// A drop-down for "at least N signals": any, 1 to 12. The response is `changed` when a
/// different number was picked.
fn signal_combo(ui: &mut Ui, salt: &str, value: &mut u32) -> egui::Response {
    let text = |n: u32| if n == 0 { "any".to_string() } else { n.to_string() };
    let mut picked = None;
    let mut resp = egui::ComboBox::from_id_salt(salt)
        .selected_text(RichText::new(text(*value)).font(theme::body_semi(15.0)).color(TEXT))
        .width(52.0)
        .height(420.0)
        .show_ui(ui, |ui| {
            for n in 0..=12 {
                if ui.selectable_label(*value == n, text(n)).clicked() {
                    picked = Some(n);
                }
            }
        })
        .response;
    if let Some(n) = picked {
        if n != *value {
            *value = n;
            resp.mark_changed();
        }
    }
    resp
}

/// The body's name without its system's in front: "A 3" for "Plooe Thoi UW-N c6-10 A 3";
/// a name of its own ("Earth") stays.
fn body_short(body: &str, system: &str) -> String {
    let (body, system) = (body.trim(), system.trim());
    match body.strip_prefix(system) {
        Some(rest) if rest.starts_with(' ') && !rest.trim().is_empty() => rest.trim().to_string(),
        _ => body.to_string(),
    }
}

/// A planet class for a narrow column: "High metal content body" as "High metal content",
/// "Sudarsky class III gas giant" as "Class III gas giant".
fn short_class(class: &str) -> String {
    let c = class.strip_prefix("Sudarsky class ").map(|rest| format!("Class {rest}")).unwrap_or_else(|| class.to_string());
    c.strip_suffix(" body").map(str::to_string).unwrap_or(c)
}

/// An atmosphere type in words: `CarbonDioxideRich` as "Carbon dioxide rich", `None` as
/// "No atmosphere".
fn atmosphere_name(kind: &str) -> String {
    match kind {
        "None" | "" => return "No atmosphere".into(),
        "EarthLike" => return "Earth-like".into(),
        _ => {}
    }
    let mut out = String::new();
    for (i, ch) in kind.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push(' ');
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// The atmosphere column's text and its hover text (the scan's description and the
/// pressure): "none" for a body without one, "—" when unknown.
fn atmosphere_cell(kind: Option<&str>, description: Option<&str>, pressure_atm: Option<f32>) -> (String, Option<String>) {
    let text = match kind {
        None => "—".to_string(),
        Some("None") | Some("") => "none".to_string(),
        Some(k) => atmosphere_name(k),
    };
    let mut tip: Vec<String> = Vec::new();
    if let Some(d) = description.map(str::trim).filter(|d| !d.is_empty()) {
        tip.push(d.to_string());
    }
    if let Some(p) = pressure_atm.filter(|p| *p > 0.0) {
        tip.push(format!("{p:.3} atm"));
    }
    (text, if tip.is_empty() { None } else { Some(tip.join(" · ")) })
}

/// A picker entry: the kind, with its known count on the right. True when clicked.
fn kind_row(ui: &mut Ui, on: bool, name: &str, note: &str) -> bool {
    widgets::selectable_row(ui, on, 26.0, |ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if !note.is_empty() {
                ui.label(RichText::new(note).font(theme::body(13.5)).color(widgets::row_text_color(on, TEXT_DIM)));
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add(egui::Label::new(RichText::new(name).font(theme::body_semi(15.5)).color(widgets::row_text_color(on, TEXT))).truncate());
            });
        });
    })
    .clicked()
}

/// A distance in ly with thousands grouped: 3,048.51.
fn distance(ly: f32) -> String {
    let cents = (ly.max(0.0) as f64 * 100.0).round() as i64;
    format!("{}.{:02}", group(cents / 100), cents % 100)
}

fn right(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.with_layout(Layout::right_to_left(Align::Center), add);
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

/// A narrower picking column than [`split`], for a results table with many columns.
fn split_narrow(r: Rect) -> [Rect; 2] {
    let w = (r.width() * 0.22).clamp(240.0, 320.0);
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

#[cfg(test)]
mod tests {
    #[test]
    fn distances_group_thousands_and_keep_two_places() {
        assert_eq!(super::distance(3048.509), "3,048.51");
        assert_eq!(super::distance(139.285), "139.29");
        assert_eq!(super::distance(9.996), "10.00");
        assert_eq!(super::distance(0.0), "0.00");
    }

    #[test]
    fn body_names_drop_their_system() {
        assert_eq!(super::body_short("Plooe Thoi UW-N c6-10 A 3", "Plooe Thoi UW-N c6-10"), "A 3");
        assert_eq!(super::body_short("Earth", "Sol"), "Earth");
        // Only a whole word of the system name goes.
        assert_eq!(super::body_short("Solati 4", "Sol"), "Solati 4");
        assert_eq!(super::body_short("Sol", "Sol"), "Sol");
    }

    #[test]
    fn classes_and_atmospheres_read_short() {
        assert_eq!(super::short_class("High metal content body"), "High metal content");
        assert_eq!(super::short_class("Sudarsky class III gas giant"), "Class III gas giant");
        assert_eq!(super::short_class("Water world"), "Water world");
        assert_eq!(super::atmosphere_name("CarbonDioxideRich"), "Carbon dioxide rich");
        assert_eq!(super::atmosphere_name("None"), "No atmosphere");
        assert_eq!(super::atmosphere_name("EarthLike"), "Earth-like");
        assert_eq!(super::atmosphere_cell(Some("None"), None, None), ("none".to_string(), None));
        assert_eq!(super::atmosphere_cell(None, None, None).0, "—");
        assert_eq!(
            super::atmosphere_cell(Some("SulphurDioxide"), Some("thin sulphur dioxide atmosphere"), Some(0.0123)),
            ("Sulphur dioxide".to_string(), Some("thin sulphur dioxide atmosphere · 0.012 atm".to_string()))
        );
    }
}
