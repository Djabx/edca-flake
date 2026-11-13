//! Desktop-window drawing for [`PilotView`]: commander, live ship status, ranks, reputation,
//! powerplay and suit as HUD cards in two columns.
//!
//! Also home of the small drawing helpers the Commander and Workshop subviews share (cards that
//! size to their content, meter rows, grade pips, key-driven scrolling).

use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Ui};

use super::{
    expansions, flight_mode, format_credits, rank_name, status_flags, PilotView, Tone, COMBAT_RANKS,
    CQC_RANKS, EMPIRE_RANKS, EXOBIO_RANKS, EXPLORE_RANKS, FEDERATION_RANKS, SOLDIER_RANKS, TRADE_RANKS,
};
use crate::gui::theme::{self, accent, accent_dim, alpha, CYAN, GREEN, GREY, RED, TEXT, TEXT_DIM, YELLOW};
use crate::gui::widgets;
use crate::journal_reader::{JournalData, SuitData};

/// Engineering / suit-modification purple.
pub(in crate::views) const PURPLE: Color32 = Color32::from_rgb(180, 100, 255);
/// Height of one key-scrolled line (w/s move the page by this much).
const LINE_H: f32 = 24.0;
/// Width of the label column in cards, so grids and meter rows line up.
const LABEL_W: f32 = 112.0;

impl PilotView {
    pub fn ui(&mut self, ui: &mut Ui, journal: &JournalData) {
        // Labels must not take clicks meant for the rows they sit on.
        ui.style_mut().interaction.selectable_labels = false;
        let p = &journal.pilot;
        if p.name.is_empty() {
            widgets::hud_panel(ui, "Pilot", "", |ui| {
                widgets::empty_state(ui, "No pilot data — set the journal folder in Settings and start the game.");
            });
            return;
        }

        let mut lines = self.scroll as usize;
        let max = key_scroll(ui, "pilot-page", &mut lines, LINE_H, |ui, offset| {
            let mut area = egui::ScrollArea::vertical().id_salt("pilot-page").auto_shrink([false, false]);
            if let Some(o) = offset {
                area = area.vertical_scroll_offset(o);
            }
            area.show(ui, |ui| {
                ui.columns(2, |cols| {
                    let (left, right) = cols.split_at_mut(1);
                    self.left_column(&mut left[0], journal);
                    right_column(&mut right[0], journal);
                });
            })
        });
        self.scroll = lines.min(max) as u16;
    }

    fn left_column(&self, ui: &mut Ui, journal: &JournalData) {
        let p = &journal.pilot;
        card(ui, "cmdr", "Commander", "", |ui| {
            kv_grid(ui, "pilot-cmdr", |ui| {
                widgets::kv(ui, "Name", widgets::value(&p.name).font(theme::body_semi(17.0)));
                widgets::kv(ui, "Credits", widgets::value(format_credits(p.credits)).color(theme::accent_hi()));
                widgets::kv(ui, "Game mode", widgets::value(&p.game_mode));
                widgets::kv(ui, "Expansions", widgets::value(expansions(p)));
            });
        });

        if journal.status.valid {
            let title = if journal.status.on_foot() { "Suit status" } else { "Ship status" };
            card(ui, "status", title, "live", |ui| ship_status(ui, journal));
        }

        if !p.power.is_empty() {
            card(ui, "power", "Powerplay", "", |ui| {
                kv_grid(ui, "pilot-power", |ui| {
                    widgets::kv(ui, "Pledged to", widgets::value(&p.power));
                    widgets::kv(ui, "Merits", widgets::value(edcas_common::format::group_thousands(p.power_merits)).color(accent()));
                });
            });
        }
    }
}

