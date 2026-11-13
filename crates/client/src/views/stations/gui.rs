//! Desktop-window drawing for [`StationsView`]: the station list (pinned, searched, current
//! system, history) beside a detail panel with Overview / Market / Outfitting / Shipyard /
//! Price History sub-tabs. Every control maps onto the same state and key handling the
//! terminal view uses.

use std::collections::HashMap;

use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, Layout, Rect, Sense, Shape, Stroke, Ui};
use edcas_common::api::{CommodityResponse, ModuleResponse, ShipResponse, StationResponse};
use edcas_common::format::group_thousands;
use egui_extras::{Column, TableBuilder};

use super::{ListItem, ListSection, StationsView};
use crate::api_client::ApiClient;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::gui::theme::{self, accent, accent_hi, alpha, BG, CYAN, GREEN, GREY, PANEL, RED, TEXT, TEXT_DIM, YELLOW};
use crate::gui::widgets;
use crate::journal_reader::{JournalData, StationData};
use crate::todo::TodoList;
use crate::views::construction::{depot_ui, selection_marker, soften_table_highlights, DepotSummary, ResourceAction};
use crate::views::util::{
    compute_todo_needed, effective_todo_for_market, fmt_ts, normalize_commodity_name, raw_diff,
    sorted_commodities, status_style, FocusArea, MarketSortCol, SearchState, StationDetailTab as DetailTab,
};
use crate::views::ViewEvent;

/// Height of one station-list line (headings use the same slot so the list can be virtualised).
const LIST_ROW_H: f32 = 40.0;
/// Pixels per `detail_scroll` step, so keyboard scrolling moves the detail panel.
const SCROLL_STEP: f32 = 22.0;
/// Colour of history entries (stations visited in other systems).
const HISTORY: Color32 = Color32::from_rgb(100, 180, 200);
const DETAIL_TABS: [DetailTab; 5] =
    [DetailTab::Overview, DetailTab::Market, DetailTab::Outfitting, DetailTab::Shipyard, DetailTab::PriceHistory];
const MARKET_COLS: [(MarketSortCol, &str, &str); 8] = [
    (MarketSortCol::Name, "Commodity", "Sort by name (1)"),
    (MarketSortCol::Buy, "Buy", "Sort by buy price (2)"),
    (MarketSortCol::BuyDiff, "Δ", "Sort by buy price minus galactic mean (3)"),
    (MarketSortCol::Sell, "Sell", "Sort by sell price (4)"),
    (MarketSortCol::SellDiff, "Δ", "Sort by sell price minus galactic mean (5)"),
    (MarketSortCol::Mean, "Mean", "Sort by galactic mean price (6)"),
    (MarketSortCol::Stock, "Stock", "Sort by stock (7)"),
    (MarketSortCol::Demand, "Demand", "Sort by demand (8)"),
];

/// A user action from this frame, applied after drawing (drawing only borrows the view).
enum Act {
    /// Exactly what pressing the key does now.
    Key(KeyCode),
    /// The key, pressed with the given panel focused (the panel the control sits in).
    KeyIn(FocusArea, KeyCode),
    /// The key as if pressed in the list, without moving the focus there.
    KeyFromList(KeyCode),
    Select(usize),
    Tab(DetailTab),
    Sort(MarketSortCol),
    MarketRow(usize),
    Resource(usize),
}

/// One slot of the station list: a section heading or a station (`idx` into the display list).
enum Entry<'a> {
    Heading(String, usize),
    Station { idx: usize, section: ListSection, item: ListItem<'a> },
}

impl StationsView {
    pub fn ui(&mut self, ui: &mut Ui, api: &ApiClient, journal: &JournalData, todo: &mut TodoList) -> ViewEvent {
        let mut acts = Vec::new();
        self.select_wanted(journal);
        let full = ui.available_rect_before_wrap();
        let left_w = (full.width() * 0.32).clamp(280.0, 470.0);
        let left = Rect::from_min_size(full.min, vec2(left_w, full.height()));
        let right = Rect::from_min_max(pos2(left.right() + 12.0, full.top()), full.max);

        let count = self.display_count(journal);
        let caption = if count == 1 { "1 station".to_string() } else { format!("{count} stations") };
        widgets::hud_panel_at(ui, left, "Stations", &caption, |ui| self.list_ui(ui, journal, &mut acts));
        let caption = self.detail_caption(journal);
        widgets::hud_panel_at(ui, right, "Station details", caption, |ui| self.detail_ui(ui, journal, todo, &mut acts));

        let mut event = ViewEvent::None;
        for act in acts {
            let ev = self.apply(act, api, journal, todo);
            if matches!(event, ViewEvent::None | ViewEvent::Consumed) && !matches!(ev, ViewEvent::None) {
                event = ev;
            }
        }
        event
    }

    fn apply(&mut self, act: Act, api: &ApiClient, journal: &JournalData, todo: &mut TodoList) -> ViewEvent {
        let key = KeyEvent::from;
        match act {
            Act::Key(code) => return self.handle_key(&key(code), api, journal, todo),
            Act::KeyIn(focus, code) => {
                self.focus = focus;
                return self.handle_key(&key(code), api, journal, todo);
            }
            Act::KeyFromList(code) => {
                let prev = self.focus;
                self.focus = FocusArea::List;
                let ev = self.handle_key(&key(code), api, journal, todo);
                self.focus = prev;
                return ev;
            }
            Act::Select(idx) => {
                self.focus = FocusArea::List;
                if idx != self.selected_idx {
                    self.select_list_idx(idx, journal);
                }
            }
            Act::Tab(tab) => {
                self.focus = FocusArea::Detail;
                if tab != self.detail_tab {
                    self.detail_tab = tab;
                    self.detail_scroll = 0;
                }
            }
            Act::Sort(col) => {
                self.focus = FocusArea::Detail;
                self.sort_market_by(col);
            }
            Act::MarketRow(i) => {
                self.focus = FocusArea::Detail;
                self.market_selected_row = i;
            }
            Act::Resource(i) => {
                self.focus = FocusArea::Detail;
                self.construction_resource_idx = i;
            }
        }
        ViewEvent::Consumed
    }

    // ------------------------------------------------------------ list

