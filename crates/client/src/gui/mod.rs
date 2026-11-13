//! The desktop window: an eframe (egui on OpenGL) shell around the [`App`]. The shell owns
//! the chrome (tab bar, status bar, alert toasts, help); each view draws its own content
//! through its `ui` method.
//!
//! Rendering is on demand: a frame runs on input, when background work calls
//! [`crate::wake::wake`], and once a second for clocks and toast expiry. OpenGL (glow) is used
//! instead of wgpu because wgpu reserves ~200 MB of VRAM, which the game needs more.

#[cfg(debug_assertions)]
mod devscript;
pub mod genus_art;
mod keys;
pub mod theme;
pub mod widgets;

use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, Layout, Rect, Sense, Stroke, Ui};

use crate::alerts::Severity;
use crate::app::{App, AppView, TABS};
use theme::{accent, accent_dim, accent_hi, alpha, BG, TEXT, TEXT_DIM};
use widgets::txt;

/// The tab bar needs this much.
const MIN_WIDTH: f32 = 1060.0;

const LOGO_PNG: &[u8] = include_bytes!("../../../../assets/graphics/logo/edcas_128.png");

/// Opens the window and runs until it is closed.
pub fn run() -> anyhow::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("EDCAS")
        .with_app_id("edcas-client")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([MIN_WIDTH, 600.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(LOGO_PNG) {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    // Debug builds: EDCAS_GUI_SIZE=WxH opens at another size, to check layouts (see devscript).
    #[cfg(debug_assertions)]
    if let Some((w, h)) = std::env::var("EDCAS_GUI_SIZE").ok().and_then(|s| {
        let (w, h) = s.split_once('x')?;
        Some((w.parse::<f32>().ok()?, h.parse::<f32>().ok()?))
    }) {
        viewport = viewport.with_inner_size([w, h]);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "EDCAS",
        options,
        Box::new(|cc| {
            theme::install_fonts(&cc.egui_ctx);
            theme::apply_style(&cc.egui_ctx);
            // Installed before App::new() so the journal reader's first snapshot wakes us.
            let ctx = cc.egui_ctx.clone();
            crate::wake::install(move || ctx.request_repaint());
            Ok(Box::new(GuiApp {
                app: App::new(),
                accent: crate::theme::accent_rgb(),
                logo: None,
                #[cfg(debug_assertions)]
                script: devscript::DevScript::from_env(),
            }))
        }),
    )
    .map_err(|e| anyhow::anyhow!("window failed: {e}"))
}

struct GuiApp {
    app: App,
    /// Accent the current style was built from; restyle when Settings changes it.
    accent: (u8, u8, u8),
    logo: Option<egui::TextureHandle>,
    #[cfg(debug_assertions)]
    script: Option<devscript::DevScript>,
}

impl eframe::App for GuiApp {
    #[cfg(debug_assertions)]
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if let Some(s) = &mut self.script {
            s.inject(ctx, raw_input);
        }
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.tick();
        #[cfg(debug_assertions)]
        if let Some(s) = &mut self.script {
            s.collect_screenshot(ctx);
        }

        // Keyboard focus belongs to text fields only. Buttons and rows must not keep it, or
        // Space/Enter would both click them and reach the view's own shortcuts.
        if !ctx.text_edit_focused() {
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        for key in keys::collect(ctx) {
            self.app.handle_key(&key);
        }

        let acc = crate::theme::accent_rgb();
        if acc != self.accent {
            self.accent = acc;
            theme::apply_style(ctx);
        }
        // Clocks (tick countdowns, toast expiry) need a frame now and then even when idle.
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.logo.is_none() {
            if let Ok(icon) = eframe::icon_data::from_png_bytes(LOGO_PNG) {
                let img = egui::ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
                self.logo = Some(ctx.load_texture("edcas-logo", img, egui::TextureOptions::LINEAR));
            }
        }

        egui::Panel::top("tabs")
            .exact_size(52.0)
            .frame(egui::Frame::NONE.fill(BG))
            .show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("status")
            .exact_size(26.0)
            .frame(egui::Frame::NONE.fill(theme::PANEL))
            .show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG).inner_margin(egui::Margin { left: 12, right: 12, top: 4, bottom: 10 }))
            .show(ui, |ui| self.view(ui));

        self.toasts(&ctx);
        self.help(&ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        BG.to_normalized_gamma_f32()
    }
}

impl GuiApp {
    fn top_bar(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let p = ui.painter().clone();

        // The tabs need ~850 px. On narrow windows drop the current-system block first,
        // then the wordmark, keeping the logo.
        let tabs_w: f32 = TABS
            .iter()
            .map(|t| widgets::text_size(ui, &t.to_uppercase(), theme::head(10.0)).x + 1.6 * t.len() as f32 + 30.0)
            .sum();
        let system = self.app.journal.current_system.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "—".into());
        let system_w = widgets::text_size(ui, &system.to_uppercase(), theme::head_bold(11.0)).x.max(110.0) + 1.2 * system.len() as f32 + 40.0;
        let logo_w = 54.0;
        let wordmark_w = 200.0;
        let show_wordmark = full.width() >= logo_w + wordmark_w + tabs_w + 24.0;
        let show_system = full.width() >= logo_w + if show_wordmark { wordmark_w } else { 0.0 } + tabs_w + system_w + 24.0;

        let mut x = full.left() + 14.0;
        if let Some(logo) = &self.logo {
            let r = Rect::from_center_size(pos2(x + 15.0, full.center().y - 1.0), vec2(30.0, 30.0));
            p.image(logo.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            x += 40.0;
        }
        let mut tabs_left = x;
        if show_wordmark {
            let title = txt(&p, pos2(x, full.top() + 19.0), Align2::LEFT_CENTER, "EDCAS", theme::head_bold(19.0), accent(), 4.0);
            txt(&p, pos2(x + 1.0, full.top() + 36.0), Align2::LEFT_CENTER, "COMMANDER ASSISTANT SYSTEM", theme::head(6.6), accent_dim(), 1.6);
            tabs_left = title.right().max(x + 170.0) + 24.0;
        }

        // Tab bar, numbered like the 1–9 shortcuts.
        let mut tab_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_max(pos2(tabs_left, full.top() + 12.0), pos2(full.right() - 8.0, full.top() + 40.0)))
                .layout(Layout::left_to_right(Align::Center)),
        );
        tab_ui.spacing_mut().item_spacing.x = 4.0;
        let current = if self.app.view == AppView::SearchNearest { usize::MAX } else { self.app.view.index() };
        for (i, name) in TABS.iter().enumerate() {
            let resp = widgets::tab_button(&mut tab_ui, name, i == current, 28.0)
                .on_hover_text(match i {
                    0..=8 => format!("{name}  (key {})", i + 1),
                    9 => format!("{name}  (key 0)"),
                    _ => name.to_string(),
                });
            if resp.clicked() {
                if let Some(v) = AppView::from_index(i) {
                    self.app.select_view(v);
                }
            }
        }

        if show_system {
            txt(&p, pos2(full.right() - 16.0, full.top() + 17.0), Align2::RIGHT_CENTER, "CURRENT SYSTEM", theme::head(7.5), accent_dim(), 1.5);
            txt(&p, pos2(full.right() - 16.0, full.top() + 34.0), Align2::RIGHT_CENTER, &system.to_uppercase(), theme::head_bold(11.0), TEXT, 1.2);
        }

        p.line_segment([pos2(full.left(), full.bottom() - 0.5), pos2(full.right(), full.bottom() - 0.5)], Stroke::new(1.0, alpha(accent(), 70)));
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        ui.painter().line_segment([full.left_top(), full.right_top()], Stroke::new(1.0, alpha(accent(), 90)));
        let inner = full.shrink2(vec2(12.0, 0.0));

        // Build info sits on the right; the hints get what's left and drop the ones that
        // don't fit, rather than running underneath it.
        let mut right = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(Layout::right_to_left(Align::Center)));
        let latest = self.app.latest_build.lock().unwrap().clone();
        let outdated = latest.as_deref().map(|l| l != crate::build_info::BUILD).unwrap_or(false);
        if outdated {
            right
                .label(egui::RichText::new("UPDATE AVAILABLE").color(theme::RED).font(theme::head(8.5)))
                .on_hover_text(format!("Build {} is available at edcas.de", latest.unwrap_or_default()));
        }
        right.label(egui::RichText::new(format!("build {}", crate::build_info::BUILD)).color(TEXT_DIM).font(theme::body(13.0)));
        self.unsold_data(&mut right);
        let right_left = right.min_rect().left();

        let left_rect = Rect::from_min_max(inner.min, pos2(right_left - 16.0, inner.max.y));
        let mut row = ui.new_child(egui::UiBuilder::new().max_rect(left_rect).layout(Layout::left_to_right(Align::Center)));
        row.set_clip_rect(left_rect.intersect(ui.clip_rect()));
        row.spacing_mut().item_spacing.x = 6.0;
        if self.app.settings.journal_reader.journal_directory.trim().is_empty() {
            let r = row.add(egui::Label::new(egui::RichText::new("⚠ Journal folder not set — open Settings").color(theme::RED).font(theme::body_semi(14.0))).sense(Sense::click()));
            if r.clicked() {
                self.app.select_view(AppView::Settings);
            }
            row.add_space(10.0);
        }
        let mut hints: Vec<(&str, &str)> = vec![("?", "help")];
        if !self.app.active_toasts.is_empty() {
            hints.push(("esc", "dismiss alerts"));
        }
        hints.extend(self.app.view_hints().iter().copied());
        for (key, desc) in hints {
            let need = widgets::text_size(&row, &key.to_uppercase(), theme::head(8.5)).x
                + widgets::text_size(&row, desc, theme::body(13.5)).x
                + 26.0;
            if row.available_width() < need {
                break;
            }
            widgets::key_hint(&mut row, key, desc);
        }
    }

    /// Estimated value of the exploration and organic data not sold yet (see
    /// [`crate::cartography`]); a click opens the explorer.
    fn unsold_data(&mut self, ui: &mut Ui) {
        let unsold = self.app.unsold.data();
        let (total, bio) = (unsold.total(), unsold.bio_total());
        if !unsold.loaded || total <= 0 && bio <= 0 {
            return;
        }
        ui.add_space(14.0);
        let fmt = |font, color| egui::TextFormat { font_id: font, color, ..Default::default() };
        let plural = |n: usize| if n == 1 { "" } else { "s" };
        let mut job = egui::text::LayoutJob::default();
        if total > 0 {
            let systems = unsold.system_count();
            job.append("UNSOLD DATA  ", 0.0, fmt(theme::head(8.5), theme::accent_dim()));
            job.append(&edcas_common::format::credits(total), 0.0, fmt(theme::body_semi(14.0), theme::accent_hi()));
            job.append(&format!("  {systems} system{}", plural(systems)), 0.0, fmt(theme::body(13.0), TEXT_DIM));
        }
        if bio > 0 {
            let species = unsold.bio_count();
            job.append("BIO  ", if total > 0 { 14.0 } else { 0.0 }, fmt(theme::head(8.5), theme::accent_dim()));
            job.append(&edcas_common::format::credits(bio), 0.0, fmt(theme::body_semi(14.0), theme::GREEN));
            job.append(&format!("  {species} species"), 0.0, fmt(theme::body(13.0), TEXT_DIM));
        }
        let r = ui
            .add(egui::Label::new(job).sense(Sense::click()))
            .on_hover_text(
                "Estimated value of everything scanned and surface-mapped since it was last sold at \
                 Universal Cartographics, and of every organic species analysed since it was last \
                 sold at Vista Genomics (with the first-footfall bonus where the body was untrodden), \
                 across sessions. Dying loses it. Click to open the explorer.",
            );
        if r.clicked() {
            self.app.select_view(AppView::Explorer);
        }
    }

    fn view(&mut self, ui: &mut Ui) {
        let a = &mut self.app;
        let event = match a.view {
            AppView::News => a.news.ui(ui, &a.api),
            AppView::Commander => a.commander.ui(ui, &a.journal),
            AppView::Carrier => a.carrier.ui(ui, &a.api, &a.journal, &a.owned_carriers),
            AppView::Explorer => a.explorer.ui(ui, &a.settings, &a.journal),
            AppView::GalaxyMap => a.galaxy_map.ui(ui, &a.api, &a.settings, &a.journal),
            AppView::Workshop => a.workshop.ui(ui, &a.journal),
            AppView::Search => a.search.ui(ui, &a.api, &a.journal, &mut a.todo_view.todo, &mut a.stations, &mut a.factions),
            AppView::TradeRoutes => a.trade_routes.ui(ui, &a.api, &a.journal),
            AppView::Todo => a.todo_view.ui(ui, &a.journal),
            AppView::SearchNearest => a.search_nearest.ui(ui, &a.api),
            AppView::Settings => a.settings_view.ui(ui, &mut a.settings),
        };
        a.apply_view_event(event);
    }

    /// Alert toasts, newest first, stacked in the top-right corner. Click one to dismiss it.
    fn toasts(&mut self, ctx: &egui::Context) {
        if self.app.active_toasts.is_empty() {
            return;
        }
        let mut dismiss = None;
        egui::Area::new(egui::Id::new("toasts"))
            .anchor(Align2::RIGHT_TOP, vec2(-16.0, 62.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_width(360.0);
                for (i, t) in self.app.active_toasts.iter().enumerate().rev() {
                    let (col, label) = match t.alert.severity {
                        Severity::Info => (theme::CYAN, "INFO"),
                        Severity::Warning => (theme::YELLOW, "WARNING"),
                        Severity::Critical => (theme::RED, "ALERT"),
                    };
                    let resp = egui::Frame::NONE
                        .fill(theme::PANEL)
                        .stroke(Stroke::new(1.0, col))
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.set_width(336.0);
                            ui.horizontal(|ui| {
                                widgets::badge(ui, label, col);
                                ui.label(egui::RichText::new(&t.alert.title).color(col).font(theme::body_semi(16.0)));
                            });
                            ui.label(egui::RichText::new(&t.alert.body).color(TEXT).font(theme::body(15.0)));
                        })
                        .response
                        .interact(Sense::click())
                        .on_hover_text("Click to dismiss");
                    if resp.clicked() {
                        dismiss = Some(i);
                    }
                    ui.add_space(6.0);
                }
            });
        if let Some(i) = dismiss {
            self.app.active_toasts.remove(i);
        }
    }

    /// The keyboard reference (`?`), as a centred window over the content.
    fn help(&mut self, ctx: &egui::Context) {
        if !self.app.help_open {
            return;
        }
        let mut open = true;
        let screen = ctx.content_rect();
        egui::Window::new(egui::RichText::new("HELP · KEYBOARD REFERENCE").font(theme::head_bold(12.0)).color(accent_hi()))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size(vec2(screen.width() * 0.7, screen.height() * 0.75))
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    widgets::styled_lines(ui, &crate::views::about::build_lines());
                });
            });
        if !open {
            self.app.help_open = false;
        }
    }
}
