//! Desktop-window drawing for [`TradeRoutesView`]: a filter bar over the route/loop table and
//! the selected entry's detail card. Tab still cycles Filter → List → Detail; clicking a panel
//! focuses it the same way.

use eframe::egui::{self, pos2, vec2, Align, Color32, Layout, Painter, Rect, RichText, Sense, Shape, Stroke, Ui};
use egui_extras::{Column, TableBuilder};

use super::{format_num, pad_str, FocusArea, RouteType, TradeRoutesView, SUPERPOWER_OPTIONS, TYPE_OPTIONS};
use crate::api_client::ApiClient;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::gui::theme::{self, accent, accent_dim, accent_hi, BG, GREY, TEXT, TEXT_DIM};
use crate::gui::widgets;
use crate::journal_reader::JournalData;
use crate::views::util::fmt_ts;
use crate::views::ViewEvent;

const FILTER_HEIGHT: f32 = 86.0;
const ROW_HEIGHT: f32 = 38.0;
/// Profit column: the widest figure plus room for the scroll bar.
const PROFIT_W: f32 = 112.0;

impl TradeRoutesView {
    pub fn ui(&mut self, ui: &mut Ui, api: &ApiClient, journal: &JournalData) -> ViewEvent {
        let mut event = ViewEvent::None;

        let filter_rect = Rect::from_min_size(ui.available_rect_before_wrap().min, vec2(ui.available_width(), FILTER_HEIGHT));
        let hint = if self.focus == FocusArea::Filter { "w/s field · ←/→ change" } else { "Pre-computed · 500 Ly · Large pad · 15 min refresh" };
        widgets::hud_panel_at(ui, filter_rect, "Filters", hint, |ui| self.filter_bar(ui, api, journal));
        if self.focus == FocusArea::Filter {
            focus_frame(ui.painter(), filter_rect);
        }
        ui.add_space(8.0);

        let rest = ui.available_rect_before_wrap();
        let split = rest.left() + (rest.width() * 0.57).round();
        let list_rect = Rect::from_min_max(rest.min, pos2(split - 5.0, rest.bottom()));
        let detail_rect = Rect::from_min_max(pos2(split + 5.0, rest.top()), rest.max);

        let count = self.current_count();
        let kind = TYPE_OPTIONS[self.type_idx];
        let list_hint = if self.focus == FocusArea::List { "w/s · c copy · r refresh".to_string() } else { format!("{count} shown") };
        let list_clicked = widgets::hud_panel_at(ui, list_rect, kind, &list_hint, |ui| self.list(ui, api, journal, &mut event));
        let detail_hint = if self.focus == FocusArea::Detail { "w/s select · c copy system" } else { "" };
        let detail_clicked = widgets::hud_panel_at(ui, detail_rect, "Detail", detail_hint, |ui| self.detail(ui, api, journal, &mut event));
        // A click on a panel's empty space focuses it, like Tab would.
        if list_clicked {
            self.focus = FocusArea::List;
        }
        if detail_clicked {
            self.focus = FocusArea::Detail;
        }
        match self.focus {
            FocusArea::List => focus_frame(ui.painter(), list_rect),
            FocusArea::Detail => focus_frame(ui.painter(), detail_rect),
            FocusArea::Filter => {}
        }
        event
    }

    /// Type, superpower and minimum supply/demand, each editable by mouse, plus refresh.
    fn filter_bar(&mut self, ui: &mut Ui, api: &ApiClient, journal: &JournalData) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 22.0;

            self.filter_field_ui(ui, 0, "Type", |s, ui| {
                let labels = [
                    format!("{}  {}", TYPE_OPTIONS[0], s.filtered_routes().len()),
                    format!("{}  {}", TYPE_OPTIONS[1], s.filtered_loops().len()),
                ];
                let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
                match widgets::tab_strip(ui, &labels, s.type_idx) {
                    Some(i) => {
                        if i != s.type_idx {
                            s.type_idx = i;
                            s.reset_list();
                        }
                        true
                    }
                    None => false,
                }
            });