    fn list_ui(&mut self, ui: &mut Ui, journal: &JournalData, acts: &mut Vec<Act>) {
        if !self.embedded {
            self.search_box(ui, acts);
        }
        let status_col = status_style(&self.status_msg).fg.map(theme::from_ratatui).unwrap_or(TEXT_DIM);
        ui.add(egui::Label::new(egui::RichText::new(&self.status_msg).color(status_col).font(theme::body(14.5))).truncate())
            .on_hover_text(&self.status_msg);
        ui.add_space(2.0);

        let entries = self.list_entries(journal);
        if entries.is_empty() {
            widgets::empty_state(ui, "No stations visited yet — dock somewhere in-game, or check the journal directory in Settings.");
            return;
        }

        let id = ui.id().with("stations-list-scroll");
        let (prev_sel, prev_off, view_h): (Option<usize>, f32, f32) = ui.data(|d| d.get_temp(id)).unwrap_or((None, 0.0, 0.0));
        let mut area = egui::ScrollArea::vertical().id_salt("stations-list").auto_shrink([false, false]);
        if prev_sel != Some(self.selected_idx) {
            if let Some(slot) = entries.iter().position(|e| matches!(e, Entry::Station { idx, .. } if *idx == self.selected_idx)) {
                let top = slot as f32 * LIST_ROW_H;
                if top < prev_off {
                    area = area.vertical_scroll_offset(top);
                } else if top + LIST_ROW_H > prev_off + view_h {
                    area = area.vertical_scroll_offset(top + LIST_ROW_H - view_h);
                }
            }
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        let out = area.show_rows(ui, LIST_ROW_H, entries.len(), |ui, range| {
            for entry in &entries[range] {
                match entry {
                    Entry::Heading(label, n) => {
                        ui.add_space(LIST_ROW_H - 24.0);
                        widgets::section(ui, label, Some(*n));
                    }
                    Entry::Station { idx, section, item } => self.station_row(ui, *idx, *section, item, journal, acts),
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(id, (Some(self.selected_idx), out.state.offset.y, out.inner_rect.height())));
    }

    /// The search field and button. Typing mode (`/` or `f`) focuses the field; Enter searches,
    /// Esc cancels, just like the keys in the terminal.
    fn search_box(&mut self, ui: &mut Ui, acts: &mut Vec<Act>) {
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 86.0).max(80.0);
            let resp = widgets::text_field(ui, &mut self.search_query, "Search stations by name…", w)
                .on_hover_text("Station name to look up on edcas.de (/ or f to start typing, Enter to search)");
            let typing = self.search_state == SearchState::Typing;
            if widgets::submitted(ui, &resp) {
                self.search_state = SearchState::Typing;
                acts.push(Act::Key(KeyCode::Enter));
            } else if resp.lost_focus() {
                if typing {
                    self.search_state = SearchState::Idle;
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.status_msg = "Search cancelled".into();
                    }
                }
            } else if resp.gained_focus() {
                self.search_state = SearchState::Typing;
                self.status_msg = "Typing… (Enter to search, Esc to cancel)".into();
            } else if typing && !resp.has_focus() {
                resp.request_focus();
            }
            if widgets::hud_button(ui, "Search").on_hover_text("Search stations by name (Enter)").clicked() {
                self.search_state = SearchState::Typing;
                acts.push(Act::Key(KeyCode::Enter));
            }
        });
        ui.add_space(4.0);
    }

    fn list_entries<'a>(&'a self, journal: &'a JournalData) -> Vec<Entry<'a>> {
        let list = self.build_sectioned_list(journal);
        let current_sys = journal.current_system.as_ref().map(|s| s.name.clone()).unwrap_or_default();
        let headings = [
            (ListSection::Pinned, "Pinned".to_string()),
            (ListSection::Search, "Search results".to_string()),
            (ListSection::CurrentSystem, if current_sys.is_empty() { "Current system".to_string() } else { current_sys }),
            (ListSection::History, "History".to_string()),
        ];
        let counts: Vec<usize> = headings.iter().map(|(s, _)| list.iter().filter(|(sec, _)| sec == s).count()).collect();
        let mut out = Vec::with_capacity(list.len() + 4);
        let mut last = None;
        for (idx, (section, item)) in list.into_iter().enumerate() {
            if last != Some(section) {
                let h = headings.iter().position(|(s, _)| *s == section).unwrap_or(0);
                out.push(Entry::Heading(headings[h].1.clone(), counts[h]));
                last = Some(section);
            }
            out.push(Entry::Station { idx, section, item });
        }
        out
    }

