//! Desktop-window drawing for [`SearchNearestView`]: the single-commodity search (two fields
//! with commodity suggestions over a results table) and the multi-commodity Supply Scout
//! (a summary over station cards ranked by coverage).

use eframe::egui::{self, pos2, vec2, Align, Color32, Id, Key, Layout, Modifiers, Rect, RichText, Sense, Stroke, Ui};
use egui_extras::{Column, TableBuilder};

use super::{ActiveField, SearchNearestView};
use crate::api_client::ApiClient;
use crate::event_shim::{KeyCode, KeyEvent};
use crate::gui::theme::{self, accent, accent_hi, BG, GREY, TEXT, TEXT_DIM};
use crate::gui::widgets;
use crate::views::ViewEvent;

const SEARCH_HEIGHT: f32 = 114.0;
const FIELD_WIDTH: f32 = 290.0;

impl SearchNearestView {
    pub fn ui(&mut self, ui: &mut Ui, api: &ApiClient) -> ViewEvent {
        // This view has no tab of its own; offer the way back that `q` gives the keyboard.
        let back = widgets::hud_button(ui, "◀ Back to Todo").on_hover_text("Back to the Todo tab  (q)").clicked();
        ui.add_space(6.0);
        let event = if self.multi_mode { self.multi_ui(ui, api) } else { self.single_ui(ui, api) };
        if back {
            ViewEvent::Back
        } else {
            event
        }
    }

    fn key(&mut self, code: KeyCode, api: &ApiClient) -> ViewEvent {
        self.handle_key(&KeyEvent::from(code), api)
    }

    // ------------------------------------------------------------ single commodity

    fn single_ui(&mut self, ui: &mut Ui, api: &ApiClient) -> ViewEvent {
        let mut event = ViewEvent::None;
        let top = Rect::from_min_size(ui.available_rect_before_wrap().min, vec2(ui.available_width(), SEARCH_HEIGHT));
        let hint = if self.editing { "↓/↑ suggestions · Tab complete · Enter search · Esc cancel" } else { "Enter edit · w/s results · c copy system" };
        widgets::hud_panel_at(ui, top, "Search nearest", hint, |ui| self.search_bar(ui, api, &mut event));
        ui.add_space(8.0);

        let rest = ui.available_rect_before_wrap();
        widgets::hud_panel_at(ui, rest, "Results", "10 nearest · no fleet carriers", |ui| self.results_table(ui, api, &mut event));
        event
    }

    /// The commodity and reference-system fields, kept in step with the view's edit mode:
    /// focusing a field is `editing`, Enter searches, Esc cancels, Tab completes / switches.
    fn search_bar(&mut self, ui: &mut Ui, api: &ApiClient, event: &mut ViewEvent) {
        let c_id = Id::new("search-nearest-commodity");
        let s_id = Id::new("search-nearest-system");
        let (c_had, s_had) = ui.memory(|m| (m.has_focus(c_id), m.has_focus(s_id)));

        // Keys the fields keep for us (see `field`): suggestion moves and Tab.
        if c_had || s_had {
            for (key, code) in [(Key::ArrowDown, KeyCode::Down), (Key::ArrowUp, KeyCode::Up), (Key::Tab, KeyCode::Tab)] {
                if ui.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                    self.key(code, api);
                    if code == KeyCode::Tab {
                        let next = if self.active_field == ActiveField::Commodity { c_id } else { s_id };
                        ui.memory_mut(|m| m.request_focus(next));
                    }
                }
            }
        }