            self.filter_field_ui(ui, 1, "Superpower", |s, ui| {
                let mut touched = false;
                egui::ComboBox::from_id_salt("trade-superpower")
                    .selected_text(RichText::new(SUPERPOWER_OPTIONS[s.superpower_idx]).font(theme::body_semi(15.0)).color(TEXT))
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for (i, name) in SUPERPOWER_OPTIONS.iter().enumerate() {
                            if ui.selectable_label(i == s.superpower_idx, *name).clicked() {
                                touched = true;
                                if i != s.superpower_idx {
                                    s.superpower_idx = i;
                                    s.reset_list();
                                }
                            }
                        }
                    })
                    .response
                    .on_hover_text("Only stations of this superpower (or of unknown allegiance)");
                touched
            });

            for (idx, label, value) in [(2, "Min supply", self.min_supply), (3, "Min demand", self.min_demand)] {
                self.filter_field_ui(ui, idx, label, |s, ui| {
                    let mut step = None;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if step_button(ui, "−", value > 0).on_hover_text("−1,000  (←)").clicked() {
                            step = Some(KeyCode::Left);
                        }
                        let (r, _) = ui.allocate_exact_size(vec2(72.0, 24.0), Sense::hover());
                        ui.painter().rect_filled(r, 0.0, ui.visuals().extreme_bg_color);
                        widgets::txt(ui.painter(), r.center(), egui::Align2::CENTER_CENTER, &format_num(value), theme::body_semi(16.0), TEXT, 0.0);
                        if step_button(ui, "+", true).on_hover_text("+1,000  (→)").clicked() {
                            step = Some(KeyCode::Right);
                        }
                    });
                    if let Some(code) = step {
                        s.focus = FocusArea::Filter;
                        s.filter_field = idx;
                        s.handle_key(&KeyEvent::from(code), api, journal);
                        true
                    } else {
                        false
                    }
                });
            }

            ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                let loading = self.route_rx.is_some() || self.loop_rx.is_some();
                if widgets::hud_button_enabled(ui, "Refresh", !loading).on_hover_text("Fetch the latest pre-computed routes and loops  (r)").clicked() {
                    self.fetch_all(api);
                }
            });
        });
    }

    /// One filter: caption over its control, outlined while the keyboard cursor is on it.
    /// `add` draws the control and returns whether the mouse used it.
    fn filter_field_ui(&mut self, ui: &mut Ui, idx: usize, label: &str, add: impl FnOnce(&mut Self, &mut Ui) -> bool) {
        let active = self.focus == FocusArea::Filter && self.filter_field == idx;
        let backdrop = ui.painter().add(Shape::Noop);
        let resp = ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            widgets::caption(ui, label, if active { accent_hi() } else { theme::CYAN });
            add(self, ui)
        });
        if active {
            let r = resp.response.rect.expand2(vec2(6.0, 3.0));
            ui.painter().set(backdrop, Shape::rect_filled(r, 0.0, theme::alpha(accent(), 30)));
            ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0, accent()), egui::StrokeKind::Outside);
        }
        if resp.inner {
            self.focus = FocusArea::Filter;
            self.filter_field = idx;
        }
    }

    /// The filtered routes or loops as a table, with the status line below. Returns whether
    /// the panel's empty space was clicked.
    fn list(&mut self, ui: &mut Ui, api: &ApiClient, journal: &JournalData, event: &mut ViewEvent) -> bool {
        let loading = self.route_rx.is_some() || self.loop_rx.is_some();
        let count = self.current_count();
        let full = ui.available_rect_before_wrap();
        let status_h = 22.0;
        let table_rect = Rect::from_min_max(full.min, pos2(full.right(), full.bottom() - status_h - 4.0));
        let status_rect = Rect::from_min_max(pos2(full.left(), full.bottom() - status_h), full.max);

        let bg = ui.interact(table_rect, ui.id().with("trade-list-bg"), Sense::click());
        let mut table_ui = ui.new_child(egui::UiBuilder::new().max_rect(table_rect).layout(Layout::top_down(Align::Min)));
        if count == 0 {
            if loading && self.routes.is_empty() && self.loops.is_empty() {
                table_ui.add_space(24.0);
                table_ui.vertical_centered(|ui| {
                    ui.add(egui::Spinner::new().size(22.0).color(accent()));
                    widgets::hint(ui, "Loading pre-computed trade data…");
                });
            } else if self.routes.is_empty() && self.loops.is_empty() {
                widgets::empty_state(&mut table_ui, "No data yet — the server cache may still be computing. Refresh (r) in a minute.");
            } else {
                widgets::empty_state(&mut table_ui, "Nothing matches these filters — lower Min supply / Min demand or pick another superpower.");
            }
        } else {
            match self.route_type() {
                RouteType::Routes => self.routes_table(&mut table_ui, api, journal, event),
                RouteType::Loops => self.loops_table(&mut table_ui, api, journal, event),
            }
        }

        let mut status_ui = ui.new_child(egui::UiBuilder::new().max_rect(status_rect).layout(Layout::left_to_right(Align::Center)));
        widgets::rule(status_ui.painter(), status_rect.left(), status_rect.right(), status_rect.top() - 2.0);
        if loading {
            status_ui.add(egui::Spinner::new().size(13.0).color(theme::CYAN));
        }
        // The status line also carries terminal key hints after a `|`; the status bar has those.
        let msg = self.status_msg.split('|').next().unwrap_or_default().trim();
        let col = crate::views::util::status_style(msg).fg.map(theme::from_ratatui).unwrap_or(TEXT_DIM);
        let col = if col == GREY { TEXT_DIM } else { col };
        status_ui.add(egui::Label::new(RichText::new(msg).font(theme::body(14.5)).color(col)).truncate());
        bg.clicked()
    }

    fn routes_table(&mut self, ui: &mut Ui, api: &ApiClient, journal: &JournalData, event: &mut ViewEvent) {
        let rows: Vec<RouteRow> = self
            .filtered_routes()
            .into_iter()
            .map(|r| RouteRow {
                commodity: title_case(&r.commodity),
                from: (r.from_station_name.clone(), r.from_system_name.clone()),
                to: (r.to_station_name.clone(), r.to_system_name.clone()),
                distance: r.distance_ly,
                profit: r.profit,
            })
            .collect();
        let selected = self.selected_idx;
        let lead_w = (ui.available_width() * 0.2).clamp(76.0, 140.0);
        let mut clicked = None;
        let mut copy = None;
        table(ui, "trade-routes", selected, |b| {
            b.column(Column::exact(lead_w).clip(true))
                .column(Column::remainder().clip(true))
                .column(Column::remainder().clip(true))
                .column(Column::exact(66.0))
                .column(Column::exact(PROFIT_W))
                .header(20.0, |mut h| {
                    h.col(|ui| _ = widgets::th(ui, "Commodity"));
                    h.col(|ui| _ = widgets::th(ui, "Buy at"));
                    h.col(|ui| _ = widgets::th(ui, "Sell at"));
                    h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Dist")));
                    h.col(|ui| right(ui, |ui| {
                        ui.add_space(10.0);
                        widgets::th(ui, "Profit/t");
                    }));
                })
                .body(|body| {
                    body.rows(ROW_HEIGHT, rows.len(), |mut row| {
                        let i = row.index();
                        let r = &rows[i];
                        let sel = i == selected;
                        row.set_selected(sel);
                        row.col(|ui| _ = ui.add(egui::Label::new(RichText::new(&r.commodity).font(theme::body_semi(16.0)).color(widgets::row_text_color(sel, TEXT))).truncate()));
                        row.col(|ui| two_line(ui, &r.from.0, &r.from.1, sel));
                        row.col(|ui| two_line(ui, &r.to.0, &r.to.1, sel));
                        row.col(|ui| right(ui, |ui| _ = ui.label(ly(r.distance, sel))));
                        row.col(|ui| right(ui, |ui| {
                            ui.add_space(10.0);
                            ui.label(credits(r.profit, 17.0, widgets::row_text_color(sel, accent_hi()), sel));
                        }));
                        row_actions(&row.response(), i, &mut clicked, &mut copy);
                    });
                });
        });
        self.after_table(clicked, copy, api, journal, event);
    }

    fn loops_table(&mut self, ui: &mut Ui, api: &ApiClient, journal: &JournalData, event: &mut ViewEvent) {
        let rows: Vec<LoopRow> = self
            .filtered_loops()
            .into_iter()
            .map(|l| LoopRow {
                out: title_case(&l.commodity_out),
                back: title_case(&l.commodity_back),
                a: (l.station_name_a.clone(), l.system_name_a.clone()),
                b: (l.station_name_b.clone(), l.system_name_b.clone()),
                distance: l.distance_ly,
                profit: l.total_profit,
            })
            .collect();
        let selected = self.selected_idx;
        let lead_w = (ui.available_width() * 0.2).clamp(76.0, 140.0);
        let mut clicked = None;
        let mut copy = None;
        table(ui, "trade-loops", selected, |b| {
            b.column(Column::exact(lead_w).clip(true))
                .column(Column::remainder().clip(true))
                .column(Column::remainder().clip(true))
                .column(Column::exact(66.0))
                .column(Column::exact(PROFIT_W))
                .header(20.0, |mut h| {
                    h.col(|ui| _ = widgets::th(ui, "Out / back"));
                    h.col(|ui| _ = widgets::th(ui, "Station A"));
                    h.col(|ui| _ = widgets::th(ui, "Station B"));
                    h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Dist")));
                    h.col(|ui| right(ui, |ui| {
                        ui.add_space(10.0);
                        widgets::th(ui, "Loop/t");
                    }));
                })
                .body(|body| {
                    body.rows(ROW_HEIGHT, rows.len(), |mut row| {
                        let i = row.index();
                        let r = &rows[i];
                        let sel = i == selected;
                        row.set_selected(sel);
                        row.col(|ui| two_line(ui, &format!("→ {}", r.out), &format!("← {}", r.back), sel));
                        row.col(|ui| two_line(ui, &r.a.0, &r.a.1, sel));
                        row.col(|ui| two_line(ui, &r.b.0, &r.b.1, sel));
                        row.col(|ui| right(ui, |ui| _ = ui.label(ly(r.distance, sel))));
                        row.col(|ui| right(ui, |ui| {
                            ui.add_space(10.0);
                            ui.label(credits(r.profit, 17.0, widgets::row_text_color(sel, accent_hi()), sel));
                        }));
                        row_actions(&row.response(), i, &mut clicked, &mut copy);
                    });
                });
        });
        self.after_table(clicked, copy, api, journal, event);
    }

    /// Applies a row click (select + focus the list) or a copy from a row's context menu.
    fn after_table(&mut self, clicked: Option<usize>, copy: Option<usize>, api: &ApiClient, journal: &JournalData, event: &mut ViewEvent) {
        if let Some(i) = clicked {
            self.selected_idx = i;
            self.focus = FocusArea::List;
        }
        if let Some(i) = copy {
            self.selected_idx = i;
            self.focus = FocusArea::List;
            *event = self.handle_key(&KeyEvent::from(KeyCode::Char('c')), api, journal);
        }
    }

    /// The selected route or loop. Returns whether the panel's empty space was clicked.
    fn detail(&mut self, ui: &mut Ui, api: &ApiClient, journal: &JournalData, event: &mut ViewEvent) -> bool {
        let bg = ui.interact(ui.available_rect_before_wrap(), ui.id().with("trade-detail-bg"), Sense::click());
        let mut pick = None;
        let mut copy = None;
        let mut copy_both = false;
        let focused = self.focus == FocusArea::Detail;
        let cursor = self.detail_cursor;
        egui::ScrollArea::vertical().id_salt("trade-detail").auto_shrink([false, false]).show(ui, |ui| {
            ui.style_mut().interaction.selectable_labels = false;
            match self.route_type() {
                RouteType::Routes => {
                    let Some(r) = self.filtered_routes().into_iter().nth(self.selected_idx) else {
                        widgets::empty_state(ui, "No item selected — pick a route on the left.");
                        return;
                    };
                    headline(ui, "Route", &title_case(&r.commodity), "Profit / unit", r.profit);

                    widgets::section(ui, "Buy from", None);
                    leg_row(ui, &r.from_system_name, pad_str(&r.from_max_pad), focused && cursor == 0, 0, &mut pick, &mut copy);
                    station_line(ui, &r.from_station_name, r.from_allegiance.as_deref());
                    leg_grid(ui, "trade-buy", &[
                        ("Buy price", credits(r.buy_price, 15.0, theme::GREEN, false)),
                        ("Supply", widgets::value(format!("{} t", format_num(r.supply))).into()),
                    ]);

                    ui.add_space(6.0);
                    widgets::section(ui, "Sell at", None);
                    leg_row(ui, &r.to_system_name, pad_str(&r.to_max_pad), focused && cursor == 1, 1, &mut pick, &mut copy);
                    station_line(ui, &r.to_station_name, r.to_allegiance.as_deref());
                    leg_grid(ui, "trade-sell", &[
                        ("Sell price", credits(r.sell_price, 15.0, theme::YELLOW, false)),
                        ("Demand", widgets::value(format!("{} t", format_num(r.demand))).into()),
                    ]);

                    ui.add_space(6.0);
                    widgets::section(ui, "Route", None);
                    leg_grid(ui, "trade-route", &[
                        ("Distance", widgets::value(format!("{:.2} Ly", r.distance_ly)).into()),
                        ("Cached", RichText::new(fmt_ts(r.cached_at.as_ref())).color(TEXT_DIM).font(theme::body(15.0)).into()),
                    ]);
                }
                RouteType::Loops => {
                    let Some(l) = self.filtered_loops().into_iter().nth(self.selected_idx) else {
                        widgets::empty_state(ui, "No item selected — pick a loop on the left.");
                        return;
                    };
                    let title = format!("{} ⇄ {}", title_case(&l.commodity_out), title_case(&l.commodity_back));
                    headline(ui, "Loop", &title, "Total profit / unit", l.total_profit);

                    widgets::section(ui, "Outbound →", None);
                    leg_row(ui, &format!("{}  →  {}", l.system_name_a, l.system_name_b), "", focused && cursor == 0, 0, &mut pick, &mut copy);
                    station_line(ui, &format!("{}  →  {}", l.station_name_a, l.station_name_b), None);
                    leg_grid(ui, "loop-out", &[
                        ("Commodity", widgets::value(title_case(&l.commodity_out)).into()),
                        ("Buy → sell", buy_sell(l.buy_price_out, l.sell_price_out, theme::GREEN)),
                        ("Profit", credits(l.profit_out, 15.0, accent(), false)),
                        ("Supply / demand", widgets::value(format!("{} t / {} t", format_num(l.supply_out), format_num(l.demand_out))).into()),
                    ]);

                    ui.add_space(6.0);
                    widgets::section(ui, "Return ←", None);
                    leg_row(ui, &format!("{}  →  {}", l.system_name_b, l.system_name_a), "", focused && cursor == 1, 1, &mut pick, &mut copy);
                    station_line(ui, &format!("{}  →  {}", l.station_name_b, l.station_name_a), None);
                    leg_grid(ui, "loop-back", &[
                        ("Commodity", widgets::value(title_case(&l.commodity_back)).into()),
                        ("Buy → sell", buy_sell(l.buy_price_back, l.sell_price_back, theme::YELLOW)),
                        ("Profit", credits(l.profit_back, 15.0, accent(), false)),
                        ("Supply / demand", widgets::value(format!("{} t / {} t", format_num(l.supply_back), format_num(l.demand_back))).into()),
                    ]);

                    ui.add_space(6.0);
                    widgets::section(ui, "Route", None);
                    leg_grid(ui, "loop-route", &[
                        ("Distance", widgets::value(format!("{:.2} Ly", l.distance_ly)).into()),
                        ("Pad", widgets::value(pad_str(&l.max_pad)).into()),
                        ("Cached", RichText::new(fmt_ts(l.cached_at.as_ref())).color(TEXT_DIM).font(theme::body(15.0)).into()),
                    ]);
                }
            }
            ui.add_space(10.0);
            if widgets::hud_button(ui, "Copy both systems").on_hover_text("Copy “A → B” to the clipboard  (c in the list)").clicked() {
                copy_both = true;
            }
        });

        if let Some(n) = pick {
            self.focus = FocusArea::Detail;
            self.detail_cursor = n;
        }
        if let Some(n) = copy {
            self.focus = FocusArea::Detail;
            self.detail_cursor = n;
            *event = self.handle_key(&KeyEvent::from(KeyCode::Char('c')), api, journal);
        }
        if copy_both {
            self.focus = FocusArea::List;
            *event = self.handle_key(&KeyEvent::from(KeyCode::Char('c')), api, journal);
        }
        bg.clicked()
    }
}