    /// One station in the list: marker icon, name, type (and system away from the current one).
    fn station_row(&self, ui: &mut Ui, idx: usize, section: ListSection, item: &ListItem<'_>, journal: &JournalData, acts: &mut Vec<Act>) {
        let (mid, name, kind, system) = item_basics(item);
        let is_carrier = journal.visited_carriers.iter().any(|c| c.market_id == mid);
        let mine = journal.carrier_cargo.contains_key(&mid);
        let live = matches!(item, ListItem::Api(_)) && self.auto_fetched.contains_key(&mid);
        let construction = self.is_construction(mid, journal);
        let pinned = self.pinned_ids.contains(&mid);
        let (icon, icon_col, name_col) = match section {
            ListSection::Pinned if pinned => ("★", YELLOW, YELLOW),
            ListSection::Pinned => ("⚙", YELLOW, YELLOW),
            ListSection::Search => ("", TEXT, TEXT),
            _ if is_carrier && mine => ("◈", accent(), accent()),
            _ if live => ("●", GREEN, GREEN),
            ListSection::History => ("⌚", HISTORY, HISTORY),
            _ => ("", TEXT, TEXT),
        };
        let subtitle = if section == ListSection::CurrentSystem || system.is_empty() {
            kind.to_string()
        } else if kind.is_empty() {
            system.to_string()
        } else {
            format!("{kind} · {system}")
        };
        // The ⚙ tag marks construction sites, unless the row's icon already is one.
        let tag = construction && icon != "⚙";

        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), LIST_ROW_H), Sense::click());
        let selected = idx == self.selected_idx;
        let filled = selected && self.focus == FocusArea::List;
        let p = ui.painter_at(rect);
        let row = rect.shrink2(vec2(0.0, 2.0));
        if filled {
            p.rect_filled(row, 0.0, accent());
        } else if selected {
            p.rect_filled(row, 0.0, alpha(accent(), 44));
            p.rect_filled(Rect::from_min_size(row.min, vec2(3.0, row.height())), 0.0, accent());
        } else if resp.hovered() {
            p.rect_filled(row, 0.0, alpha(accent(), 26));
        }
        let c = |col: Color32| widgets::row_text_color(filled, col);
        if !icon.is_empty() {
            widgets::txt(&p, pos2(row.left() + 14.0, row.top() + 12.0), Align2::CENTER_CENTER, icon, theme::body(15.0), c(icon_col), 0.0);
        }
        let text_right = row.right() - if tag { 28.0 } else { 6.0 };
        let text_p = p.with_clip_rect(Rect::from_min_max(row.min, pos2(text_right, row.bottom())).intersect(p.clip_rect()));
        let name_font = if selected { theme::body_semi(16.5) } else { theme::body(16.5) };
        widgets::txt(&text_p, pos2(row.left() + 28.0, row.top() + 12.0), Align2::LEFT_CENTER, name, name_font, c(name_col), 0.0);
        widgets::txt(&text_p, pos2(row.left() + 28.0, row.top() + 27.0), Align2::LEFT_CENTER, &subtitle, theme::body(13.5), c(TEXT_DIM), 0.0);
        if tag {
            widgets::txt(&p, pos2(row.right() - 14.0, row.center().y), Align2::CENTER_CENTER, "⚙", theme::body(15.0), c(YELLOW), 0.0);
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() || resp.secondary_clicked() {
            acts.push(Act::Select(idx));
        }
        resp.on_hover_text(format!("{name}\nClick to select · right-click for actions")).context_menu(|ui| {
            let mut chosen = None;
            if ui.button(if idx < self.pinned_results.len() { "Unpin  (p)" } else { "Pin  (p)" }).clicked() {
                chosen = Some('p');
            }
            if ui.button("Copy system name  (c)").clicked() {
                chosen = Some('c');
            }
            if is_carrier && ui.button(if mine { "Unmark as my carrier  (m)" } else { "Mark as my carrier  (m)" }).clicked() {
                chosen = Some('m');
            }
            if construction && ui.button("Add to / remove from todo  (t)").clicked() {
                chosen = Some('t');
            }
            if let Some(code) = chosen {
                acts.push(Act::Select(idx));
                acts.push(Act::Key(KeyCode::Char(code)));
                ui.close();
            }
        });
    }

    /// Whether the station is a construction site this view knows about (the ⚙ tag).
    fn is_construction(&self, mid: i64, journal: &JournalData) -> bool {
        self.construction_tracked.contains(&mid)
            || journal.construction_depots.contains_key(&mid)
            || self.depots.iter().any(|d| d.market_id == mid)
    }

    // ------------------------------------------------------------ detail

    fn detail_caption(&self, journal: &JournalData) -> &'static str {
        match self.selected_item(journal) {
            Some(ListItem::Api(s)) if self.pinned_ids.contains(&s.market_id) => "pinned · live data",
            Some(ListItem::Api(_)) => "live data",
            Some(ListItem::Journal(s)) if docked_at(journal, s.market_id) => "docked",
            Some(ListItem::Journal(_)) => "visit snapshot",
            None => "",
        }
    }

    fn detail_ui(&self, ui: &mut Ui, journal: &JournalData, todo: &TodoList, acts: &mut Vec<Act>) {
        let Some(item) = self.selected_item(journal) else {
            widgets::empty_state(ui, "Select a station from the list.");
            return;
        };
        self.detail_header(ui, &item, journal, todo, acts);
        ui.add_space(6.0);
        let labels: Vec<&str> = DETAIL_TABS.iter().map(|t| t.label()).collect();
        let current = DETAIL_TABS.iter().position(|t| *t == self.detail_tab).unwrap_or(0);
        if let Some(i) = widgets::tab_strip(ui, &labels, current) {
            acts.push(Act::Tab(DETAIL_TABS[i]));
        }
        ui.add_space(8.0);
        match self.detail_tab {
            DetailTab::Overview => self.overview_tab(ui, &item, journal, acts),
            DetailTab::Market => self.market_tab(ui, &item, journal, todo, acts),
            DetailTab::Outfitting => match &item {
                ListItem::Api(s) => self.scrolled(ui, s.market_id, "outfitting", |ui| outfitting_ui(ui, &s.modules)),
                ListItem::Journal(s) => widgets::empty_state(ui, self.snapshot_data_msg(s)),
            },
            DetailTab::Shipyard => match &item {
                ListItem::Api(s) => self.scrolled(ui, s.market_id, "shipyard", |ui| shipyard_ui(ui, &s.ships)),
                ListItem::Journal(s) => widgets::empty_state(ui, self.snapshot_data_msg(s)),
            },
            DetailTab::PriceHistory => self.price_history_ui(ui),
        }
    }

    /// Name, status badges, system, and the actions that apply to any selected station.
    fn detail_header(&self, ui: &mut Ui, item: &ListItem<'_>, journal: &JournalData, todo: &TodoList, acts: &mut Vec<Act>) {
        let (mid, name, kind, system) = item_basics(item);
        let is_carrier = journal.visited_carriers.iter().any(|c| c.market_id == mid);
        let mine = journal.carrier_cargo.contains_key(&mid);
        let construction = self.is_construction(mid, journal);

        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(name).font(theme::body_semi(24.0)).color(accent_hi()));
            if !kind.is_empty() {
                widgets::badge(ui, kind, CYAN);
            }
            if self.pinned_ids.contains(&mid) {
                widgets::badge(ui, "Pinned", YELLOW);
            }
            if construction {
                widgets::badge(ui, "Construction", YELLOW);
            }
            if is_carrier && mine {
                widgets::badge(ui, "My carrier", accent());
            }
            match item {
                ListItem::Api(_) => widgets::badge(ui, "Live", GREEN),
                ListItem::Journal(s) if docked_at(journal, s.market_id) => widgets::badge(ui, "Docked", GREEN),
                ListItem::Journal(_) => widgets::badge(ui, "Snapshot", GREY),
            };
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(system).font(theme::body(17.0)).color(TEXT));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::hud_button(ui, "Copy system").on_hover_text(format!("Copy “{}” to the clipboard (c)", system.trim())).clicked() {
                    acts.push(Act::Key(KeyCode::Char('c')));
                }
                let pin_label = if self.selected_idx < self.pinned_results.len() { "Unpin" } else { "Pin" };
                if widgets::hud_button(ui, pin_label).on_hover_text("Keep this station at the top of the list, or release it (p)").clicked() {
                    acts.push(Act::Key(KeyCode::Char('p')));
                }
                if is_carrier {
                    let label = if mine { "Not my carrier" } else { "My carrier" };
                    if widgets::hud_button(ui, label).on_hover_text("Mark or unmark this fleet carrier as yours, to track its inventory (m)").clicked() {
                        acts.push(Act::KeyFromList(KeyCode::Char('m')));
                    }
                }
                if construction {
                    let label = if todo.construction_items.iter().any(|i| i.market_id == mid) { "Remove from todo" } else { "Add to todo" };
                    if widgets::hud_button(ui, label).on_hover_text("Add this site's missing resources to the Todo list, or remove them (t)").clicked() {
                        acts.push(Act::KeyFromList(KeyCode::Char('t')));
                    }
                }
            });
        });
    }

    /// A vertical scroll area per station and tab that follows the keyboard's `detail_scroll`.
    fn scrolled(&self, ui: &mut Ui, mid: i64, tab: &str, add: impl FnOnce(&mut Ui)) {
        let id = ui.id().with(("detail-scroll", mid, tab));
        let prev: Option<usize> = ui.data(|d| d.get_temp(id));
        let mut area = egui::ScrollArea::vertical().id_salt(("station-detail", mid, tab)).auto_shrink([false, false]);
        if prev.is_some_and(|p| p != self.detail_scroll) {
            area = area.vertical_scroll_offset(self.detail_scroll as f32 * SCROLL_STEP);
        }
        area.show(ui, add);
        ui.data_mut(|d| d.insert_temp(id, self.detail_scroll));
    }

    // ------------------------------------------------------------ overview

    fn overview_tab(&self, ui: &mut Ui, item: &ListItem<'_>, journal: &JournalData, acts: &mut Vec<Act>) {
        let (mid, ..) = item_basics(item);
        let (facts, services) = match item {
            ListItem::Api(s) => (api_facts(s, journal), &s.services),
            ListItem::Journal(s) => (journal_facts(s), &s.services),
        };
        self.scrolled(ui, mid, "overview", |ui| {
            let facts_ui = |ui: &mut Ui| {
                widgets::section(ui, "Station", None);
                egui::Grid::new(("station-facts", mid)).num_columns(2).min_col_width(96.0).spacing([16.0, 5.0]).show(ui, |ui| {
                    for (k, v, dim) in &facts {
                        let text = if *dim { egui::RichText::new(v).color(GREY).font(theme::body(15.0)) } else { widgets::value(v) };
                        widgets::kv(ui, k, text);
                    }
                });
            };
            let services_ui = |ui: &mut Ui| {
                widgets::section(ui, "Services", Some(services.len()));
                if services.is_empty() {
                    widgets::hint(ui, "No services reported.");
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                        for s in services.iter() {
                            chip(ui, s);
                        }
                    });
                }
            };
            if ui.available_width() >= 620.0 {
                ui.columns(2, |cols| {
                    facts_ui(&mut cols[0]);
                    services_ui(&mut cols[1]);
                });
            } else {
                facts_ui(ui);
                ui.add_space(8.0);
                services_ui(ui);
            }

            if let ListItem::Journal(s) = item {
                ui.add_space(10.0);
                if s.station_type == "FleetCarrier" {
                    self.carrier_ui(ui, s.market_id, journal, acts);
                } else {
                    let has_session_market = !s.commodities.is_empty()
                        || journal.local_market.as_ref().map(|(m, _)| *m == s.market_id).unwrap_or(false);
                    let msg = if has_session_market {
                        "Market data captured this session — see the Market tab."
                    } else if self.live_fetch_pending(s.market_id) {
                        "Loading live market data…"
                    } else {
                        "No market data available for this station."
                    };
                    widgets::hint(ui, msg);
                }
            }

            if self.is_construction(mid, journal) {
                ui.add_space(12.0);
                self.construction_ui(ui, mid, journal, acts);
            }
        });
    }

    /// Construction progress with its actions (best station, nearest seller).
    fn construction_ui(&self, ui: &mut Ui, mid: i64, journal: &JournalData, acts: &mut Vec<Act>) {
        widgets::section(ui, "Construction", None);
        let depot = if let Some(local) = journal.construction_depots.get(&mid) {
            DepotSummary::from_local(local)
        } else if let Some(d) = self.depots.iter().find(|d| d.market_id == mid) {
            DepotSummary::from_response(d)
        } else {
            widgets::hint(ui, "No construction data for this site. Dock at the construction site to load resource requirements.");
            return;
        };
        let selected = (self.focus == FocusArea::Detail).then_some(self.construction_resource_idx);
        ui.horizontal(|ui| {
            if widgets::hud_button(ui, "Best station")
                .on_hover_text("Find the one station that sells the most of the missing resources (g)")
                .clicked()
            {
                acts.push(Act::KeyIn(FocusArea::Detail, KeyCode::Char('g')));
            }
            let target = depot.resource_name(self.construction_resource_idx).unwrap_or("the selected resource");
            if widgets::hud_button(ui, "Nearest seller")
                .on_hover_text(format!("Find the nearest station selling {target} (f) — or double-click a resource"))
                .clicked()
            {
                acts.push(Act::KeyIn(FocusArea::Detail, KeyCode::Char('f')));
            }
        });
        ui.add_space(4.0);
        let id = ui.id().with(("resource-sel", mid));
        let prev: Option<Option<usize>> = ui.data(|d| d.get_temp(id));
        let moved = prev.is_some_and(|p| p != selected);
        ui.data_mut(|d| d.insert_temp(id, selected));
        match depot_ui(ui, &depot, &ship_cargo(journal), selected, moved) {
            Some(ResourceAction::Select(i)) => acts.push(Act::Resource(i)),
            Some(ResourceAction::Nearest(i)) => {
                acts.push(Act::Resource(i));
                acts.push(Act::KeyIn(FocusArea::Detail, KeyCode::Char('f')));
            }
            None => {}
        }
    }

    /// A fleet carrier: the "mark as mine" prompt, or the owned carrier's stored inventory.
    fn carrier_ui(&self, ui: &mut Ui, mid: i64, journal: &JournalData, acts: &mut Vec<Act>) {
        let Some(cargo) = journal.carrier_cargo.get(&mid) else {
            widgets::section(ui, "Carrier", None);
            ui.horizontal(|ui| {
                widgets::hint(ui, "Mark this carrier as yours to track its inventory.");
                if widgets::hud_button(ui, "Mark as mine").on_hover_text("Track this carrier's inventory (m)").clicked() {
                    acts.push(Act::KeyIn(FocusArea::Detail, KeyCode::Char('m')));
                }
            });
            return;
        };
        let mut items: Vec<(&String, i32)> = cargo.iter().filter(|(_, &n)| n > 0).map(|(k, &n)| (k, n)).collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        widgets::section(ui, "My carrier inventory", Some(items.len()));
        ui.horizontal(|ui| {
            widgets::hint(ui, "Tallied from cargo transfers to and from this carrier.");
            if widgets::hud_button(ui, "Unmark as mine").on_hover_text("Stop tracking this carrier as yours (m)").clicked() {
                acts.push(Act::KeyIn(FocusArea::Detail, KeyCode::Char('m')));
            }
        });
        if items.is_empty() {
            widgets::empty_state(ui, "Empty — cargo transferred to this carrier will appear here.");
            return;
        }
        let w = ui.available_width().min(520.0);
        for (i, (name, count)) in items.iter().enumerate() {
            let display = journal.commodity_names.get(*name).cloned().unwrap_or_else(|| super::prettify_commodity(name));
            value_row(ui, w, i, &display, TEXT, &group_thousands(*count as i64), TEXT);
        }
        let total: i32 = items.iter().map(|(_, n)| n).sum();
        value_row(ui, w, 1, "Total", TEXT_DIM, &group_thousands(total as i64), accent_hi());
    }

    // ------------------------------------------------------------ market

    fn market_tab(&self, ui: &mut Ui, item: &ListItem<'_>, journal: &JournalData, todo: &TodoList, acts: &mut Vec<Act>) {
        match item {
            ListItem::Api(s) => {
                if s.commodities.is_empty() {
                    widgets::empty_state(ui, "No market data available.");
                } else {
                    let note = format!("Market data as of {}", fmt_ts(s.market_updated_at.as_ref()));
                    self.market_table(ui, s.market_id, &s.commodities, &note, true, journal, todo, acts);
                }
            }
            ListItem::Journal(s) => {
                let local = if !s.commodities.is_empty() {
                    Some(&s.commodities)
                } else {
                    journal.local_market.as_ref().filter(|(m, _)| *m == s.market_id).map(|(_, c)| c)
                };
                if let Some(commodities) = local {
                    self.market_table(ui, s.market_id, commodities, "Local market data (from Market.json)", false, journal, todo, acts);
                } else if docked_at(journal, s.market_id) {
                    widgets::empty_state(ui, "No market data for this station yet.");
                    ui.vertical_centered(|ui| widgets::hint(ui, "Open the Commodities Market panel in-game and the data will appear here automatically."));
                } else {
                    widgets::empty_state(ui, self.snapshot_data_msg(s));
                }
            }
        }
    }

    /// The sortable commodity table: click a header to sort (again to reverse), click a row
    /// to select it, double-click for its price history. Prices are coloured against the
    /// galactic mean; commodities a todo construction site still needs come first.
    #[allow(clippy::too_many_arguments)]
    fn market_table(
        &self,
        ui: &mut Ui,
        mid: i64,
        commodities: &[CommodityResponse],
        note: &str,
        selectable: bool,
        journal: &JournalData,
        todo: &TodoList,
        acts: &mut Vec<Act>,
    ) {
        let todo_needed = compute_todo_needed(&todo.construction_items, &journal.cargo);
        let effective = effective_todo_for_market(commodities, &todo_needed);
        let sorted = sorted_commodities(commodities, &effective, self.market_sort_col, self.market_sort_asc);
        let ship_cargo = ship_cargo(journal);
        let sel_row = (selectable && self.focus == FocusArea::Detail).then_some(self.market_selected_row);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(note).color(GREY).font(theme::body(14.5)));
            ui.label(egui::RichText::new("·").color(GREY));
            ui.label(egui::RichText::new("good deal").color(GREEN).font(theme::body(14.5)));
            ui.label(egui::RichText::new("bad deal").color(RED).font(theme::body(14.5)));
            if selectable {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let target = sorted.get(self.market_selected_row).map(|c| c.name.as_str());
                    let resp = widgets::hud_button_enabled(ui, "Price history", target.is_some())
                        .on_hover_text(format!("30-day price history of {} (h) — or double-click a row", target.unwrap_or("the selected commodity")));
                    if resp.clicked() {
                        acts.push(Act::KeyIn(FocusArea::Detail, KeyCode::Char('h')));
                    }
                });
            }
        });
        ui.add_space(4.0);

        let show_need = !effective.is_empty();
        let widths = [66.0, 58.0, 66.0, 58.0, 66.0, 76.0, 76.0];
        let need_w = 120.0;
        let min_w = 150.0 + widths.iter().sum::<f32>() + if show_need { need_w } else { 0.0 } + 9.0 * 8.0;

        let id = ui.id().with(("market-sel", mid));
        let prev: Option<Option<usize>> = ui.data(|d| d.get_temp(id));
        let scroll_to = sel_row.filter(|_| prev.is_some_and(|p| p != sel_row));
        ui.data_mut(|d| d.insert_temp(id, sel_row));

        let mut table = |ui: &mut Ui| {
            let right = Layout::right_to_left(Align::Center);
            soften_table_highlights(ui);
            let mut tb = TableBuilder::new(ui)
                .id_salt(("market", mid))
                .striped(true)
                .sense(Sense::click())
                .cell_layout(Layout::left_to_right(Align::Center))
                .auto_shrink([false, false])
                .column(Column::remainder().at_least(150.0).clip(true));
            for w in widths {
                tb = tb.column(Column::exact(w));
            }
            if show_need {
                tb = tb.column(Column::exact(need_w));
            }
            if let Some(r) = scroll_to {
                tb = tb.scroll_to_row(r, None);
            }
            tb.header(24.0, |mut h| {
                for (i, (col, label, hover)) in MARKET_COLS.iter().enumerate() {
                    let state = (*col == self.market_sort_col).then_some(self.market_sort_asc);
                    h.col(|ui| {
                        let resp = if i == 0 {
                            widgets::th_sort(ui, label, state)
                        } else {
                            ui.with_layout(right, |ui| widgets::th_sort(ui, label, state)).inner
                        };
                        if resp.on_hover_text(*hover).clicked() {
                            acts.push(Act::Sort(*col));
                        }
                    });
                }
                if show_need {
                    h.col(|ui| _ = widgets::th(ui, "Todo").on_hover_text("Still needed by construction sites on your todo list · tonnes in your hold"));
                }
            })
            .body(|body| {
                body.rows(24.0, sorted.len(), |mut row| {
                    let i = row.index();
                    let c = sorted[i];
                    let norm = normalize_commodity_name(&c.name);
                    let need = effective.get(&norm).copied();
                    let is_sel = sel_row == Some(i);
                    row.set_selected(is_sel);
                    let buy_col = price_color(c.buy_price, c.mean_price, false);
                    let sell_col = price_color(c.sell_price, c.mean_price, true);
                    row.col(|ui| {
                        if is_sel {
                            selection_marker(ui);
                        }
                        let (col, font) = if need.is_some() { (YELLOW, theme::body_semi(15.5)) } else { (TEXT, theme::body(15.5)) };
                        ui.label(egui::RichText::new(&c.name).color(col).font(font));
                    });
                    let cells = [
                        (price_text(c.buy_price), buy_col),
                        (diff_text(c.buy_price, c.mean_price), buy_col),
                        (price_text(c.sell_price), sell_col),
                        (diff_text(c.sell_price, c.mean_price), sell_col),
                        (group_thousands(c.mean_price as i64), GREY),
                        (group_thousands(c.stock as i64), TEXT),
                        (group_thousands(c.demand as i64), TEXT),
                    ];
                    for (text, col) in cells {
                        row.col(|ui| {
                            ui.with_layout(right, |ui| ui.label(egui::RichText::new(text).color(col).font(theme::body(15.5))));
                        });
                    }
                    if show_need {
                        row.col(|ui| {
                            if let Some(n) = need {
                                ui.label(egui::RichText::new(format!("★ {}", group_thousands(n as i64))).color(YELLOW).font(theme::body_semi(15.0)));
                                let in_ship = ship_cargo.get(&norm).copied().unwrap_or(0);
                                if in_ship > 0 {
                                    ui.label(egui::RichText::new(format!("ship {in_ship}")).color(theme::mix(YELLOW, BG, 0.3)).font(theme::body(14.0)));
                                }
                            }
                        });
                    }
                    if selectable {
                        let resp = row.response();
                        if resp.double_clicked() {
                            acts.push(Act::MarketRow(i));
                            acts.push(Act::KeyIn(FocusArea::Detail, KeyCode::Char('h')));
                        } else if resp.clicked() || resp.secondary_clicked() {
                            acts.push(Act::MarketRow(i));
                        }
                        resp.on_hover_text("Click to select · double-click: price history (h)").context_menu(|ui| {
                            if ui.button(format!("Price history of {}  (h)", c.name)).clicked() {
                                acts.push(Act::MarketRow(i));
                                acts.push(Act::KeyIn(FocusArea::Detail, KeyCode::Char('h')));
                                ui.close();
                            }
                        });
                    }
                });
            });
        };
        if ui.available_width() < min_w {
            egui::ScrollArea::horizontal()
                .id_salt(("market-h", mid))
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                ui.set_width(min_w);
                table(ui);
            });
        } else {
            table(ui);
        }
    }

    // ------------------------------------------------------------ price history

    fn price_history_ui(&self, ui: &mut Ui) {
        if self.commodity_history_name.is_empty() {
            widgets::empty_state(ui, "Go to the Market tab, select a commodity, then press h (or double-click it).");
            return;
        }
        widgets::section(ui, &format!("Price history · {}", self.commodity_history_name), None);
        if self.loading_history {
            widgets::empty_state(ui, "Loading price history…");
            return;
        }
        if self.history_buy_data.len() < 2 {
            let msg = match self.history_x_labels.first() {
                Some(e) if self.history_buy_data.is_empty() && e.starts_with("Error") => e.as_str(),
                _ if self.history_buy_data.is_empty() => "No price history recorded yet for this commodity.",
                _ => "Not enough data points to draw a chart yet.",
            };
            widgets::empty_state(ui, msg);
            return;
        }
        self.price_chart(ui);
    }

    /// Buy and sell price over time as two lines, with gridlines, axis labels, a legend and a
    /// hover read-out of the nearest sample.
    fn price_chart(&self, ui: &mut Ui) {
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let plot = Rect::from_min_max(pos2(rect.left() + 70.0, rect.top() + 34.0), pos2(rect.right() - 18.0, rect.bottom() - 34.0));
        if plot.width() < 40.0 || plot.height() < 40.0 {
            return;
        }
        let p = ui.painter_at(rect);
        let [x0, x1] = self.history_time_bounds;
        let [y0, y1] = self.history_y_bounds;
        let to_screen = |x: f64, y: f64| {
            pos2(
                plot.left() + ((x - x0) / (x1 - x0).max(1e-9)) as f32 * plot.width(),
                plot.bottom() - ((y - y0) / (y1 - y0).max(1e-9)) as f32 * plot.height(),
            )
        };

        p.rect_filled(plot, 0.0, PANEL);
        // Exact credit values: the terminal's short labels ("10k") collapse on narrow ranges.
        for i in 0..5 {
            let t = i as f32 / 4.0;
            let y = plot.bottom() - t * plot.height();
            let v = y0 + (y1 - y0) * t as f64;
            p.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], Stroke::new(1.0, alpha(accent(), 28)));
            widgets::txt(&p, pos2(plot.left() - 8.0, y), Align2::RIGHT_CENTER, &group_thousands(v.round() as i64), theme::body(14.0), TEXT_DIM, 0.0);
        }
        let nx = self.history_x_labels.len();
        for (i, label) in self.history_x_labels.iter().enumerate() {
            let t = if nx > 1 { i as f32 / (nx - 1) as f32 } else { 0.0 };
            let x = plot.left() + t * plot.width();
            p.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], Stroke::new(1.0, alpha(accent(), 18)));
            let align = if i == 0 { Align2::LEFT_TOP } else if i + 1 == nx { Align2::RIGHT_TOP } else { Align2::CENTER_TOP };
            widgets::txt(&p, pos2(x, plot.bottom() + 6.0), align, label, theme::body(14.0), TEXT_DIM, 0.0);
        }
        p.line_segment([plot.left_bottom(), plot.right_bottom()], Stroke::new(1.0, alpha(accent(), 120)));
        p.line_segment([plot.left_bottom(), plot.left_top()], Stroke::new(1.0, alpha(accent(), 120)));
        widgets::txt(&p, pos2(plot.left() - 8.0, rect.top() + 14.0), Align2::RIGHT_CENTER, "CR", theme::head(8.5), theme::accent_dim(), 1.2);

        let series = [(&self.history_buy_data, accent(), "Buy price"), (&self.history_sell_data, GREEN, "Sell price")];
        let clipped = p.with_clip_rect(plot.expand(3.0).intersect(p.clip_rect()));
        for (data, col, _) in series {
            // A zero price means "not traded": break the line there instead of diving to 0.
            for run in data.split(|(_, v)| *v <= 0.0) {
                let pts: Vec<_> = run.iter().map(|(x, y)| to_screen(*x, *y)).collect();
                if pts.len() >= 2 {
                    clipped.add(Shape::line(pts.clone(), Stroke::new(2.0, col)));
                }
                if data.len() <= 60 || pts.len() == 1 {
                    for pt in pts {
                        clipped.circle_filled(pt, 2.5, col);
                    }
                }
            }
        }

        let mut lx = plot.right() - 8.0;
        for (_, col, label) in series.iter().rev() {
            let r = widgets::txt(&p, pos2(lx, rect.top() + 14.0), Align2::RIGHT_CENTER, label, theme::body(14.5), TEXT, 0.0);
            p.rect_filled(Rect::from_center_size(pos2(r.left() - 12.0, r.center().y), vec2(14.0, 3.0)), 0.0, *col);
            lx = r.left() - 30.0;
        }

        if let Some(hover) = resp.hover_pos().filter(|h| plot.expand(4.0).contains(*h)) {
            let nearest = self
                .history_buy_data
                .iter()
                .enumerate()
                .min_by(|a, b| (to_screen(a.1 .0, y0).x - hover.x).abs().total_cmp(&(to_screen(b.1 .0, y0).x - hover.x).abs()))
                .map(|(i, _)| i);
            if let Some(i) = nearest {
                let (x, buy) = self.history_buy_data[i];
                let sell = self.history_sell_data.get(i).map(|s| s.1).unwrap_or(0.0);
                let sx = to_screen(x, y0).x;
                p.line_segment([pos2(sx, plot.top()), pos2(sx, plot.bottom())], Stroke::new(1.0, alpha(TEXT, 90)));
                for (v, col) in [(buy, accent()), (sell, GREEN)] {
                    if v > 0.0 {
                        clipped.circle_stroke(to_screen(x, v), 5.0, Stroke::new(2.0, col));
                    }
                }
                let fmt = |v: f64| if v > 0.0 { format!("{} cr", group_thousands(v as i64)) } else { "—".to_string() };
                resp.on_hover_text_at_pointer(format!("Buy  {}\nSell  {}", fmt(buy), fmt(sell)));
            }
        }
    }
}