fn right_column(ui: &mut Ui, journal: &JournalData) {
    let p = &journal.pilot;
    card(ui, "ranks", "Ranks", "progress to next", |ui| {
        let mut ranks: Vec<(&str, &'static [&'static str], u8, u8)> = vec![
            ("Combat", COMBAT_RANKS, p.rank_combat, p.progress_combat),
            ("Trade", TRADE_RANKS, p.rank_trade, p.progress_trade),
            ("Exploration", EXPLORE_RANKS, p.rank_explore, p.progress_explore),
            ("CQC", CQC_RANKS, p.rank_cqc, p.progress_cqc),
        ];
        if p.odyssey {
            ranks.push(("Soldier", SOLDIER_RANKS, p.rank_soldier, p.progress_soldier));
            ranks.push(("Exobiologist", EXOBIO_RANKS, p.rank_exobiologist, p.progress_exobiologist));
        }
        ranks.push(("Federation", FEDERATION_RANKS, p.rank_federation, p.progress_federation));
        ranks.push(("Empire", EMPIRE_RANKS, p.rank_empire, p.progress_empire));
        for (label, names, rank, prog) in ranks {
            let max_rank = names.len().saturating_sub(1) as u8;
            meter_row(
                ui,
                label,
                Some((rank_name(names, rank), TEXT)),
                prog as f32 / 100.0,
                accent(),
                &format!("{prog}%"),
                TEXT_DIM,
            )
            .on_hover_text(format!("{label}: rank {rank} of {max_rank}, {prog}% to the next"));
        }
    });

    card(ui, "rep", "Reputation", "superpowers", |ui| {
        for (label, value) in [
            ("Empire", p.reputation_empire),
            ("Federation", p.reputation_federation),
            ("Alliance", p.reputation_alliance),
        ] {
            reputation_row(ui, label, value);
        }
    });

    if p.odyssey {
        card(ui, "suit", "Suit", "", |ui| match &p.suit {
            None => widgets::hint(ui, "No suit data — requires a SuitLoadout event.").on_hover_text("Change or select a suit loadout in game"),
            Some(suit) => suit_card(ui, suit),
        });
    }
}

/// Live Status.json telemetry: suit life support on foot, ship gauges otherwise.
fn ship_status(ui: &mut Ui, journal: &JournalData) -> egui::Response {
    let s = &journal.status;
    let p = &journal.pilot;
    let resp = ui.scope(|ui| {
        if s.on_foot() {
            if let Some(h) = s.health {
                let col = if s.low_health() { RED } else { GREEN };
                meter_row(ui, "Health", None, h, col, &format!("{:.0}%", h * 100.0), TEXT);
            }
            if let Some(o) = s.oxygen {
                let col = if s.low_oxygen() { RED } else { GREEN };
                meter_row(ui, "Oxygen", None, o, col, &format!("{:.0}%", o * 100.0), TEXT);
            }
            kv_grid(ui, "pilot-foot", |ui| {
                if let Some(t) = s.temperature_k {
                    widgets::kv(ui, "Temperature", widgets::value(format!("{t:.0} K")));
                }
                if let Some(g) = s.gravity {
                    widgets::kv(ui, "Gravity", widgets::value(format!("{g:.2} g")));
                }
                if s.in_taxi() {
                    widgets::kv(ui, "State", widgets::value("Taxi"));
                }
                if !s.body_name.is_empty() {
                    widgets::kv(ui, "Location", widgets::value(&s.body_name));
                }
            });
            return;
        }

        if p.fuel_capacity > 0.0 {
            let col = if s.low_fuel() { RED } else { accent() };
            meter_row(ui, "Fuel", None, s.fuel_main / p.fuel_capacity, col, &format!("{:.1} / {:.0} t", s.fuel_main, p.fuel_capacity), TEXT);
            if s.scooping_fuel() {
                ui.horizontal(|ui| {
                    ui.add_space(LABEL_W);
                    widgets::badge(ui, "⛽ Scooping fuel", GREEN);
                });
            }
        }
        if p.hull_health > 0.0 {
            let col = if p.hull_health < 0.5 { RED } else { GREEN };
            meter_row(ui, "Hull", None, p.hull_health, col, &format!("{:.0}%", p.hull_health * 100.0), TEXT);
        }
        let cargo_frac = if p.cargo_capacity > 0 { s.cargo_tons / p.cargo_capacity as f32 } else { 0.0 };
        meter_row(ui, "Cargo", None, cargo_frac, CYAN, &format!("{:.0} / {} t", s.cargo_tons, p.cargo_capacity), TEXT);
        pips_row(ui, s.pips_sys, s.pips_eng, s.pips_wep);

        ui.add_space(4.0);
        kv_grid(ui, "pilot-ship", |ui| {
            let mut mode = flight_mode(s).to_string();
            if s.analysis_mode() {
                mode.push_str("  ·  Analysis");
            }
            widgets::kv(ui, "Mode", egui::RichText::new(format!("{mode}  ·  FG {}", s.firegroup + 1)).font(theme::body_semi(15.0)).color(TEXT));
            if !s.destination.is_empty() {
                widgets::kv(ui, "Destination", widgets::value(&s.destination));
            }
            if !s.body_name.is_empty() {
                let mut body = s.body_name.clone();
                if let Some(alt) = s.altitude {
                    body.push_str(&format!("  (alt {:.1} km)", alt / 1000.0));
                }
                widgets::kv(ui, "Body", widgets::value(body));
            }
            if !s.legal_state.is_empty() && s.legal_state != "Clean" {
                widgets::kv(ui, "Legal", widgets::value(&s.legal_state).color(RED));
            }
        });

        let flags = status_flags(s);
        if !flags.is_empty() {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                for (label, tone) in flags {
                    let col = match tone {
                        Tone::Normal => TEXT,
                        Tone::Dim => GREY,
                        Tone::Good => GREEN,
                        Tone::Warn => RED,
                    };
                    widgets::badge(ui, label, col);
                }
            });
        }
    });
    resp.response
}

