//! Desktop-window drawing for [`CarrierView`]: the carrier list and a server search on the
//! left; status, finances with the jump log, and the market for the selected carrier.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, Layout, Rect, Sense, Shape, Stroke, Ui};
use edcas_common::api::CarrierResponse;

use super::{ago, capitalise, countdown, credits, in_lockdown, service_name, thousands, CarrierView, OrderRow, Role, Snapshot};
use crate::api_client::ApiClient;
use crate::carrier_log::{self, CarrierRecord, OrderKind};
use crate::gui::theme::{self, accent, accent_dim, accent_hi, alpha, BG, BLUE, CYAN, GREEN, GREY, MAGENTA, PANEL, RED, TEXT, TEXT_DIM, YELLOW};
use crate::gui::widgets::{self, txt};
use crate::journal_reader::JournalData;
use crate::views::ViewEvent;

/// Height of a row in the carrier list.
const LIST_ROW: f32 = 52.0;
const ROW: f32 = 22.0;

#[derive(Default)]
pub(super) struct GuiState {
    /// Show orders the market says are filled or sold out.
    show_filled: bool,
}

enum Act {
    Select(i64),
    Search,
    Mark(i64, Option<Box<CarrierResponse>>),
    Refresh(i64),
    ShowFilled(bool),
}

impl CarrierView {
    pub fn ui(&mut self, ui: &mut Ui, api: &ApiClient, journal: &JournalData, marked: &HashSet<i64>) -> ViewEvent {
        let full = ui.available_rect_before_wrap();
        let gap = 10.0;
        let left_w = (full.width() * 0.2).clamp(250.0, 310.0);
        let listed = self.listed(marked);
        let list_h = (60.0 + listed.len().max(1) as f32 * LIST_ROW).min(full.height() * 0.45);
        let list_rect = Rect::from_min_size(full.min, vec2(left_w, list_h));
        let search_rect = Rect::from_min_max(pos2(full.left(), list_rect.bottom() + gap), pos2(list_rect.right(), full.bottom()));
        let rest = Rect::from_min_max(pos2(list_rect.right() + gap, full.top()), full.max);
        let col_w = (rest.width() - 2.0 * gap) / 3.0;
        let status_rect = Rect::from_min_size(rest.min, vec2(col_w, rest.height()));
        let mid_x = status_rect.right() + gap;
        let fin_h = (rest.height() * 0.56).round();
        let fin_rect = Rect::from_min_size(pos2(mid_x, rest.top()), vec2(col_w, fin_h));
        let jumps_rect = Rect::from_min_max(pos2(mid_x, fin_rect.bottom() + gap), pos2(mid_x + col_w, rest.bottom()));
        let market_rect = Rect::from_min_max(pos2(fin_rect.right() + gap, rest.top()), rest.max);

        let mut acts = Vec::new();
        let mut event = ViewEvent::None;
        self.list_panel(ui, list_rect, &listed, journal, &mut acts);
        self.search_panel(ui, search_rect, marked, &mut acts);

        let now = Utc::now();
        match self.snapshot(journal) {
            Some(snap) => {
                let role = listed.iter().find(|l| l.id == snap.id).map(|l| l.role).unwrap_or(Role::Marked);
                let title = listed.iter().find(|l| l.id == snap.id).map(|l| l.title.clone()).unwrap_or_default();
                status_panel(ui, status_rect, &snap, role, &title, journal, now, &mut acts);
                finance_panel(ui, fin_rect, &snap, now);
                jumps_panel(ui, jumps_rect, &snap, now);
                market_panel(ui, market_rect, &snap, journal, self.gui.show_filled, &mut acts);
            }
            None => {
                let msg = if self.scanner.is_none() {
                    "Set the journal folder in Settings to find your carrier."
                } else if !self.book.loaded {
                    "Reading your journals for carrier events…"
                } else {
                    "No carrier found. Open your carrier's management panel in-game once, or find a carrier on the left and mark it as yours."
                };
                widgets::hud_panel_at(ui, rest, "Carrier", "", |ui| widgets::empty_state(ui, msg));
            }
        }

        for act in acts {
            match act {
                Act::Select(id) => self.selected = Some(id),
                Act::Search => self.start_search(api),
                Act::Mark(id, data) => {
                    if let Some(d) = data {
                        let e = self.server.entry(id).or_default();
                        e.data = Some(*d);
                        e.requested = Some(std::time::Instant::now());
                    }
                    if !marked.contains(&id) {
                        self.selected = Some(id);
                    }
                    event = ViewEvent::ToggleMyCarrier(id);
                }
                Act::Refresh(id) => self.fetch_server(api, id, true),
                Act::ShowFilled(v) => self.gui.show_filled = v,
            }
        }
        ui.allocate_rect(full, Sense::hover());
        event
    }

