//! Desktop-window drawing for [`NewsView`]: the Galnet articles as a typeset reading column
//! (date, headline, paragraphs) centred in the panel, scrolled by wheel or w/s and PgUp/PgDn.

use eframe::egui::{self, text::LayoutJob, Align, Color32, FontId, TextFormat, Ui};

use super::{Article, NewsView};
use crate::gui::theme::{self, accent, accent_dim, accent_hi, CYAN, RED, TEXT, TEXT_DIM};
use crate::gui::widgets;
use crate::views::ViewEvent;

/// Widest the reading column gets (about 95 characters of body text).
const COLUMN_W: f32 = 760.0;
/// Body text size and its line height.
const BODY: f32 = 17.0;
const LEADING: f32 = 25.0;
/// Pixels one `scroll` step (w/s) moves: one body line.
const STEP: f32 = LEADING;

impl NewsView {
    pub fn ui(&mut self, ui: &mut Ui, api: &crate::api_client::ApiClient) -> ViewEvent {
        _ = api;
        let caption = if self.loading {
            "loading".to_string()
        } else if self.error.is_some() {
            "unavailable".to_string()
        } else {
            format!("{} articles", self.articles.len())
        };
        widgets::hud_panel(ui, "Galnet news", &caption, |ui| {
            if self.loading {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Spinner::new().size(22.0).color(accent()));
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Loading Galnet articles…").color(CYAN).font(theme::body_semi(16.0)));
                });
                return;
            }
            if let Some(error) = &self.error {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Galnet is unavailable").color(RED).font(theme::body_semi(18.0)));
                    ui.add_space(4.0);
                    ui.set_max_width(COLUMN_W);
                    ui.add(egui::Label::new(egui::RichText::new(format!("Error: {error}")).color(TEXT_DIM).font(theme::body(15.0))).wrap());
                });
                return;
            }
            if self.articles.is_empty() {
                widgets::empty_state(ui, "Galnet returned no articles.");
                return;
            }
            let articles = &self.articles;
            line_scroll(ui, "news-feed", &mut self.scroll, |ui| {
                let avail = ui.available_width();
                let col_w = avail.min(COLUMN_W);
                ui.horizontal_top(|ui| {
                    ui.add_space(((avail - col_w) / 2.0 - 8.0).max(0.0));
                    ui.vertical(|ui| {
                        ui.set_width(col_w);
                        ui.add_space(6.0);
                        widgets::caption(ui, "Elite Dangerous Commander Assistant System · Galnet", accent_dim());
                        ui.add_space(14.0);
                        for (i, article) in articles.iter().enumerate() {
                            if i > 0 {
                                ui.add_space(22.0);
                                let (r, _) = ui.allocate_exact_size(egui::vec2(col_w, 1.0), egui::Sense::hover());
                                widgets::rule(ui.painter(), r.left(), r.right(), r.center().y);
                                ui.add_space(22.0);
                            }
                            article_block(ui, article);
                        }
                        ui.add_space(24.0);
                    });
                });
            });
        });
        ViewEvent::None
    }
}

/// One article: date, headline, then its paragraphs (the lead set a little heavier).
fn article_block(ui: &mut Ui, article: &Article) {
    widgets::caption(ui, article.date.trim(), CYAN);
    ui.add_space(2.0);
    ui.add(egui::Label::new(egui::RichText::new(article.title.trim()).font(theme::body_semi(25.0)).color(accent_hi())).wrap());
    ui.add_space(8.0);
    let text = article.text.replace('\r', "");
    let paragraphs = text.split('\n').map(str::trim).filter(|p| !p.is_empty());
    for (i, para) in paragraphs.enumerate() {
        if i > 0 {
            ui.add_space(9.0);
        }
        let (font, color) = if i == 0 { (theme::body_semi(BODY), TEXT) } else { (theme::body(BODY), TEXT) };
        ui.add(egui::Label::new(paragraph(para, font, color)).wrap());
    }
}

/// A paragraph as a layout job with relaxed leading.
fn paragraph(text: &str, font: FontId, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat { font_id: font, color, line_height: Some(LEADING), valign: Align::Center, ..Default::default() },
    );
    job.wrap.max_width = f32::INFINITY;
    job
}

/// A vertical scroll area tied to `NewsView::scroll` (moved by w/s and PgUp/PgDn): a change
/// since the last frame scrolls to `line * STEP`; wheel scrolling writes the position back,
/// so the keys continue from where the wheel left off and stop at the end.
fn line_scroll<R>(ui: &mut Ui, salt: &str, line: &mut usize, add: impl FnOnce(&mut Ui) -> R) -> R {
    let id = egui::Id::new(("news-line", salt));
    let prev = ui.ctx().data(|d| d.get_temp::<usize>(id));
    let mut area = egui::ScrollArea::vertical().id_salt(salt).auto_shrink([false, false]);
    if prev != Some(*line) {
        area = area.vertical_scroll_offset(*line as f32 * STEP);
    }
    let out = area.show(ui, add);
    let max = (out.content_size.y - out.inner_rect.height()).max(0.0);
    *line = (out.state.offset.y.clamp(0.0, max) / STEP).round() as usize;
    ui.ctx().data_mut(|d| d.insert_temp(id, *line));
    out.inner
}