struct RouteRow {
    commodity: String,
    from: (String, String),
    to: (String, String),
    distance: f32,
    profit: i32,
}

struct LoopRow {
    out: String,
    back: String,
    a: (String, String),
    b: (String, String),
    distance: f32,
    profit: i32,
}

/// A striped, clickable table filling `ui`, with the selection drawn as a solid accent bar and
/// scrolled into view when it moved since the last frame.
fn table(ui: &mut Ui, salt: &str, selected: usize, build: impl FnOnce(TableBuilder<'_>)) {
    let id = ui.id().with(salt).with("last-selected");
    let moved = ui.data(|d| d.get_temp::<usize>(id)) != Some(selected);
    ui.data_mut(|d| d.insert_temp(id, selected));
    ui.scope(|ui| {
        ui.visuals_mut().selection.bg_fill = accent();
        ui.visuals_mut().selection.stroke.color = BG;
        ui.style_mut().interaction.selectable_labels = false;
        let mut b = TableBuilder::new(ui)
            .id_salt(salt)
            .striped(true)
            .sense(Sense::click())
            .auto_shrink([false, false])
            .cell_layout(Layout::left_to_right(Align::Center));
        if moved {
            b = b.scroll_to_row(selected, None);
        }
        build(b);
    });
}

/// Row click selects it; its context menu copies both systems.
fn row_actions(resp: &egui::Response, i: usize, clicked: &mut Option<usize>, copy: &mut Option<usize>) {
    if resp.clicked() {
        *clicked = Some(i);
    }
    resp.context_menu(|ui| {
        if ui.button("Copy both systems  (c)").clicked() {
            *copy = Some(i);
            ui.close();
        }
    });
}

/// Right-aligns a cell's content.
fn right(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.with_layout(Layout::right_to_left(Align::Center), add);
}

/// A station over its system, the second line dimmer.
fn two_line(ui: &mut Ui, top: &str, bottom: &str, sel: bool) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add_space(2.0);
        ui.add(egui::Label::new(RichText::new(top).font(theme::body_semi(15.5)).color(widgets::row_text_color(sel, TEXT))).truncate());
        ui.add(egui::Label::new(RichText::new(bottom).font(theme::body(14.0)).color(widgets::row_text_color(sel, TEXT_DIM))).truncate());
    });
}