    fn list_panel(&self, ui: &mut Ui, rect: Rect, listed: &[super::Listed], journal: &JournalData, acts: &mut Vec<Act>) {
        let right = if self.book.loaded || self.scanner.is_none() { format!("{}", listed.len()) } else { "reading…".into() };
        widgets::hud_panel_at(ui, rect, "Carriers", &right, |ui| {
            if listed.is_empty() {
                widgets::hint(ui, if self.book.loaded { "None in your journals yet." } else { "Reading journals…" });
                return;
            }
            egui::ScrollArea::vertical().id_salt("carrier_list").auto_shrink([false, false]).show(ui, |ui| {
                for l in listed {
                    let sel = Some(l.id) == self.selected;
                    let rec = self.book.get(l.id);
                    let place = rec.and_then(|r| r.location.as_ref()).map(|p| p.system.clone()).or_else(|| {
                        self.server.get(&l.id).and_then(|e| e.data.as_ref()).map(|s| s.system_name.clone())
                    });
                    let here = rec
                        .and_then(|r| r.location.as_ref())
                        .zip(journal.current_system.as_ref())
                        .is_some_and(|(p, s)| p.system_address == s.system_address);
                    let jumping = rec.is_some_and(|r| r.pending_jump.is_some());
                    let resp = widgets::selectable_row(ui, sel, LIST_ROW - 4.0, |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&l.title).font(theme::body_semi(16.0)).color(widgets::row_text_color(sel, TEXT)));
                                if let Some(c) = &l.callsign {
                                    if *c != l.title {
                                        ui.label(egui::RichText::new(c).font(theme::mono(12.5)).color(widgets::row_text_color(sel, TEXT_DIM)));
                                    }
                                }
                            });
                            let mut sub = l.role.label().to_uppercase();
                            if let Some(p) = &place {
                                sub = format!("{sub} · {p}");
                            }
                            ui.add(egui::Label::new(egui::RichText::new(sub).font(theme::body(12.5)).color(widgets::row_text_color(sel, TEXT_DIM))).truncate());
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if jumping {
                                ui.label(egui::RichText::new("⤴").color(widgets::row_text_color(sel, YELLOW))).on_hover_text("Jump scheduled");
                            }
                            if here {
                                ui.label(egui::RichText::new("●").color(widgets::row_text_color(sel, GREEN))).on_hover_text("In your current system");
                            }
                        });
                    });
                    if resp.clicked() {
                        acts.push(Act::Select(l.id));
                    }
                    ui.add_space(4.0);
                }
            });
        });
    }

    fn search_panel(&mut self, ui: &mut Ui, rect: Rect, marked: &HashSet<i64>, acts: &mut Vec<Act>) {
        let search = &mut self.search;
        let searching = self.searching;
        let results = &self.search_results;
        widgets::hud_panel_at(ui, rect, "Find a carrier", "server", |ui| {
            ui.horizontal(|ui| {
                let w = ui.available_width() - 70.0;
                let r = widgets::text_field(ui, search, "Name or callsign", w);
                if widgets::submitted(ui, &r) {
                    acts.push(Act::Search);
                }
                if widgets::hud_button_enabled(ui, "Find", search.trim().len() >= 2).clicked() {
                    acts.push(Act::Search);
                }
            });
            ui.add_space(6.0);
            if searching {
                widgets::hint(ui, "Searching…");
                return;
            }
            match results {
                None => {
                    widgets::hint(ui, "Search the carriers the server knows and mark yours; marked carriers join the list above.");
                }
                Some(Err(e)) => {
                    ui.label(egui::RichText::new(format!("Search failed: {e}")).color(RED).font(theme::body(14.0)));
                }
                Some(Ok(list)) if list.is_empty() => {
                    widgets::hint(ui, "No carrier matches.");
                }
                Some(Ok(list)) => {
                    egui::ScrollArea::vertical().id_salt("carrier_search").auto_shrink([false, false]).show(ui, |ui| {
                        for c in list {
                            let mine = marked.contains(&c.market_id);
                            ui.horizontal(|ui| {
                                ui.set_min_height(40.0);
                                ui.vertical(|ui| {
                                    ui.set_max_width(ui.available_width() - 78.0);
                                    let title = c.carrier_name.clone().unwrap_or_else(|| c.name.clone());
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&title).font(theme::body_semi(15.0)).color(TEXT));
                                        if title != c.name {
                                            ui.label(egui::RichText::new(&c.name).font(theme::mono(12.0)).color(TEXT_DIM));
                                        }
                                    });
                                    let when = c.updated_at.map(|t| format!(" · {}", ago(Utc::now(), t))).unwrap_or_default();
                                    ui.add(egui::Label::new(egui::RichText::new(format!("{}{when}", c.system_name)).font(theme::body(12.5)).color(TEXT_DIM)).truncate());
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let label = if mine { "Unmark" } else { "Mine" };
                                    let hover = if mine { "Remove from your carriers" } else { "Add to your carriers and track its inventory" };
                                    if widgets::hud_button(ui, label).on_hover_text(hover).clicked() {
                                        acts.push(Act::Mark(c.market_id, Some(Box::new(c.clone()))));
                                    }
                                });
                            });
                            ui.add_space(2.0);
                        }
                    });
                }
            }
        });
    }
}