        let mut c_rect = Rect::NOTHING;
        let mut search = false;
        let mut copy = false;
        let (c_resp, s_resp) = ui
            .horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                let c_resp = labelled(ui, "Commodity", |ui| field(ui, c_id, &mut self.commodity_input, "e.g. Steel", FIELD_WIDTH));
                c_rect = c_resp.rect;
                let s_resp = labelled(ui, "Reference system", |ui| field(ui, s_id, &mut self.system_input, "current system", FIELD_WIDTH));
                labelled(ui, "", |ui| {
                    let r = widgets::hud_button_enabled(ui, "Search", !self.is_loading).on_hover_text("Search  (Enter)");
                    search = r.clicked();
                    r
                });
                ui.add_space(8.0);
                labelled(ui, "Pad", |ui| {
                    let (label, tip) = pad_filter_label(self.pad_filter);
                    ui.add_space(3.0);
                    widgets::badge(ui, label, theme::CYAN).on_hover_text(tip)
                });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    labelled(ui, "", |ui| {
                        let r = widgets::hud_button_enabled(ui, "Copy system", !self.results.is_empty())
                            .on_hover_text("Copy the selected station's system name  (c)");
                        copy = r.clicked();
                        r
                    });
                });
                (c_resp, s_resp)
            })
            .inner;
        if c_resp.changed() {
            self.commodity_canonical.clear();
            self.suggestion_idx = None;
            self.update_suggestions();
        }

        let picked = self.suggestions_popup(ui, c_rect);

        let (c_has, s_has) = ui.memory(|m| (m.has_focus(c_id), m.has_focus(s_id)));
        let lost = c_resp.lost_focus() || s_resp.lost_focus();
        let (enter, esc) = ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
        if let Some(i) = picked {
            self.suggestion_idx = Some(i);
            self.editing = true;
            self.key(KeyCode::Enter, api);
        } else if lost && enter && self.editing {
            self.key(KeyCode::Enter, api);
        } else if lost && esc && self.editing {
            self.key(KeyCode::Esc, api);
        } else if c_has || s_has {
            let field = if c_has { ActiveField::Commodity } else { ActiveField::System };
            if field == ActiveField::System {
                self.suggestions.clear();
                self.suggestion_idx = None;
            }
            self.editing = true;
            self.active_field = field;
        } else if lost {
            self.editing = false;
            self.suggestions.clear();
            self.suggestion_idx = None;
        } else if self.editing {
            // Edit mode entered by keyboard (Enter): put the caret in the active field.
            let id = if self.active_field == ActiveField::Commodity { c_id } else { s_id };
            ui.memory_mut(|m| m.request_focus(id));
        }

        if search {
            self.editing = true;
            self.key(KeyCode::Enter, api);
        }
        if copy && !self.editing {
            *event = self.key(KeyCode::Char('c'), api);
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            status_line(ui, self.is_loading, &self.status_msg);
            if !self.commodity_canonical.is_empty() && self.commodity_canonical != self.commodity_input {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("searching as “{}”", self.commodity_canonical)).font(theme::body(14.0)).color(GREY));
                });
            }
        });
    }

    /// Commodity suggestions under the commodity field while it is being edited. Returns the
    /// one clicked, if any.
    fn suggestions_popup(&self, ui: &Ui, below: Rect) -> Option<usize> {
        if !self.editing || self.active_field != ActiveField::Commodity || self.suggestions.is_empty() {
            return None;
        }
        let mut picked = None;
        egui::Area::new(Id::new("search-nearest-suggestions"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(below.left(), below.bottom() + 2.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::NONE
                    .fill(theme::PANEL)
                    .stroke(Stroke::new(1.0, accent()))
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        ui.set_width(below.width() - 8.0);
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.style_mut().interaction.selectable_labels = false;
                        for (i, name) in self.suggestions.iter().enumerate() {
                            let sel = self.suggestion_idx == Some(i);
                            let resp = widgets::selectable_row(ui, sel, 24.0, |ui| {
                                ui.label(RichText::new(*name).font(theme::body(15.5)).color(widgets::row_text_color(sel, TEXT)));
                            });
                            if resp.clicked() {
                                picked = Some(i);
                            }
                        }
                    });
            });
        picked
    }

    fn results_table(&mut self, ui: &mut Ui, api: &ApiClient, event: &mut ViewEvent) {
        if self.results.is_empty() {
            if self.is_loading {
                loading(ui, "Searching…");
            } else {
                widgets::empty_state(ui, "No results. Enter a commodity and a reference system above, then press Enter or Search.");
            }
            return;
        }
        let selected = self.selected_idx;
        let moved = selection_moved(ui, "search-nearest-results", selected);
        let mut clicked = None;
        let mut copy = None;
        ui.scope(|ui| {
            ui.visuals_mut().selection.bg_fill = accent();
            ui.visuals_mut().selection.stroke.color = BG;
            ui.style_mut().interaction.selectable_labels = false;
            let mut b = TableBuilder::new(ui)
                .id_salt("search-nearest-results")
                .striped(true)
                .sense(Sense::click())
                .auto_shrink([false, false])
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::exact(78.0))
                .column(Column::remainder().clip(true))
                .column(Column::remainder().clip(true))
                .column(Column::exact(76.0))
                .column(Column::exact(76.0))
                .column(Column::exact(80.0))
                .column(Column::exact(52.0));
            if moved {
                b = b.scroll_to_row(selected, None);
            }
            b.header(20.0, |mut h| {
                h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Dist")));
                h.col(|ui| _ = widgets::th(ui, "System"));
                h.col(|ui| _ = widgets::th(ui, "Station"));
                h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Buy")));
                h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Sell")));
                h.col(|ui| right(ui, |ui| _ = widgets::th(ui, "Stock")));
                h.col(|ui| _ = widgets::th(ui, "Pad"));
            })
            .body(|body| {
                body.rows(36.0, self.results.len(), |mut row| {
                    let i = row.index();
                    let r = &self.results[i];
                    let sel = i == selected;
                    let c = |normal: Color32| widgets::row_text_color(sel, normal);
                    row.set_selected(sel);
                    row.col(|ui| right(ui, |ui| _ = ui.label(RichText::new(format!("{:.2} Ly", r.distance_ly)).font(theme::body_semi(16.0)).color(c(accent_hi())))));
                    row.col(|ui| _ = ui.add(egui::Label::new(RichText::new(r.system_name.trim()).font(theme::body_semi(16.0)).color(c(TEXT))).truncate()));
                    row.col(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.add(egui::Label::new(RichText::new(&r.station_name).font(theme::body_semi(15.0)).color(c(TEXT))).truncate());
                            ui.add(egui::Label::new(RichText::new(r.station_type.as_deref().unwrap_or("Unknown type")).font(theme::body(13.5)).color(c(TEXT_DIM))).truncate());
                        });
                    });
                    let buy = if r.buy_price > 0 { group(r.buy_price) } else { "—".into() };
                    row.col(|ui| right(ui, |ui| _ = ui.label(RichText::new(buy).font(theme::body(15.5)).color(c(theme::GREEN)))));
                    row.col(|ui| right(ui, |ui| _ = ui.label(RichText::new(group(r.sell_price)).font(theme::body(15.5)).color(c(theme::YELLOW)))));
                    row.col(|ui| right(ui, |ui| _ = ui.label(RichText::new(group(r.stock)).font(theme::body(15.5)).color(c(TEXT)))));
                    row.col(|ui| {
                        let pad = match (r.has_large_pad, r.has_medium_pad) {
                            (true, _) => "L",
                            (false, true) => "M",
                            _ => "S",
                        };
                        widgets::badge(ui, pad, if sel { BG } else { theme::CYAN });
                    });
                    let resp = row.response();
                    if resp.clicked() {
                        clicked = Some(i);
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Copy system name  (c)").clicked() {
                            copy = Some(i);
                            ui.close();
                        }
                    });
                });
            });
        });
        if let Some(i) = clicked {
            self.selected_idx = i;
        }
        if let Some(i) = copy {
            self.selected_idx = i;
            if !self.editing {
                *event = self.key(KeyCode::Char('c'), api);
            }
        }
    }

    // ------------------------------------------------------------ Supply Scout

    fn multi_ui(&mut self, ui: &mut Ui, api: &ApiClient) -> ViewEvent {
        let mut event = ViewEvent::None;
        let width = ui.available_width();
        let label_w = 160.0;
        let chip_font = theme::body_semi(15.5);
        let row_h = ui.ctx().fonts_mut(|f| f.row_height(&chip_font));
        let lines = wrapped_lines(ui, &self.multi_commodities, &chip_font, width - label_w - 48.0);
        let top_h = 26.0 + 8.0 + 24.0 + lines as f32 * (row_h + 4.0) + 8.0 + 22.0 + 12.0;
        let top = Rect::from_min_size(ui.available_rect_before_wrap().min, vec2(width, top_h));

        let mut copy = false;
        widgets::hud_panel_at(ui, top, "Supply Scout", "multi-item search nearest · q back", |ui| {
            ui.horizontal(|ui| {
                label_cell(ui, "Reference system", label_w);
                ui.label(widgets::value(if self.system_input.is_empty() { "—" } else { self.system_input.as_str() }));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    copy = widgets::hud_button_enabled(ui, "Copy system", !self.multi_results.is_empty())
                        .on_hover_text("Copy the selected station's system name  (c)")
                        .clicked();
                    let (label, tip) = pad_filter_label(self.pad_filter);
                    widgets::badge(ui, label, theme::CYAN).on_hover_text(tip);
                });
            });
            ui.horizontal_top(|ui| {
                label_cell(ui, &format!("Commodities ({})", self.multi_total), label_w);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
                    for (i, name) in self.multi_commodities.iter().enumerate() {
                        if i > 0 {
                            ui.label(RichText::new(SEP).font(chip_font.clone()).color(GREY));
                        }
                        ui.add(egui::Label::new(RichText::new(name).font(chip_font.clone()).color(TEXT)).wrap_mode(egui::TextWrapMode::Extend));
                    }
                });
            });
            ui.add_space(6.0);
            status_line(ui, self.is_loading, &self.status_msg);
        });
        if copy {
            event = self.key(KeyCode::Char('c'), api);
        }
        ui.add_space(8.0);

        let rest = ui.available_rect_before_wrap();
        let title = format!("Best coverage · {} commodities", self.multi_total);
        widgets::hud_panel_at(ui, rest, &title, "no fleet carriers", |ui| {
            if let Some(e) = self.multi_cards(ui, api) {
                event = e;
            }
        });
        event
    }

    /// One card per station: match count and bar, names, type, pad, distance, and the
    /// commodities it stocks. Returns the event of a copy from a card's menu.
    fn multi_cards(&mut self, ui: &mut Ui, api: &ApiClient) -> Option<ViewEvent> {
        if self.multi_results.is_empty() {
            if self.is_loading {
                loading(ui, "Searching stations…");
            } else {
                widgets::empty_state(ui, "No results. Try a larger reference system or check the commodity names.");
            }
            return None;
        }
        let selected = self.selected_idx;
        let moved = selection_moved(ui, "supply-scout", selected);
        let total = self.multi_total.max(1);
        let mut clicked = None;
        let mut copy = None;
        egui::ScrollArea::vertical().id_salt("supply-scout").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            ui.style_mut().interaction.selectable_labels = false;
            for (i, r) in self.multi_results.iter().enumerate() {
                let sel = i == selected;
                let c = |normal: Color32| widgets::row_text_color(sel, normal);
                let full = r.matched_count as usize >= self.multi_total;
                let resp = widgets::selectable_row(ui, sel, 62.0, |ui| {
                    ui.allocate_ui_with_layout(vec2(76.0, 50.0), Layout::top_down(Align::Min), |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let mut job = widgets::job(&r.matched_count.to_string(), theme::body_semi(22.0), c(if full { theme::GREEN } else { accent_hi() }), 0.0);
                        job.append(&format!(" / {}", self.multi_total), 0.0, egui::TextFormat { font_id: theme::body(15.0), color: c(TEXT_DIM), valign: Align::BOTTOM, ..Default::default() });
                        ui.label(job);
                        widgets::bar(ui, 64.0, r.matched_count as f32 / total as f32, if sel { BG } else if full { theme::GREEN } else { accent() });
                    });
                    ui.add_space(6.0);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(4.0);
                        ui.label(RichText::new(format!("{:.2} Ly", r.distance_ly)).font(theme::body_semi(17.0)).color(c(accent_hi())));
                        let pad = match (r.has_large_pad, r.has_medium_pad) {
                            (true, _) => "L+M",
                            (false, true) => "M",
                            _ => "S",
                        };
                        widgets::badge(ui, pad, if sel { BG } else { theme::CYAN });
                        ui.label(RichText::new(r.station_type.as_deref().unwrap_or("Unknown")).font(theme::body(14.5)).color(c(TEXT_DIM)));
                        ui.with_layout(Layout::top_down(Align::Min), |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.add_space(4.0);
                            let mut job = widgets::job(r.system_name.trim(), theme::body_semi(16.5), c(TEXT), 0.0);
                            job.append(&format!("   {}", r.station_name), 0.0, egui::TextFormat { font_id: theme::body(15.0), color: c(TEXT_DIM), ..Default::default() });
                            ui.add(egui::Label::new(job).truncate());
                            ui.add(egui::Label::new(RichText::new(r.matched_commodities.join("  ·  ")).font(theme::body(14.0)).color(c(GREY))).truncate())
                                .on_hover_text(r.matched_commodities.join("\n"));
                        });
                    });
                });
                if sel && moved {
                    resp.scroll_to_me(None);
                }
                if resp.clicked() {
                    clicked = Some(i);
                }
                resp.context_menu(|ui| {
                    if ui.button("Copy system name  (c)").clicked() {
                        copy = Some(i);
                        ui.close();
                    }
                });
            }
        });
        if let Some(i) = clicked {
            self.selected_idx = i;
        }
        copy.map(|i| {
            self.selected_idx = i;
            self.key(KeyCode::Char('c'), api)
        })
    }
}