// ---------------------------------------------------------------- helpers

/// Market ID, name, type and system of a list item.
fn item_basics<'a>(item: &'a ListItem<'_>) -> (i64, &'a str, &'a str, &'a str) {
    match item {
        ListItem::Api(s) => (s.market_id, s.name.as_str(), s.station_type.as_deref().unwrap_or(""), s.system_name.as_str()),
        ListItem::Journal(s) => (s.market_id, s.name.as_str(), s.station_type.as_str(), s.system_name.as_str()),
    }
}

/// Whether the commander is docked at this market right now.
fn docked_at(journal: &JournalData, mid: i64) -> bool {
    journal.last_docked.as_ref().map(|(m, _, _, _)| *m == mid).unwrap_or(false)
}

/// Tonnes of each commodity in the ship's hold, keyed by normalised name.
fn ship_cargo(journal: &JournalData) -> HashMap<String, i32> {
    journal.cargo.iter().map(|item| (normalize_commodity_name(&item.name), item.count)).collect()
}

/// Turns a raw game symbol like `$government_Feudal;` into `Feudal`; plain text passes through.
fn clean_symbol(s: &str) -> String {
    match s.strip_prefix('$') {
        Some(rest) => {
            let rest = rest.trim_end_matches(';');
            rest.split_once('_').map(|(_, v)| v).unwrap_or(rest).replace('_', " ")
        }
        None => s.to_string(),
    }
}