// ---------------------------------------------------------------- status

#[allow(clippy::too_many_arguments)]
fn status_panel(ui: &mut Ui, rect: Rect, snap: &Snapshot, role: Role, title: &str, journal: &JournalData, now: DateTime<Utc>, acts: &mut Vec<Act>) {
    let rec = snap.rec;
    let right = rec.and_then(|r| r.stats_at).map(|t| format!("panel {}", ago(now, t))).unwrap_or_default();
    widgets::hud_panel_at(ui, rect, "Status", &right, |ui| {
        egui::ScrollArea::vertical().id_salt("carrier_status").auto_shrink([false, false]).show(ui, |ui| {
            ui.label(egui::RichText::new(title).font(theme::body_semi(22.0)).color(accent_hi()));
            ui.horizontal_wrapped(|ui| {
                if let Some(c) = snap.callsign() {
                    ui.label(egui::RichText::new(c).font(theme::mono(14.0)).color(TEXT_DIM));
                }
                widgets::badge(ui, role.label(), match role {
                    Role::Own => accent(),
                    Role::Squadron => CYAN,
                    Role::Marked => TEXT_DIM,
                });
                let here_addr = journal.current_system.as_ref().map(|s| s.system_address);
                if rec.and_then(|r| r.location.as_ref()).is_some_and(|l| Some(l.system_address) == here_addr) {
                    widgets::badge(ui, "In your system", GREEN);
                }
                if journal.last_docked.as_ref().is_some_and(|d| d.0 == snap.id) && journal.current_system.is_some() {
                    widgets::badge(ui, "Last docked", TEXT_DIM);
                }
                if rec.is_some_and(|r| r.pending_decommission) {
                    widgets::badge(ui, "Decommissioning", RED);
                }
            });
            if let Some(t) = rec.and_then(|r| r.decommission_at.filter(|_| r.pending_decommission)) {
                ui.label(egui::RichText::new(format!("Scrapped {} UTC unless cancelled", t.format("%Y-%m-%d %H:%M"))).color(RED).font(theme::body_semi(14.5)));
            }
            ui.add_space(6.0);

            // Location and the scheduled jump.
            widgets::section(ui, "Location", None);
            match snap.location() {
                Some((place, when, src)) => {
                    let system = snap.system_name().unwrap_or_else(|| place.clone());
                    ui.label(egui::RichText::new(&system).font(theme::body_semi(17.0)).color(TEXT));
                    if place != system {
                        ui.label(egui::RichText::new(&place).font(theme::body(14.5)).color(TEXT_DIM));
                    }
                    let mut note = format!("Since {} · from {src}", ago(now, when));
                    // The server's distance belongs to the system it last saw the carrier in.
                    let same = snap.server.is_some_and(|s| s.system_name == system);
                    if let Some(d) = snap.server.filter(|_| same).and_then(|s| s.dist_from_star_ls).filter(|d| *d > 0.0) {
                        note = format!("{d:.0} Ls from the star · {note}");
                    }
                    widgets::hint(ui, &note);
                }
                None => {
                    widgets::hint(ui, if snap.server_loading { "Asking the server…" } else { "Unknown" });
                }
            }
            if let Some(p) = rec.and_then(|r| r.pending_jump.as_ref()) {
                ui.add_space(6.0);
                let lock = in_lockdown(now, p.departure);
                let col = if lock { RED } else { YELLOW };
                let frame = egui::Frame::new().fill(alpha(col, 20)).stroke(Stroke::new(1.0, alpha(col, 140))).inner_margin(8.0);
                frame.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    widgets::caption(ui, "Jump scheduled", col);
                    ui.label(egui::RichText::new(p.to.body.as_deref().unwrap_or(&p.to.system)).font(theme::body_semi(16.0)).color(TEXT));
                    ui.label(egui::RichText::new(countdown(now, p.departure)).font(theme::body_semi(18.0)).color(col));
                    widgets::hint(ui, &format!("Departure {} UTC · requested {}", p.departure.format("%H:%M"), ago(now, p.requested)));
                });
            }

            if let Some(r) = rec {
                tritium(ui, r);
                space(ui, r);
                services(ui, r);
                access(ui, r);
                if !r.module_packs.is_empty() || !r.ship_packs.is_empty() {
                    widgets::section(ui, "Packs", Some(r.module_packs.len() + r.ship_packs.len()));
                    for (kind, p) in r.ship_packs.iter().map(|p| ("Ships", p)).chain(r.module_packs.iter().map(|p| ("Modules", p))) {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&p.theme).font(theme::body(14.5)).color(TEXT));
                            ui.label(egui::RichText::new(format!("{kind} · tier {}", p.tier)).font(theme::body(13.0)).color(TEXT_DIM));
                        });
                    }
                }
                if let Some(b) = &r.bought {
                    ui.add_space(6.0);
                    widgets::hint(ui, &format!("Bought {} in {} for {} CR", b.at.format("%Y-%m-%d"), b.system, credits(b.price)));
                }
            } else {
                server_services(ui, snap);
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if widgets::hud_button(ui, "Refresh server data").clicked() {
                    acts.push(Act::Refresh(snap.id));
                }
                if role == Role::Marked && widgets::hud_button(ui, "Unmark").on_hover_text("Remove from your carriers").clicked() {
                    acts.push(Act::Mark(snap.id, None));
                }
            });
            if snap.server_failed {
                widgets::hint(ui, "The server could not be reached.");
            }
        });
    });
}