/// A single-line field that keeps Tab and the arrow keys (the view uses them for
/// suggestions and switching fields), styled like [`widgets::text_field`].
fn field(ui: &mut Ui, id: Id, value: &mut String, placeholder: &str, width: f32) -> egui::Response {
    let resp = ui.add(
        egui::TextEdit::singleline(value)
            .id(id)
            .hint_text(RichText::new(placeholder).color(GREY))
            .font(theme::body(16.0))
            .text_color(TEXT)
            .desired_width(width)
            .margin(vec2(6.0, 3.0))
            .event_filter(egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: false }),
    );
    if ui.memory(|m| m.has_focus(id)) {
        ui.painter().rect_stroke(resp.rect, 0.0, Stroke::new(1.5, accent()), egui::StrokeKind::Outside);
    }
    resp
}

/// A caption over a control; returns the control's response.
fn labelled(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> egui::Response) -> egui::Response {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        widgets::caption(ui, label, theme::CYAN);
        add(ui)
    })
    .inner
}

/// A fixed-width row label in the field-label colour.
fn label_cell(ui: &mut Ui, text: &str, width: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(width, 20.0), Sense::hover());
    widgets::txt(ui.painter(), r.left_center(), egui::Align2::LEFT_CENTER, text, theme::body(15.0), theme::CYAN, 0.0);
}