/// Overview facts of a station from the API: (label, value, de-emphasised).
fn api_facts(s: &StationResponse, journal: &JournalData) -> Vec<(&'static str, String, bool)> {
    let mut v = vec![
        ("System", s.system_name.clone(), false),
        ("Type", s.station_type.clone().unwrap_or_else(|| "Unknown".into()), false),
        ("Market ID", s.market_id.to_string(), false),
    ];
    if let Some(d) = s.dist_from_star_ls.filter(|&d| d > 0.0) {
        v.push(("Distance", format!("{} Ls", group_thousands(d.round() as i64)), false));
    }
    let faction = s.faction_name.as_deref().or_else(|| {
        journal.visited_stations.iter().find(|j| j.market_id == s.market_id).map(|j| j.faction.as_str()).filter(|f| !f.is_empty())
    });
    if let Some(f) = faction {
        v.push(("Faction", f.to_string(), false));
    }
    if let Some(g) = &s.government {
        v.push(("Government", clean_symbol(g), false));
    }
    if let Some(e) = &s.economy {
        v.push(("Economy", clean_symbol(e), false));
    }
    if let Some(p) = &s.landing_pads {
        v.push(("Landing pads", format!("S {} · M {} · L {}", p.small, p.medium, p.large), false));
    }
    v.push(("Updated", fmt_ts(s.updated_at.as_ref()), true));
    v
}