fn suit_card(ui: &mut Ui, suit: &SuitData) -> egui::Response {
    ui.scope(|ui| {
        kv_grid(ui, "pilot-suit", |ui| {
            ui.label(egui::RichText::new("Type").color(CYAN).font(theme::body(15.0)));
            ui.horizontal(|ui| {
                ui.label(widgets::value(format!("{}  G{}", suit.suit_type, suit.grade)).color(accent()));
                grade_pips(ui, suit.grade, accent());
            });
            ui.end_row();
            if !suit.loadout_name.is_empty() {
                widgets::kv(ui, "Loadout", widgets::value(&suit.loadout_name));
            }
            if !suit.mods.is_empty() {
                ui.label(egui::RichText::new("Suit mods").color(CYAN).font(theme::body(15.0)));
                ui.add(egui::Label::new(egui::RichText::new(suit.mods.join(", ")).color(PURPLE).font(theme::body(15.0))).wrap());
                ui.end_row();
            }
        });
        ui.add_space(6.0);
        if suit.weapons.is_empty() {
            widgets::hint(ui, "No weapons in loadout.");
            return;
        }
        widgets::section(ui, "Weapons", Some(suit.weapons.len()));
        egui::Grid::new("pilot-weapons").num_columns(3).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
            widgets::th(ui, "Slot");
            widgets::th(ui, "Weapon");
            widgets::th(ui, "Grade");
            ui.end_row();
            for w in &suit.weapons {
                ui.label(egui::RichText::new(&w.slot).color(CYAN).font(theme::body(15.0)));
                ui.vertical(|ui| {
                    ui.label(widgets::value(&w.name));
                    if !w.mods.is_empty() {
                        ui.label(egui::RichText::new(w.mods.join(", ")).color(PURPLE).font(theme::body(13.5)));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("G{}", w.class)).color(PURPLE).font(theme::body_semi(15.0)));
                    grade_pips(ui, w.class, PURPLE);
                });
                ui.end_row();
            }
        });
    })
    .response
}

// ---------------------------------------------------------------- shared helpers