fn tritium(ui: &mut Ui, r: &CarrierRecord) {
    let Some(fuel) = r.fuel else { return };
    widgets::section(ui, "Tritium", None);
    let frac = fuel as f32 / carrier_log::FUEL_CAPACITY as f32;
    let col = if fuel < 135 { RED } else if fuel < 400 { YELLOW } else { GREEN };
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{fuel}")).font(theme::body_semi(20.0)).color(col));
        ui.label(egui::RichText::new(format!("/ {} t in the depot", carrier_log::FUEL_CAPACITY)).font(theme::body(14.0)).color(TEXT_DIM));
    });
    widgets::bar(ui, ui.available_width(), frac, col);
    // The estimate's formula is fitted to fleet carriers; squadron carriers burn less.
    if r.kind != carrier_log::CarrierKind::Fleet {
        return;
    }
    let used = r.space.as_ref().map(|s| s.used()).unwrap_or(0);
    let jumps = carrier_log::max_range_jumps(used, fuel);
    let one = carrier_log::jump_fuel(carrier_log::MAX_JUMP_LY, used, fuel);
    widgets::hint(ui, &format!(
        "≈ {jumps} jump{} of {:.0} ly ({one} t each at the current load)",
        if jumps == 1 { "" } else { "s" },
        carrier_log::MAX_JUMP_LY
    ));
    if let Some((cur, max)) = r.jump_range.filter(|(c, m)| c < m) {
        widgets::hint(ui, &format!("Jump range {cur:.0} of {max:.0} ly"));
    }
}

/// Segments of the space bar: label, tonnes, colour.
fn space_parts(s: &carrier_log::SpaceUsage) -> [(&'static str, i32, Color32); 6] {
    [
        ("Crew & services", s.crew, CYAN),
        ("Module packs", s.module_packs, MAGENTA),
        ("Ship packs", s.ship_packs, BLUE),
        ("Cargo", s.cargo, accent()),
        ("Reserved for orders", s.reserved, YELLOW),
        ("Free", s.free, GREY),
    ]
}

fn space(ui: &mut Ui, r: &CarrierRecord) {
    let Some(s) = &r.space else { return };
    widgets::section(ui, "Space", None);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    let p = ui.painter();
    let total = s.total.max(1) as f32;
    let mut x = rect.left();
    for (_, t, col) in space_parts(s) {
        let w = t.max(0) as f32 / total * rect.width();
        if w > 0.0 {
            let fill = if col == GREY { alpha(col, 50) } else { col };
            p.rect_filled(Rect::from_min_max(pos2(x, rect.top()), pos2(x + w, rect.bottom())), 0.0, fill);
            x += w;
        }
    }
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, alpha(accent(), 90)), egui::StrokeKind::Inside);
    ui.add_space(4.0);
    for (label, t, col) in space_parts(s) {
        if t <= 0 && label != "Free" {
            continue;
        }
        legend_row(ui, col, label, &format!("{} t", thousands(t as i64)));
    }
    legend_row(ui, Color32::TRANSPARENT, "Capacity", &format!("{} t", thousands(s.total as i64)));
}

fn legend_row(ui: &mut Ui, col: Color32, label: &str, value: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 19.0), Sense::hover());
    let p = ui.painter();
    if col != Color32::TRANSPARENT {
        p.rect_filled(Rect::from_center_size(pos2(rect.left() + 5.0, rect.center().y), vec2(8.0, 8.0)), 0.0, col);
    }
    txt(p, pos2(rect.left() + 16.0, rect.center().y), Align2::LEFT_CENTER, label, theme::body(14.0), TEXT_DIM, 0.0);
    txt(p, pos2(rect.right(), rect.center().y), Align2::RIGHT_CENTER, value, theme::body_semi(14.0), TEXT, 0.0);
}