/// `n cr` with the unit smaller and dimmer than the figure.
fn credits(n: i32, size: f32, col: Color32, sel: bool) -> egui::WidgetText {
    let mut job = widgets::job(&format_num(n), theme::body_semi(size), col, 0.0);
    let unit_col = if sel { BG } else { TEXT_DIM };
    job.append(" cr", 0.0, egui::TextFormat { font_id: theme::body(size * 0.8), color: unit_col, valign: Align::BOTTOM, ..Default::default() });
    job.into()
}

/// `buy → sell cr` for one loop leg.
fn buy_sell(buy: i32, sell: i32, col: Color32) -> egui::WidgetText {
    let mut job = widgets::job(&format_num(buy), theme::body_semi(15.0), col, 0.0);
    job.append("  →  ", 0.0, egui::TextFormat { font_id: theme::body(15.0), color: TEXT_DIM, ..Default::default() });
    job.append(&format_num(sell), 0.0, egui::TextFormat { font_id: theme::body_semi(15.0), color: col, ..Default::default() });
    job.append(" cr", 0.0, egui::TextFormat { font_id: theme::body(12.0), color: TEXT_DIM, valign: Align::BOTTOM, ..Default::default() });
    job.into()
}

fn ly(d: f32, sel: bool) -> RichText {
    RichText::new(format!("{d:.1} Ly")).font(theme::body(15.0)).color(widgets::row_text_color(sel, TEXT_DIM))
}