/// A HUD panel as tall as its content, placed at the cursor and followed by a gap. The height
/// is measured each frame and remembered, so the card settles on its content's size (one
/// extra frame is requested whenever it changes).
pub(in crate::views) fn card<R>(ui: &mut Ui, salt: &str, title: &str, right: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    let id = ui.id().with(("card", salt));
    let h: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(120.0);
    let rect = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), h));
    let (r, used) = widgets::hud_panel_at(ui, rect, title, right, |ui| {
        let r = add(ui);
        (r, ui.min_rect().height())
    });
    // Title strip (26) + top gap (8) + bottom margin (10) + a little breathing room.
    let need = (used + 26.0 + 8.0 + 10.0 + 6.0).ceil();
    if (need - h).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(id, need));
        ui.ctx().request_repaint();
    }
    ui.add_space(10.0);
    r
}

/// A two-column label/value grid whose label column lines up with [`meter_row`].
pub(in crate::views) fn kv_grid(ui: &mut Ui, id: &str, add: impl FnOnce(&mut Ui)) {
    egui::Grid::new(id).num_columns(2).min_col_width(LABEL_W - 8.0).spacing([8.0, 6.0]).show(ui, add);
}

/// One meter line: a cyan label, an optional value (e.g. a rank name), a segmented bar
/// filling `frac` and a right-aligned reading.
pub(in crate::views) fn meter_row(
    ui: &mut Ui,
    label: &str,
    value: Option<(&str, Color32)>,
    frac: f32,
    bar_col: Color32,
    reading: &str,
    reading_col: Color32,
) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let y = r.center().y;
    let p = ui.painter_at(r);
    widgets::txt(&p, pos2(r.left(), y), Align2::LEFT_CENTER, label, theme::body(15.0), CYAN, 0.0);
    let mut x = r.left() + LABEL_W;
    if let Some((v, col)) = value {
        let vw = (r.width() * 0.34).clamp(110.0, 170.0);
        let vp = ui.painter_at(Rect::from_min_max(pos2(x, r.top()), pos2(x + vw - 6.0, r.bottom())));
        widgets::txt(&vp, pos2(x, y), Align2::LEFT_CENTER, v, theme::body_semi(15.0), col, 0.0);
        x += vw;
    }
    let reading_w = 92.0;
    let bar = Rect::from_min_max(pos2(x, y - 4.0), pos2((r.right() - reading_w).max(x + 20.0), y + 4.0));
    widgets::seg_bar(&p, bar, frac, bar_col);
    widgets::txt(&p, pos2(r.right(), y), Align2::RIGHT_CENTER, reading, theme::body_semi(15.0), reading_col, 0.0);
    resp
}

/// A reputation line: the bar grows from its centre, green to the right for friendly
/// standing and red to the left for hostile (−100 … +100).
fn reputation_row(ui: &mut Ui, label: &str, value: f32) {
    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let y = r.center().y;
    let p = ui.painter_at(r);
    widgets::txt(&p, pos2(r.left(), y), Align2::LEFT_CENTER, label, theme::body(15.0), CYAN, 0.0);
    let bar = Rect::from_min_max(pos2(r.left() + LABEL_W, y - 4.0), pos2((r.right() - 92.0).max(r.left() + LABEL_W + 20.0), y + 4.0));
    let (seg, gap) = (4.0, 2.0);
    let n = (((bar.width() + gap) / (seg + gap)).floor() as usize).max(2) & !1;
    let mid = n / 2;
    let pos = ((value.clamp(-100.0, 100.0) + 100.0) / 200.0 * n as f32).round() as usize;
    let (lo, hi) = if pos >= mid { (mid, pos) } else { (pos, mid) };
    let col = if value >= 0.0 { GREEN } else { RED };
    for i in 0..n {
        let sx = bar.left() + i as f32 * (seg + gap);
        let sr = Rect::from_min_size(pos2(sx, bar.top()), vec2(seg, bar.height()));
        p.rect_filled(sr, 0.0, if i >= lo && i < hi { col } else { alpha(GREY, 40) });
    }
    let cx = bar.left() + mid as f32 * (seg + gap) - gap / 2.0;
    p.line_segment([pos2(cx, bar.top() - 3.0), pos2(cx, bar.bottom() + 3.0)], egui::Stroke::new(1.0, accent_dim()));
    widgets::txt(&p, pos2(r.right(), y), Align2::RIGHT_CENTER, &format!("{value:+.0}"), theme::body_semi(15.0), col, 0.0);
    resp.on_hover_text(format!("{label} reputation {value:+.1} (−100 hostile … +100 allied)"));
}