fn services(ui: &mut Ui, r: &CarrierRecord) {
    if r.crew.is_empty() {
        return;
    }
    let mut crew: Vec<_> = r.crew.iter().filter(|c| c.role != "Captain").collect();
    crew.sort_by_key(|c| (!c.activated, !c.enabled, service_name(&c.role).to_owned()));
    widgets::section(ui, "Services", Some(crew.iter().filter(|c| c.activated).count()));
    if let Some(cap) = r.crew.iter().find(|c| c.role == "Captain").and_then(|c| c.name.as_ref()) {
        widgets::hint(ui, &format!("Captain {cap}"));
    }
    for c in crew {
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
        let p = ui.painter();
        let (state, col) = match (c.activated, c.enabled) {
            (true, true) => ("ACTIVE", GREEN),
            (true, false) => ("PAUSED", YELLOW),
            _ => ("OFF", GREY),
        };
        let name_col = if c.activated { TEXT } else { GREY };
        txt(p, pos2(rect.left(), rect.center().y), Align2::LEFT_CENTER, service_name(&c.role), theme::body(14.5), name_col, 0.0);
        txt(p, pos2(rect.right(), rect.center().y), Align2::RIGHT_CENTER, state, theme::head(8.5), col, 1.2);
        if let Some(n) = &c.name {
            resp.on_hover_text(format!("Crew: {n}"));
        }
    }
}

fn access(ui: &mut Ui, r: &CarrierRecord) {
    let Some(a) = &r.docking_access else { return };
    widgets::section(ui, "Docking", None);
    let who = super::docking_label(a);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(who).font(theme::body_semi(15.0)).color(TEXT));
        if r.allow_notorious == Some(true) {
            widgets::badge(ui, "Notorious allowed", YELLOW);
        }
    });
}

fn server_services(ui: &mut Ui, snap: &Snapshot) {
    widgets::hint(ui, "Fuel, space, finances and crew show only for your own or your squadron's carrier.");
    let Some(s) = snap.server else { return };
    if let Some(p) = &s.landing_pads {
        widgets::section(ui, "Landing pads", None);
        widgets::hint(ui, &format!("{} large · {} medium · {} small", p.large, p.medium, p.small));
    }
    let skip = ["dock", "autodock", "contacts", "crewlounge", "flightcontroller", "stationoperations", "stationMenu", "carriermanagement", "livery", "socialspace", "engineer"];
    let svcs: Vec<&String> = s.services.iter().filter(|x| !skip.contains(&x.as_str())).collect();
    if !svcs.is_empty() {
        widgets::section(ui, "Services", Some(svcs.len()));
        ui.horizontal_wrapped(|ui| {
            for x in svcs {
                widgets::badge(ui, &capitalise(x), TEXT_DIM);
            }
        });
    }
}

// ---------------------------------------------------------------- finance

fn finance_panel(ui: &mut Ui, rect: Rect, snap: &Snapshot, now: DateTime<Utc>) {
    let rec = snap.rec;
    let right = rec.and_then(|r| r.finance_at).map(|t| ago(now, t)).unwrap_or_default();
    widgets::hud_panel_at(ui, rect, "Finance", &right, |ui| {
        let Some((r, f)) = rec.and_then(|r| Some((r, r.finance.as_ref()?))) else {
            widgets::empty_state(ui, "Finances appear once the carrier's management panel has been opened in-game.");
            return;
        };
        egui::ScrollArea::vertical().id_salt("carrier_finance").auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{} CR", thousands(f.balance))).font(theme::body_semi(22.0)).color(accent_hi()));
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(format!("Available {}", credits(f.available))).font(theme::body(14.5)).color(TEXT));
                ui.label(egui::RichText::new(format!("· reserve {} ({}%)", credits(f.reserve), f.reserve_percent)).font(theme::body(14.5)).color(TEXT_DIM));
            });
            ui.add_space(6.0);
            balance_chart(ui, &r.balance_history, &r.transfers);
            if !f.taxes.is_empty() {
                widgets::section(ui, "Tariffs", None);
                ui.horizontal_wrapped(|ui| {
                    for (svc, pct) in &f.taxes {
                        let col = if *pct == 0 { GREEN } else if *pct >= 50 { YELLOW } else { TEXT };
                        ui.label(egui::RichText::new(capitalise(svc)).font(theme::body(14.0)).color(TEXT_DIM));
                        ui.label(egui::RichText::new(format!("{pct}%")).font(theme::body_semi(14.0)).color(col));
                        ui.add_space(8.0);
                    }
                });
            }
            if !r.transfers.is_empty() {
                widgets::section(ui, "Bank transfers", Some(r.transfers.len()));
                for t in r.transfers.iter().rev().take(8) {
                    let (sign, col) = if t.amount >= 0 { ("+", GREEN) } else { ("", RED) };
                    legend_value(ui, &t.at.format("%Y-%m-%d").to_string(), &format!("{sign}{} CR", credits(t.amount)), col);
                }
            }
        });
    });
}

fn legend_value(ui: &mut Ui, label: &str, value: &str, col: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 19.0), Sense::hover());
    let p = ui.painter();
    txt(p, pos2(rect.left(), rect.center().y), Align2::LEFT_CENTER, label, theme::body(14.0), TEXT_DIM, 0.0);
    txt(p, pos2(rect.right(), rect.center().y), Align2::RIGHT_CENTER, value, theme::body_semi(14.0), col, 0.0);
}