/// Separator between commodity names in the Supply Scout summary.
const SEP: &str = "   ·   ";

/// How many lines `names` take when laid out like the summary does (whole names, wrapped at
/// `width`), so the panel can be sized before it is drawn.
fn wrapped_lines(ui: &Ui, names: &[String], font: &egui::FontId, width: f32) -> usize {
    let w = |s: &str| widgets::text_size(ui, s, font.clone()).x;
    let sep = w(SEP);
    let mut lines = 1;
    let mut x = 0.0;
    for (i, name) in names.iter().enumerate() {
        let item = w(name) + if i > 0 { sep } else { 0.0 };
        if x > 0.0 && x + item > width {
            lines += 1;
            x = w(name);
        } else {
            x += item;
        }
    }
    lines
}

/// The search status, coloured by `status_style` (searching cyan, errors red), with a
/// spinner while a request is out.
fn status_line(ui: &mut Ui, loading: bool, msg: &str) {
    ui.horizontal(|ui| {
        if loading {
            ui.add(egui::Spinner::new().size(14.0).color(theme::CYAN));
        }
        let text = if loading && !msg.to_lowercase().starts_with("search") { "Searching…" } else { msg };
        let col = crate::views::util::status_style(text).fg.map(theme::from_ratatui).unwrap_or(TEXT_DIM);
        let col = if col == GREY { TEXT_DIM } else { col };
        ui.add(egui::Label::new(RichText::new(text).font(theme::body(15.0)).color(col)).truncate());
    });
}

fn loading(ui: &mut Ui, text: &str) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.add(egui::Spinner::new().size(22.0).color(accent()));
        widgets::hint(ui, text);
    });
}

/// The pad filter (from the current ship) as a short label and its explanation.
fn pad_filter_label(pad: char) -> (&'static str, &'static str) {
    match pad {
        'L' => ("Large", "Only stations with a large pad (your ship needs one)"),
        'M' => ("Medium+", "Only stations with a medium or large pad"),
        _ => ("Any", "Stations with any pad size"),
    }
}

/// Whether `selected` differs from what this list showed last frame (to scroll it into view).
fn selection_moved(ui: &Ui, salt: &str, selected: usize) -> bool {
    let id = Id::new(salt).with("last-selected");
    let moved = ui.data(|d| d.get_temp::<usize>(id)) != Some(selected);
    ui.data_mut(|d| d.insert_temp(id, selected));
    moved
}

fn right(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.with_layout(Layout::right_to_left(Align::Center), add);
}

fn group(n: i32) -> String {
    edcas_common::format::group_thousands(n as i64)
}