/// Overview facts from the journal's visit snapshot: (label, value, de-emphasised).
fn journal_facts(s: &StationData) -> Vec<(&'static str, String, bool)> {
    let mut v = vec![
        ("System", s.system_name.clone(), false),
        ("Type", s.station_type.clone(), false),
        ("Market ID", s.market_id.to_string(), false),
    ];
    if !s.faction.is_empty() {
        v.push(("Faction", s.faction.clone(), false));
    }
    if !s.government.is_empty() {
        v.push(("Government", clean_symbol(&s.government), false));
    }
    if !s.allegiance.is_empty() {
        v.push(("Allegiance", clean_symbol(&s.allegiance), false));
    }
    if !s.economy.is_empty() {
        let mut text = clean_symbol(&s.economy);
        if !s.secondary_economies.is_empty() {
            let parts: Vec<String> = s.secondary_economies.iter().map(|(n, p)| format!("{} ({:.0}%)", clean_symbol(n), p * 100.0)).collect();
            text = format!("{text} / {}", parts.join(", "));
        }
        v.push(("Economy", text, false));
    }
    if let Some((sm, md, lg)) = s.landing_pads {
        v.push(("Landing pads", format!("S {sm} · M {md} · L {lg}"), false));
    }
    if s.dist_from_star_ls > 0.0 {
        v.push(("Distance", format!("{} Ls", group_thousands(s.dist_from_star_ls.round() as i64)), false));
    }
    v
}