/// Power distributor pips: SYS / ENG / WEP, four pips each, half pips shown half lit.
fn pips_row(ui: &mut Ui, sys: u8, eng: u8, wep: u8) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let y = r.center().y;
    let p = ui.painter_at(r);
    widgets::txt(&p, pos2(r.left(), y), Align2::LEFT_CENTER, "Pips", theme::body(15.0), CYAN, 0.0);
    let mut x = r.left() + LABEL_W;
    let light = |c: Color32| theme::mix(c, Color32::WHITE, 0.3);
    for (name, half, col) in [("SYS", sys, light(theme::BLUE)), ("ENG", eng, light(YELLOW)), ("WEP", wep, light(RED))] {
        let lr = widgets::txt(&p, pos2(x, y), Align2::LEFT_CENTER, name, theme::head(9.0), col, 1.0);
        x = lr.right() + 6.0;
        for i in 0..4u8 {
            let pr = Rect::from_min_size(pos2(x, y - 5.0), vec2(9.0, 10.0));
            p.rect_filled(pr, 0.0, alpha(col, 30));
            let lit = half.saturating_sub(i * 2).min(2);
            if lit > 0 {
                let w = if lit == 2 { 9.0 } else { 4.5 };
                p.rect_filled(Rect::from_min_size(pr.min, vec2(w, 10.0)), 0.0, col);
            }
            x += 11.0;
        }
        x += 18.0;
    }
}

/// Five small diamonds, the first `filled` lit: an engineering or item grade.
pub(in crate::views) fn grade_pips(ui: &mut Ui, filled: u8, col: Color32) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(5.0 * 10.0, 14.0), Sense::hover());
    let p = ui.painter();
    for i in 0..5u8 {
        let c = pos2(r.left() + 5.0 + i as f32 * 10.0, r.center().y);
        let pts = vec![c + vec2(0.0, -4.0), c + vec2(4.0, 0.0), c + vec2(0.0, 4.0), c + vec2(-4.0, 0.0)];
        if i < filled {
            p.add(egui::Shape::convex_polygon(pts, col, egui::Stroke::NONE));
        } else {
            p.add(egui::Shape::closed_line(pts, egui::Stroke::new(1.0, alpha(col, 90))));
        }
    }
    resp
}

/// A vertical scroll area whose offset follows `*lines`, the line offset the view's w/s and
/// PgUp/PgDn keys move (one line = `line_h` points), and that writes wheel or drag scrolling
/// back into `*lines` so the keys continue from where the mouse left off. `show` builds the
/// area, applying the offset it is handed (set only when the keys moved it). Returns the
/// largest line offset the content allows, for the caller to clamp `*lines` to.
pub(in crate::views) fn key_scroll<R>(
    ui: &mut Ui,
    salt: &str,
    lines: &mut usize,
    line_h: f32,
    show: impl FnOnce(&mut Ui, Option<f32>) -> egui::scroll_area::ScrollAreaOutput<R>,
) -> usize {
    let id = ui.id().with(("key-scroll", salt));
    let (prev_lines, prev_off): (usize, f32) = ui.data(|d| d.get_temp(id)).unwrap_or((usize::MAX, 0.0));
    let keyed = *lines != prev_lines;
    let out = show(ui, keyed.then_some(*lines as f32 * line_h));
    let off = out.state.offset.y;
    if !keyed && (off - prev_off).abs() > 0.5 {
        *lines = (off / line_h).round() as usize;
    }
    ui.data_mut(|d| d.insert_temp(id, (*lines, off)));
    let max_off = (out.content_size.y - out.inner_rect.height()).max(0.0);
    (max_off / line_h).ceil() as usize
}