/// Balance over time; deposits and withdrawals marked on the line.
fn balance_chart(ui: &mut Ui, points: &[(i64, i64)], transfers: &[carrier_log::BankTransfer]) {
    const H: f32 = 150.0;
    const LEFT: f32 = 48.0;
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, H), Sense::hover());
    let p = ui.painter_at(rect.expand(2.0));
    let plot = Rect::from_min_max(pos2(rect.left() + LEFT, rect.top() + 16.0), pos2(rect.right() - 6.0, rect.bottom() - 18.0));
    let acc = accent();
    p.rect_filled(plot, 0.0, alpha(BG, 160));
    txt(&p, pos2(rect.left(), rect.top() + 5.0), Align2::LEFT_CENTER, "BALANCE", theme::head(7.5), accent_dim(), 1.2);
    if points.len() < 2 {
        txt(&p, plot.center(), Align2::CENTER_CENTER, "Not enough readings yet", theme::body(14.0), TEXT_DIM, 0.0);
        p.rect_stroke(plot, 0.0, Stroke::new(1.0, alpha(acc, 70)), egui::StrokeKind::Inside);
        return;
    }
    let (t0, t1) = (points[0].0, points[points.len() - 1].0.max(points[0].0 + 1));
    let max = points.iter().map(|p| p.1).max().unwrap_or(1).max(1) as f64;
    let step = nice_step(max / 4.0);
    let y_max = (max / step).ceil() * step;
    let to = |t: i64, v: i64| {
        pos2(
            plot.left() + (t - t0) as f32 / (t1 - t0) as f32 * plot.width(),
            plot.bottom() - (v as f64 / y_max).clamp(0.0, 1.0) as f32 * plot.height(),
        )
    };
    let grid = Stroke::new(1.0, alpha(acc, 26));
    let font = theme::body(12.0);
    let mut v = 0.0;
    while v <= y_max + 1.0 {
        let y = plot.bottom() - (v / y_max) as f32 * plot.height();
        p.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], grid);
        txt(&p, pos2(plot.left() - 5.0, y), Align2::RIGHT_CENTER, &axis_credits(v), font.clone(), TEXT_DIM, 0.0);
        v += step;
    }
    // Date labels at both ends and the middle.
    for (t, align) in [(t0, Align2::LEFT_TOP), ((t0 + t1) / 2, Align2::CENTER_TOP), (t1, Align2::RIGHT_TOP)] {
        if let Some(d) = DateTime::from_timestamp(t, 0) {
            let fmt = if t1 - t0 > 200 * 86_400 { "%b %Y" } else { "%b %d" };
            txt(&p, pos2(to(t, 0).x, plot.bottom() + 3.0), align, &d.format(fmt).to_string(), font.clone(), TEXT_DIM, 0.0);
        }
    }
    let pts: Vec<_> = points.iter().map(|&(t, v)| to(t, v)).collect();
    p.add(Shape::line(pts.clone(), Stroke::new(1.8, acc)));
    for tr in transfers {
        let t = tr.at.timestamp();
        if t < t0 || t > t1 {
            continue;
        }
        let at = points.iter().take_while(|p| p.0 <= t).last().map(|p| p.1).unwrap_or(0);
        let c = to(t, at);
        let col = if tr.amount >= 0 { GREEN } else { RED };
        p.circle_filled(c, 2.6, col);
    }
    if let Some(last) = pts.last() {
        p.circle_filled(*last, 3.0, acc);
    }
    p.rect_stroke(plot, 0.0, Stroke::new(1.0, alpha(acc, 70)), egui::StrokeKind::Inside);

    let Some(hp) = resp.hover_pos().filter(|h| plot.contains(*h)) else { return };
    let t = t0 + ((hp.x - plot.left()) / plot.width() * (t1 - t0) as f32) as i64;
    let Some(&(pt, pv)) = points.iter().take_while(|p| p.0 <= t).last() else { return };
    p.line_segment([pos2(hp.x, plot.top()), pos2(hp.x, plot.bottom())], Stroke::new(1.0, alpha(TEXT, 90)));
    let when = DateTime::from_timestamp(pt, 0).map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
    let label = format!("{when}  {} CR", credits(pv));
    let size = widgets::text_size(ui, &label, theme::body_semi(13.5)) + vec2(14.0, 8.0);
    let mut at = pos2(hp.x + 8.0, plot.top() + 4.0);
    if at.x + size.x > plot.right() {
        at.x = hp.x - size.x - 8.0;
    }
    let tip = Rect::from_min_size(at, size);
    p.rect(tip, 0.0, PANEL, Stroke::new(1.0, acc), egui::StrokeKind::Inside);
    txt(&p, tip.left_center() + vec2(7.0, 0.0), Align2::LEFT_CENTER, &label, theme::body_semi(13.5), TEXT, 0.0);
}