/// A small outlined tag in the body face, for service names.
fn chip(ui: &mut Ui, text: &str) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), theme::body(14.0), TEXT_DIM);
    let (r, _) = ui.allocate_exact_size(galley.size() + vec2(12.0, 4.0), Sense::hover());
    ui.painter().rect(r, 0.0, alpha(accent(), 14), Stroke::new(1.0, alpha(accent(), 70)), egui::StrokeKind::Inside);
    ui.painter().galley(r.center() - galley.size() / 2.0, galley, TEXT_DIM);
}

/// A striped name/value line `width` wide, value right-aligned.
fn value_row(ui: &mut Ui, width: f32, i: usize, name: &str, name_col: Color32, value: &str, value_col: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(width, 22.0), Sense::hover());
    if i.is_multiple_of(2) {
        ui.painter().rect_filled(r, 0.0, alpha(accent(), 10));
    }
    let p = ui.painter_at(r);
    widgets::txt(&p, pos2(r.left() + 6.0, r.center().y), Align2::LEFT_CENTER, name, theme::body(15.5), name_col, 0.0);
    widgets::txt(&p, pos2(r.right() - 6.0, r.center().y), Align2::RIGHT_CENTER, value, theme::body_semi(15.5), value_col, 0.0);
}

/// Outfitting grouped by category, with prices.
fn outfitting_ui(ui: &mut Ui, modules: &[ModuleResponse]) {
    if modules.is_empty() {
        widgets::empty_state(ui, "No outfitting data available.");
        return;
    }
    let w = ui.available_width().min(640.0);
    let mut i = 0;
    while i < modules.len() {
        let cat = modules[i].category.as_deref().unwrap_or("");
        let n = modules[i..].iter().take_while(|m| m.category.as_deref().unwrap_or("") == cat).count();
        if i > 0 {
            ui.add_space(8.0);
        }
        widgets::section(ui, if cat.is_empty() { "Other" } else { cat }, Some(n));
        for (j, m) in modules[i..i + n].iter().enumerate() {
            let cost = if m.cost > 0 { format!("{} cr", group_thousands(m.cost)) } else { "—".into() };
            value_row(ui, w, j, m.name.as_deref().unwrap_or(&m.id), TEXT, &cost, TEXT);
        }
        i += n;
    }
}

