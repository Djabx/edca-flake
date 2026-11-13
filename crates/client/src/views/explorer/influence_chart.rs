//! The system panel's faction influence chart: one line per faction over the last weeks,
//! so a glance shows who is rising and who is slipping, with a hover read-out of every
//! faction's share on the day under the pointer.

use eframe::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Shape, Stroke, Ui};

use crate::gui::theme::{self, accent, accent_dim, alpha, BG, BLUE, CYAN, GREEN, MAGENTA, PANEL, RED, TEXT, TEXT_DIM, YELLOW};
use crate::gui::widgets::txt;

/// Chart height, axis labels included.
const HEIGHT: f32 = 170.0;
/// Room for the influence labels left of the plot and the dates below it.
const LEFT: f32 = 34.0;
const BOTTOM: f32 = 18.0;
const TOP: f32 = 18.0;

/// One faction's line: unix seconds and influence (0–1), oldest first.
pub(super) struct Series<'a> {
    pub name: &'a str,
    pub colour: Color32,
    pub points: Vec<(i64, f32)>,
}

impl Series<'_> {
    /// Influence at `t`: the last reading at or before it.
    fn at(&self, t: i64) -> Option<f32> {
        self.points.iter().take_while(|p| p.0 <= t).last().map(|p| p.1)
    }

    /// Change from the first reading to the last, in percentage points.
    pub fn change(&self) -> Option<f32> {
        match (self.points.first(), self.points.last()) {
            (Some(a), Some(b)) if b.0 > a.0 => Some((b.1 - a.1) * 100.0),
            _ => None,
        }
    }
}

/// Line colour of the `i`-th faction (by influence, largest first).
pub(super) fn colour(i: usize) -> Color32 {
    const MORE: [Color32; 3] = [Color32::from_rgb(170, 130, 255), Color32::from_rgb(120, 220, 200), TEXT_DIM];
    match i {
        0 => accent(),
        1 => CYAN,
        2 => GREEN,
        3 => MAGENTA,
        4 => YELLOW,
        5 => BLUE,
        6 => RED,
        n => MORE[(n - 7) % MORE.len()],
    }
}