/// An axis label: "40 B", "2.5 M", "500 k".
fn axis_credits(v: f64) -> String {
    let (x, unit) = if v >= 1e9 { (v / 1e9, " B") } else if v >= 1e6 { (v / 1e6, " M") } else if v >= 1e3 { (v / 1e3, " k") } else { (v, "") };
    let s = format!("{x:.1}");
    format!("{}{unit}", s.strip_suffix(".0").unwrap_or(&s))
}

/// 1, 2 or 5 times a power of ten, at least `x`.
fn nice_step(x: f64) -> f64 {
    let x = x.max(1.0);
    let mag = 10f64.powf(x.log10().floor());
    [1.0, 2.0, 5.0, 10.0].into_iter().map(|m| m * mag).find(|s| *s >= x).unwrap_or(10.0 * mag)
}

// ---------------------------------------------------------------- jumps

fn jumps_panel(ui: &mut Ui, rect: Rect, snap: &Snapshot, now: DateTime<Utc>) {
    let jumps = snap.rec.map(|r| r.jumps.as_slice()).unwrap_or(&[]);
    let right = if jumps.is_empty() { String::new() } else { format!("{} jumps", jumps.len()) };
    widgets::hud_panel_at(ui, rect, "Jump log", &right, |ui| {
        if jumps.is_empty() {
            widgets::empty_state(ui, "Jumps you schedule for this carrier will be listed here.");
            return;
        }
        let month = jumps.iter().filter(|j| now - j.at < chrono::TimeDelta::days(30)).count();
        widgets::hint(ui, &format!("{month} in the last 30 days · first {}", jumps[0].at.format("%Y-%m-%d")));
        ui.add_space(4.0);
        egui::ScrollArea::vertical().id_salt("carrier_jumps").auto_shrink([false, false]).show_rows(ui, 38.0, jumps.len(), |ui, range| {
            for i in range {
                let j = &jumps[jumps.len() - 1 - i];
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::hover());
                let p = ui.painter();
                if i % 2 == 1 {
                    p.rect_filled(r, 0.0, alpha(accent(), 8));
                }
                let date = if j.exact { j.at.format("%Y-%m-%d %H:%M").to_string() } else { format!("by {}", j.at.format("%Y-%m-%d")) };
                txt(p, pos2(r.left() + 4.0, r.top() + 10.0), Align2::LEFT_CENTER, &date, theme::body(12.5), TEXT_DIM, 0.0);
                let dest = j.body.as_deref().unwrap_or(&j.to);
                let line = galley_trunc(ui, &format!("→ {dest}"), r.width() - 8.0, theme::body_semi(14.5), TEXT);
                ui.painter().galley(pos2(r.left() + 4.0, r.top() + 19.0), line, TEXT);
                if let Some(from) = &j.from {
                    resp.on_hover_text(format!("From {from}"));
                }
            }
        });
    });
}