/// Ships for sale with their base value.
fn shipyard_ui(ui: &mut Ui, ships: &[ShipResponse]) {
    if ships.is_empty() {
        widgets::empty_state(ui, "No shipyard data available.");
        return;
    }
    widgets::section(ui, "Ships for sale", Some(ships.len()));
    let w = ui.available_width().min(640.0);
    for (i, s) in ships.iter().enumerate() {
        let val = if s.basevalue > 0 { format!("{} cr", group_thousands(s.basevalue)) } else { "—".into() };
        value_row(ui, w, i, s.name.as_deref().unwrap_or(&s.id), TEXT, &val, TEXT);
    }
}

fn price_text(price: i32) -> String {
    if price > 0 { group_thousands(price as i64) } else { "—".into() }
}

/// Signed difference to the galactic mean, blank when either price is missing.
fn diff_text(price: i32, mean: i32) -> String {
    match raw_diff(price, mean) {
        i32::MIN => String::new(),
        d if d > 0 => format!("+{}", group_thousands(d as i64)),
        d => group_thousands(d as i64),
    }
}

/// Green for a good deal against the mean, red for a bad one, neutral within ±2 %.
/// `higher_is_better` is true for sell prices.
fn price_color(price: i32, mean: i32, higher_is_better: bool) -> Color32 {
    if price <= 0 || mean <= 0 {
        return TEXT;
    }
    let ratio = price as f32 / mean as f32;
    let (good, bad) = if higher_is_better { (ratio >= 1.02, ratio <= 0.98) } else { (ratio <= 0.98, ratio >= 1.02) };
    if good {
        GREEN
    } else if bad {
        RED
    } else {
        TEXT
    }
}