/// The detail card's head: kind caption and title on the left, the headline profit on the
/// right. The figure keeps its width; the title truncates.
fn headline(ui: &mut Ui, kind: &str, title: &str, figure_label: &str, figure: i32) {
    let label_job = widgets::job(&figure_label.to_uppercase(), theme::head(9.5), accent_dim(), 1.4);
    let figure_text = credits(figure, 26.0, accent_hi(), false);
    let label_w = ui.painter().layout_job(label_job.clone()).size().x;
    let figure_w = figure_text.clone().into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, egui::TextStyle::Body).size().x;
    ui.horizontal(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            ui.allocate_ui_with_layout(vec2(label_w.max(figure_w) + 2.0, 52.0), Layout::top_down(Align::Max), |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(label_job);
                ui.add(egui::Label::new(figure_text).wrap_mode(egui::TextWrapMode::Extend));
            });
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                widgets::caption(ui, kind, accent_dim());
                ui.add(egui::Label::new(RichText::new(title).font(theme::body_semi(24.0)).color(TEXT)).truncate());
            });
        });
    });
    ui.add_space(4.0);
}

/// A leg's system as a selectable row (the detail cursor) with its pad, and a copy button.
fn leg_row(ui: &mut Ui, system: &str, pad: &str, selected: bool, n: usize, pick: &mut Option<usize>, copy: &mut Option<usize>) {
    ui.horizontal(|ui| {
        let row_w = ui.available_width() - 64.0;
        let resp = ui
            .allocate_ui(vec2(row_w, 30.0), |ui| {
                widgets::selectable_row(ui, selected, 30.0, |ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !pad.is_empty() {
                            widgets::badge(ui, pad, if selected { BG } else { theme::CYAN });
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.add(egui::Label::new(RichText::new(system).font(theme::body_semi(17.0)).color(widgets::row_text_color(selected, TEXT))).truncate());
                        });
                    });
                })
            })
            .inner;
        if resp.on_hover_text("Select  (w/s in the detail panel)").clicked() {
            *pick = Some(n);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::hud_button(ui, "Copy").on_hover_text("Copy this system name  (c)").clicked() {
                *copy = Some(n);
            }
        });
    });
}