fn galley_trunc(ui: &Ui, text: &str, width: f32, font: egui::FontId, col: Color32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(text.to_owned(), egui::TextFormat { font_id: font, color: col, ..Default::default() });
    job.wrap = egui::text::TextWrapping { max_width: width, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    ui.painter().layout_job(job)
}

// ---------------------------------------------------------------- market

fn market_panel(ui: &mut Ui, rect: Rect, snap: &Snapshot, journal: &JournalData, show_filled: bool, acts: &mut Vec<Act>) {
    let orders = snap.orders();
    let open = orders.iter().filter(|o| o.live != Some(0)).count();
    widgets::hud_panel_at(ui, rect, "Market & inventory", &format!("{open} open orders"), |ui| {
        egui::ScrollArea::vertical().id_salt("carrier_market").auto_shrink([false, false]).show(ui, |ui| {
            widgets::section(ui, "Trade orders", Some(open));
            let source = match &snap.market {
                Some(m) if m.local => "Amounts left from the market you last opened in-game.".to_owned(),
                Some(m) => format!("Amounts left as of {} (EDDN).", m.at.map(|t| ago(Utc::now(), t)).unwrap_or_else(|| "?".into())),
                None => "No market reading yet: open the carrier's market in-game to see what is left.".to_owned(),
            };
            widgets::hint(ui, &source);
            let filled = orders.len() - open;
            if filled > 0 {
                let mut v = show_filled;
                if ui.checkbox(&mut v, egui::RichText::new(format!("Show {filled} filled / sold out")).font(theme::body(14.0)).color(TEXT_DIM)).changed() {
                    acts.push(Act::ShowFilled(v));
                }
            }
            if orders.is_empty() {
                widgets::empty_state(ui, "No orders.");
            }
            for o in orders.iter().filter(|o| show_filled || o.live != Some(0)) {
                order_row(ui, o);
            }

            if let Some(cargo) = journal.carrier_cargo.get(&snap.id).filter(|c| c.values().any(|n| *n != 0)) {
                let mut items: Vec<(&String, &i32)> = cargo.iter().filter(|(_, n)| **n > 0).collect();
                items.sort();
                widgets::section(ui, "Cargo moved aboard", Some(items.len()));
                widgets::hint(ui, "Tallied from cargo transfers in your journals.");
                for (name, n) in items {
                    let display = journal.commodity_names.get(name).cloned().unwrap_or_else(|| super::prettify(name));
                    legend_value(ui, &display, &thousands(*n as i64), TEXT);
                }
            }
            if let Some(space) = snap.rec.and_then(|r| r.space.as_ref()) {
                if space.cargo > 0 {
                    widgets::hint(ui, &format!("{} t of cargo aboard in total (carrier panel).", thousands(space.cargo as i64)));
                }
            }

            if let Some(mats) = journal.carrier_fc_materials.get(&snap.id).filter(|m| m.iter().any(|(_, n)| *n > 0)) {
                let items: Vec<_> = mats.iter().filter(|(_, n)| *n > 0).collect();
                widgets::section(ui, "Bartender", Some(items.len()));
                for (name, n) in items {
                    legend_value(ui, name, &thousands(*n as i64), TEXT);
                }
            }

            if let Some(s) = snap.server {
                if !s.ships.is_empty() {
                    widgets::section(ui, "Shipyard", Some(s.ships.len()));
                    ui.horizontal_wrapped(|ui| {
                        for sh in &s.ships {
                            widgets::badge(ui, sh.name.as_deref().unwrap_or(&sh.id), TEXT_DIM);
                        }
                    });
                }
                if !s.modules.is_empty() {
                    widgets::section(ui, "Outfitting", Some(s.modules.len()));
                    let mut cats: Vec<(String, usize)> = Vec::new();
                    for m in &s.modules {
                        let c = m.category.clone().unwrap_or_else(|| "Other".into());
                        match cats.iter_mut().find(|x| x.0 == c) {
                            Some(x) => x.1 += 1,
                            None => cats.push((c, 1)),
                        }
                    }
                    cats.sort();
                    for (c, n) in cats {
                        legend_value(ui, &capitalise(&c), &n.to_string(), TEXT);
                    }
                }
            }
        });
    });
}

fn order_row(ui: &mut Ui, o: &OrderRow) {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
    let p = ui.painter();
    let done = o.live == Some(0);
    let (tag, col) = match o.kind {
        OrderKind::Sell => ("SELL", YELLOW),
        OrderKind::Buy => ("BUY", GREEN),
    };
    let col = if done { GREY } else { col };
    let tr = Rect::from_min_size(pos2(r.left(), r.center().y - 8.0), vec2(36.0, 16.0));
    p.rect(tr, 0.0, alpha(col, 22), Stroke::new(1.0, col), egui::StrokeKind::Inside);
    txt(p, tr.center(), Align2::CENTER_CENTER, tag, theme::head(8.0), col, 1.0);
    let price = format!("{} CR", thousands(o.price));
    let amount = match o.live {
        Some(n) if n != o.quantity => format!("{} / {}", thousands(n as i64), thousands(o.quantity as i64)),
        _ => thousands(o.quantity as i64),
    };
    let pr = txt(p, pos2(r.right(), r.center().y), Align2::RIGHT_CENTER, &price, theme::body(13.5), if done { GREY } else { TEXT_DIM }, 0.0);
    let ar = txt(p, pos2(pr.left() - 12.0, r.center().y), Align2::RIGHT_CENTER, &amount, theme::body_semi(14.0), if done { GREY } else { TEXT }, 0.0);
    let name_w = ar.left() - tr.right() - 16.0;
    let g = galley_trunc(ui, &o.name, name_w.max(20.0), theme::body(14.5), if done { GREY } else { TEXT });
    ui.painter().galley(pos2(tr.right() + 8.0, r.center().y - g.size().y / 2.0), g, TEXT);
    let mut tip = format!("{} {} × {} at {price}", if o.kind == OrderKind::Sell { "Selling" } else { "Buying" }, thousands(o.quantity as i64), o.name);
    if let Some(n) = o.live {
        tip += &format!("\n{} left", thousands(n as i64));
    }
    if let Some(t) = o.set_at {
        tip += &format!("\nSet {}", t.format("%Y-%m-%d"));
    }
    resp.on_hover_text(tip);
}

#[cfg(test)]
mod tests {
    use super::{axis_credits, nice_step};

    #[test]
    fn chart_steps_are_round() {
        assert_eq!(nice_step(2.9e9), 5e9);
        assert_eq!(nice_step(1.9e8), 2e8);
        assert_eq!(nice_step(1e6), 1e6);
        assert_eq!(axis_credits(4e10), "40 B");
        assert_eq!(axis_credits(2.5e6), "2.5 M");
        assert_eq!(axis_credits(0.0), "0");
    }
}