/// Draws the chart over `[start, end]` (unix seconds). `highlight` thickens one line and
/// dims the rest; `note` replaces the lines with a message (loading, no data).
pub(super) fn chart(ui: &mut Ui, series: &[Series], start: i64, end: i64, highlight: Option<usize>, note: Option<&str>) {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, HEIGHT), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let p = ui.painter_at(rect.expand(2.0));
    let plot = Rect::from_min_max(pos2(rect.left() + LEFT, rect.top() + TOP), pos2(rect.right() - 6.0, rect.bottom() - BOTTOM));
    let acc = accent();
    p.rect_filled(plot, 0.0, alpha(BG, 160));
    let label_font = theme::body(12.5);
    txt(&p, pos2(rect.left(), rect.top() + 6.0), Align2::LEFT_CENTER, "INFLUENCE", theme::head(7.5), accent_dim(), 1.2);

    // Influence axis: 0 to the next 10 % above the largest reading.
    let top = series.iter().flat_map(|s| s.points.iter().map(|p| p.1)).fold(0.0f32, f32::max) * 100.0;
    let y_max = ((top / 10.0).ceil() * 10.0).clamp(10.0, 100.0);
    let step = if y_max <= 20.0 { 5.0 } else if y_max <= 50.0 { 10.0 } else { 20.0 };
    let span = (end - start).max(1) as f32;
    let to_screen = |t: i64, v: f32| {
        pos2(
            plot.left() + (t - start) as f32 / span * plot.width(),
            plot.bottom() - (v * 100.0 / y_max).clamp(0.0, 1.0) * plot.height(),
        )
    };
    let grid = Stroke::new(1.0, alpha(acc, 26));
    let mut v = 0.0;
    while v <= y_max + 0.01 {
        let y = plot.bottom() - v / y_max * plot.height();
        p.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], grid);
        txt(&p, pos2(plot.left() - 5.0, y), Align2::RIGHT_CENTER, &format!("{v:.0}%"), label_font.clone(), TEXT_DIM, 0.0);
        v += step;
    }

    // Date axis: whole days, as many as fit.
    let days = span / 86_400.0;
    let every = [1, 2, 3, 7, 14, 30].into_iter().find(|&d| days / d as f32 <= (plot.width() / 70.0).max(1.0)).unwrap_or(30);
    let mut t = (start / 86_400 + 1) * 86_400;
    while t < end {
        if (t / 86_400) % every == 0 {
            let x = to_screen(t, 0.0).x;
            p.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], grid);
            if let Some(d) = chrono::DateTime::from_timestamp(t, 0) {
                txt(&p, pos2(x, plot.bottom() + 3.0), Align2::CENTER_TOP, &d.format("%b %d").to_string(), label_font.clone(), TEXT_DIM, 0.0);
            }
        }
        t += 86_400;
    }

    if let Some(note) = note {
        txt(&p, plot.center(), Align2::CENTER_CENTER, note, theme::body(14.0), TEXT_DIM, 0.0);
    } else {
        // Lines back to front so the largest faction (and the highlighted one) end up on top.
        let mut order: Vec<usize> = (0..series.len()).rev().collect();
        if let Some(h) = highlight {
            order.retain(|&i| i != h);
            order.push(h);
        }
        for i in order {
            let s = &series[i];
            let (width, col) = match highlight {
                Some(h) if h == i => (2.6, s.colour),
                Some(_) => (1.2, alpha(s.colour, 90)),
                None => (1.7, s.colour),
            };
            let pts: Vec<_> = s.points.iter().map(|&(t, v)| to_screen(t, v)).collect();
            if pts.len() > 1 {
                p.add(Shape::line(pts.clone(), Stroke::new(width, col)));
            }
            if let Some(last) = pts.last() {
                p.circle_filled(*last, width + 0.8, col);
            }
        }
    }
    p.rect_stroke(plot, 0.0, Stroke::new(1.0, alpha(acc, 70)), egui::StrokeKind::Inside);

    // Hover: a line at the pointer's day and every faction's share then.
    let Some(hp) = resp.hover_pos().filter(|h| plot.contains(*h) && note.is_none()) else { return };
    let t = start + ((hp.x - plot.left()) / plot.width() * span) as i64;
    p.line_segment([pos2(hp.x, plot.top()), pos2(hp.x, plot.bottom())], Stroke::new(1.0, alpha(TEXT, 90)));
    let when = chrono::DateTime::from_timestamp(t, 0).map(|d| d.format("%b %d %H:%M UTC").to_string()).unwrap_or_default();
    let rows: Vec<(Color32, &str, f32)> = series.iter().filter_map(|s| Some((s.colour, s.name, s.at(t)?))).collect();
    if rows.is_empty() {
        return;
    }
    let font = theme::body(13.5);
    let row_h = 15.0;
    let pct_w = 44.0;
    let name_w = rows.iter().map(|r| p.layout_no_wrap(r.1.to_owned(), font.clone(), TEXT).size().x).fold(0.0, f32::max).min(plot.width() - pct_w - 40.0);
    let size = vec2(name_w + pct_w + 28.0, 22.0 + rows.len() as f32 * row_h);
    let mut at = pos2(hp.x + 10.0, plot.top() + 4.0);
    if at.x + size.x > plot.right() {
        at.x = (hp.x - size.x - 10.0).max(plot.left() + 2.0);
    }
    let tip = Rect::from_min_size(at, size);
    p.rect(tip, 0.0, PANEL, Stroke::new(1.0, acc), egui::StrokeKind::Inside);
    txt(&p, tip.min + vec2(8.0, 10.0), Align2::LEFT_CENTER, &when, theme::body(12.5), TEXT_DIM, 0.0);
    for (k, (col, name, v)) in rows.iter().enumerate() {
        let y = tip.top() + 26.0 + k as f32 * row_h;
        p.rect_filled(Rect::from_center_size(pos2(tip.left() + 11.0, y), vec2(6.0, 6.0)), 0.0, *col);
        let mut job = egui::text::LayoutJob::single_section(name.to_string(), egui::TextFormat { font_id: font.clone(), color: TEXT, ..Default::default() });
        job.wrap = egui::text::TextWrapping { max_width: name_w, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
        let g = p.layout_job(job);
        p.galley(pos2(tip.left() + 20.0, y - g.size().y / 2.0), g, TEXT);
        txt(&p, pos2(tip.right() - 8.0, y), Align2::RIGHT_CENTER, &format!("{:.1}%", v * 100.0), theme::body_semi(13.5), *col, 0.0);
    }
}