/// The station name under a leg's system, with its allegiance when known.
fn station_line(ui: &mut Ui, station: &str, allegiance: Option<&str>) {
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        ui.add(egui::Label::new(RichText::new(station).font(theme::body(15.0)).color(TEXT_DIM)).truncate());
        if let Some(a) = allegiance.filter(|a| !a.is_empty()) {
            ui.label(RichText::new(format!("· {a}")).font(theme::body(14.0)).color(GREY));
        }
    });
}

/// Label/value rows under a leg, indented below its station line.
fn leg_grid(ui: &mut Ui, id: &str, rows: &[(&str, egui::WidgetText)]) {
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        egui::Grid::new(id).num_columns(2).min_col_width(110.0).spacing([16.0, 4.0]).show(ui, |ui| {
            for (k, v) in rows {
                widgets::kv(ui, k, v.clone());
            }
        });
    });
}

/// A square −/+ button for the numeric filters (the display face draws these glyphs tiny).
fn step_button(ui: &mut Ui, glyph: &str, enabled: bool) -> egui::Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), sense);
    let acc = accent();
    let (fill, stroke, col) = if !enabled {
        (Color32::TRANSPARENT, GREY, GREY)
    } else if resp.is_pointer_button_down_on() {
        (acc, acc, BG)
    } else if resp.hovered() {
        (theme::alpha(acc, 60), acc, TEXT)
    } else {
        (theme::alpha(acc, 18), theme::alpha(acc, 140), accent_hi())
    };
    let p = ui.painter();
    p.rect(r, 0.0, fill, Stroke::new(1.0, stroke), egui::StrokeKind::Inside);
    widgets::txt(p, r.center(), egui::Align2::CENTER_CENTER, glyph, theme::body_semi(20.0), col, 0.0);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// The chamfered panel outline again, brighter, marking the panel Tab has focused.
fn focus_frame(p: &Painter, r: Rect) {
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

/// "tritium" → "Tritium": the API returns lower-case commodity keys.
fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}
