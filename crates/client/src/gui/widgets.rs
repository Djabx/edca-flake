//! HUD building blocks shared by every view: framed panels, section headings, segmented
//! bars, selectable rows, tabs, buttons and badges. Views compose these instead of styling
//! egui widgets one by one, so the whole window keeps one look.

use eframe::egui::{
    self, pos2, text::LayoutJob, vec2, Align, Align2, Color32, FontId, Layout, Painter, Pos2,
    Rect, Response, Sense, Shape, Stroke, TextFormat, Ui, UiBuilder, Vec2,
};

use super::theme::{self, accent, accent_dim, accent_hi, accent_mute, alpha, BG, PANEL, TEXT, TEXT_DIM};

// ---------------------------------------------------------------- text

/// Lays out `text` as one run with optional letter spacing.
pub fn job(text: &str, font: FontId, color: Color32, letter_spacing: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(text, 0.0, TextFormat { font_id: font, color, extra_letter_spacing: letter_spacing, ..Default::default() });
    job
}

/// Paints text anchored at `pos`, returning the rect it covers.
pub fn txt(p: &Painter, pos: Pos2, align: Align2, text: &str, font: FontId, color: Color32, letter_spacing: f32) -> Rect {
    let galley = p.layout_job(job(text, font, color, letter_spacing));
    let r = align.anchor_size(pos, galley.size());
    p.galley(r.min, galley, color);
    r
}

/// A label in the display face with letter spacing, e.g. small uppercase captions.
pub fn caption(ui: &mut Ui, text: &str, color: Color32) -> Response {
    ui.label(job(&text.to_uppercase(), theme::head(9.5), color, 1.4))
}

/// Dim helper text.
pub fn hint(ui: &mut Ui, text: &str) -> Response {
    ui.label(egui::RichText::new(text).color(TEXT_DIM).font(theme::body(14.0)))
}

/// A placeholder for a panel with nothing to show yet: what will appear and how to get it.
pub fn empty_state(ui: &mut Ui, text: &str) {
    ui.add_space(12.0);
    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(text).color(theme::GREY).font(theme::body(15.0)).italics());
    });
}

/// One label/value pair in a two-column grid; call inside `egui::Grid` rows (`kv` ends the row).
pub fn kv(ui: &mut Ui, key: &str, value: impl Into<egui::WidgetText>) {
    ui.label(egui::RichText::new(key).color(theme::CYAN).font(theme::body(15.0)));
    ui.label(value);
    ui.end_row();
}

/// A value rendered emphasised (semibold, primary colour), for `kv` and table cells.
pub fn value(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).font(theme::body_semi(15.0)).color(TEXT)
}

/// Styled text lines (`ratatui::text::Line`) as egui layout jobs, mapping colours onto the
/// HUD palette. Useful for prose-like pages (help, notes) that views already build as lines.
pub fn line_job(line: &ratatui::text::Line<'_>, font: &FontId) -> LayoutJob {
    use ratatui::style::Modifier;
    let mut job = LayoutJob::default();
    let base_fg = line.style.fg;
    for span in &line.spans {
        let st = line.style.patch(span.style);
        let fg = st.fg.or(base_fg).map(theme::from_ratatui).unwrap_or(TEXT);
        let bold = st.add_modifier.contains(Modifier::BOLD);
        let font_id = if bold && font.family == egui::FontFamily::Proportional {
            theme::body_semi(font.size)
        } else {
            font.clone()
        };
        let mut fmt = TextFormat { font_id, color: fg, ..Default::default() };
        if let Some(bg) = st.bg {
            fmt.background = theme::from_ratatui(bg);
        }
        if st.add_modifier.contains(Modifier::ITALIC) {
            fmt.italics = true;
        }
        if st.add_modifier.contains(Modifier::UNDERLINED) {
            fmt.underline = Stroke::new(1.0, fg);
        }
        job.append(&span.content, 0.0, fmt);
    }
    job
}

/// Renders styled text lines one label per line.
pub fn styled_lines(ui: &mut Ui, lines: &[ratatui::text::Line<'_>]) {
    let font = theme::body(15.0);
    for line in lines {
        if line.spans.iter().all(|s| s.content.trim().is_empty()) {
            ui.add_space(8.0);
        } else {
            ui.label(line_job(line, &font));
        }
    }
}

// ---------------------------------------------------------------- panels

/// The chamfered HUD frame drawn around a rect, with a title strip. Returns the content rect.
pub fn paint_hud_frame(p: &Painter, r: Rect, title: &str, right: &str) -> Rect {
    let acc = accent();
    let c = 12.0; // chamfer
    let pts = vec![
        r.left_top(),
        pos2(r.right() - c, r.top()),
        pos2(r.right(), r.top() + c),
        r.right_bottom(),
        pos2(r.left() + c, r.bottom()),
        pos2(r.left(), r.bottom() - c),
    ];
    p.add(Shape::convex_polygon(pts.clone(), PANEL, Stroke::NONE));
    let hh = 26.0;
    let hr = Rect::from_min_max(r.left_top(), pos2(r.right(), r.top() + hh));
    let hpts = vec![hr.left_top(), pos2(hr.right() - c, hr.top()), pos2(hr.right(), hr.top() + c), hr.right_bottom(), hr.left_bottom()];
    p.add(Shape::convex_polygon(hpts, alpha(acc, 26), Stroke::NONE));
    p.line_segment([pos2(r.left(), hr.bottom()), pos2(r.right(), hr.bottom())], Stroke::new(1.0, alpha(acc, 110)));
    p.rect_filled(Rect::from_min_size(pos2(r.left() + 10.0, hr.center().y - 5.0), vec2(4.0, 10.0)), 0.0, acc);
    let tr = txt(p, pos2(r.left() + 22.0, hr.center().y + 0.5), Align2::LEFT_CENTER, &title.to_uppercase(), theme::head_bold(11.0), accent_hi(), 2.2);
    if !right.is_empty() {
        // Keep the right-hand caption clear of the title on narrow panels.
        let rr = txt(p, pos2(r.right() - 22.0, hr.center().y + 0.5), Align2::RIGHT_CENTER, &right.to_uppercase(), theme::head(9.0), accent_dim(), 1.5);
        if rr.left() < tr.right() + 8.0 {
            p.rect_filled(rr, 0.0, PANEL);
        }
    }
    p.add(Shape::closed_line(pts, Stroke::new(1.0, alpha(acc, 80))));
    let b = 14.0;
    let s = Stroke::new(2.0, acc);
    let lt = r.left_top() + vec2(0.5, 0.5);
    p.add(Shape::line(vec![lt + vec2(0.0, b), lt, lt + vec2(b, 0.0)], s));
    let rb = r.right_bottom() - vec2(0.5, 0.5);
    p.add(Shape::line(vec![rb - vec2(0.0, b), rb, rb - vec2(b, 0.0)], s));
    p.line_segment([pos2(r.right() - c, r.top() + 0.5), pos2(r.right() - 0.5, r.top() + c)], s);
    p.line_segment([pos2(r.left() + 0.5, r.bottom() - c), pos2(r.left() + c, r.bottom() - 0.5)], s);
    Rect::from_min_max(pos2(r.left() + 12.0, hr.bottom() + 8.0), pos2(r.right() - 12.0, r.bottom() - 10.0))
}

/// A HUD panel that fills the space `ui` has left (e.g. one column of `ui.columns`, or the
/// whole view). Content gets its own child `Ui`; wrap long content in a `ScrollArea`.
///
/// ```ignore
/// hud_panel(ui, "Navigation", "27 bodies", |ui| { ... });
/// ```
pub fn hud_panel<R>(ui: &mut Ui, title: &str, right: &str, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let rect = ui.available_rect_before_wrap();
    hud_panel_at(ui, rect, title, right, add_contents)
}

/// [`hud_panel`] drawn into a given rect.
pub fn hud_panel_at<R>(ui: &mut Ui, rect: Rect, title: &str, right: &str, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let inner = paint_hud_frame(ui.painter(), rect, title, right);
    let mut child = ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::top_down(Align::Min)));
    child.set_clip_rect(inner.expand(2.0).intersect(ui.clip_rect()));
    let r = add_contents(&mut child);
    ui.allocate_rect(rect, Sense::hover());
    r
}

/// Section heading inside a panel: a small marker, the uppercase title, an optional count
/// chip, and a rule fading out to the right.
pub fn section(ui: &mut Ui, title: &str, count: Option<usize>) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let p = ui.painter();
    let acc = accent();
    let y = r.bottom() - 9.0;
    p.add(Shape::convex_polygon(vec![pos2(r.left(), y - 4.0), pos2(r.left() + 4.0, y), pos2(r.left(), y + 4.0)], acc, Stroke::NONE));
    let tr = txt(p, pos2(r.left() + 10.0, y), Align2::LEFT_CENTER, &title.to_uppercase(), theme::head_bold(10.5), accent_hi(), 2.0);
    let mut x = tr.right() + 8.0;
    if let Some(c) = count {
        let label = c.to_string();
        let cr = Align2::LEFT_CENTER.anchor_size(pos2(x + 4.0, y), p.layout_job(job(&label, theme::head(9.0), BG, 0.5)).size());
        let bg = cr.expand2(vec2(4.0, 1.5));
        p.rect_filled(bg, 1.0, accent_dim());
        txt(p, pos2(x + 4.0, y), Align2::LEFT_CENTER, &label, theme::head(9.0), BG, 0.5);
        x = bg.right() + 8.0;
    }
    rule(p, x, r.right(), y);
}

/// A horizontal rule bright at `x0`, fading toward `x1`.
pub fn rule(p: &Painter, x0: f32, x1: f32, y: f32) {
    if x1 <= x0 {
        return;
    }
    let acc = accent();
    let mut mesh = egui::Mesh::default();
    let h = 0.5;
    mesh.colored_vertex(pos2(x0, y - h), alpha(acc, 140));
    mesh.colored_vertex(pos2(x1, y - h), alpha(acc, 18));
    mesh.colored_vertex(pos2(x1, y + h), alpha(acc, 18));
    mesh.colored_vertex(pos2(x0, y + h), alpha(acc, 140));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(Shape::mesh(mesh));
}

// ---------------------------------------------------------------- bars

/// Segmented HUD bar filling `frac` (0..=1) of `r`; any non-zero value lights one segment.
pub fn seg_bar(p: &Painter, r: Rect, frac: f32, col: Color32) {
    let seg = 4.0;
    let gap = 2.0;
    let n = ((r.width() + gap) / (seg + gap)).floor().max(0.0) as usize;
    let lit = (frac.clamp(0.0, 1.0) * n as f32).round() as usize;
    let lit = if frac > 0.0 { lit.max(1) } else { 0 };
    for i in 0..n {
        let x = r.left() + i as f32 * (seg + gap);
        let sr = Rect::from_min_size(pos2(x, r.top()), vec2(seg, r.height()));
        p.rect_filled(sr, 0.0, if i < lit { col } else { alpha(col, 28) });
    }
}

/// Allocates a segmented bar `width` wide (use `ui.available_width()` to fill).
pub fn bar(ui: &mut Ui, width: f32, frac: f32, col: Color32) -> Response {
    let (r, resp) = ui.allocate_exact_size(vec2(width, 14.0), Sense::hover());
    seg_bar(ui.painter(), r.shrink2(vec2(0.0, 3.0)), frac, col);
    resp
}

// ---------------------------------------------------------------- interactive

/// A full-width clickable row. Paints hover/selection behind `add_contents` and returns the
/// row's click response. Selected rows get a solid accent fill, so draw their text in
/// [`row_text_color`].
pub fn selectable_row(ui: &mut Ui, selected: bool, height: f32, add_contents: impl FnOnce(&mut Ui)) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let acc = accent();
    if selected {
        ui.painter().rect_filled(rect, 0.0, acc);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 0.0, alpha(acc, 34));
    }
    let mut child = ui.new_child(
        UiBuilder::new().max_rect(rect.shrink2(vec2(6.0, 0.0))).layout(Layout::left_to_right(Align::Center)),
    );
    add_contents(&mut child);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Text colour for content on a row: dark on the accent fill when selected.
pub fn row_text_color(selected: bool, normal: Color32) -> Color32 {
    if selected { BG } else { normal }
}

/// Chamfered tab strip. Returns the index clicked this frame, if any.
pub fn tab_strip(ui: &mut Ui, labels: &[&str], selected: usize) -> Option<usize> {
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (i, label) in labels.iter().enumerate() {
            if tab_button(ui, label, i == selected, 24.0).clicked() {
                clicked = Some(i);
            }
        }
    });
    clicked
}

/// One chamfered tab button.
pub fn tab_button(ui: &mut Ui, label: &str, selected: bool, height: f32) -> Response {
    let acc = accent();
    let galley = ui.painter().layout_job(job(&label.to_uppercase(), theme::head(10.0), TEXT, 1.6));
    let w = galley.size().x + 26.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, height), Sense::click());
    let c = 6.0;
    let pts = vec![r.left_top(), pos2(r.right() - c, r.top()), pos2(r.right(), r.top() + c), r.right_bottom(), r.left_bottom()];
    let (fill, stroke, col) = if selected {
        (acc, Stroke::new(1.0, acc), BG)
    } else if resp.hovered() {
        (alpha(acc, 50), Stroke::new(1.0, acc), accent_hi())
    } else {
        (alpha(acc, 14), Stroke::new(1.0, alpha(acc, 90)), accent_hi())
    };
    let p = ui.painter();
    p.add(Shape::convex_polygon(pts.clone(), fill, Stroke::NONE));
    p.add(Shape::closed_line(pts, stroke));
    txt(p, r.center() + vec2(0.0, 0.5), Align2::CENTER_CENTER, &label.to_uppercase(), theme::head(10.0), col, 1.6);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// An outlined action button in the display face, e.g. `hud_button(ui, "Refresh")`.
pub fn hud_button(ui: &mut Ui, label: &str) -> Response {
    hud_button_enabled(ui, label, true)
}

/// [`hud_button`] that can be greyed out.
pub fn hud_button_enabled(ui: &mut Ui, label: &str, enabled: bool) -> Response {
    let acc = accent();
    let galley = ui.painter().layout_job(job(&label.to_uppercase(), theme::head(9.5), TEXT, 1.4));
    let size = vec2(galley.size().x + 20.0, 24.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (r, resp) = ui.allocate_exact_size(size, sense);
    let p = ui.painter();
    let (fill, stroke, col) = if !enabled {
        (Color32::TRANSPARENT, Stroke::new(1.0, theme::GREY), theme::GREY)
    } else if resp.is_pointer_button_down_on() {
        (acc, Stroke::new(1.0, acc), BG)
    } else if resp.hovered() {
        (alpha(acc, 60), Stroke::new(1.0, acc), TEXT)
    } else {
        (alpha(acc, 18), Stroke::new(1.0, alpha(acc, 140)), accent_hi())
    };
    p.rect(r, 0.0, fill, stroke, egui::StrokeKind::Inside);
    txt(p, r.center() + vec2(0.0, 0.5), Align2::CENTER_CENTER, &label.to_uppercase(), theme::head(9.5), col, 1.4);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// A single-line text field in the HUD style, with placeholder text.
pub fn text_field(ui: &mut Ui, value: &mut String, placeholder: &str, width: f32) -> Response {
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(egui::RichText::new(placeholder).color(theme::GREY))
            .font(theme::body(16.0))
            .text_color(TEXT)
            .desired_width(width)
            .margin(vec2(6.0, 3.0)),
    )
}

/// Whether `resp` (a text field) was just submitted with Enter.
pub fn submitted(ui: &Ui, resp: &Response) -> bool {
    resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}

/// An outlined uppercase tag, e.g. `badge(ui, "Landable", CYAN)`.
pub fn badge(ui: &mut Ui, text: &str, col: Color32) -> Response {
    let galley = ui.painter().layout_job(job(&text.to_uppercase(), theme::head(8.5), col, 1.2));
    let size = galley.size() + vec2(12.0, 6.0);
    let (r, resp) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    p.rect(r, 0.0, alpha(col, 22), Stroke::new(1.0, col), egui::StrokeKind::Inside);
    txt(p, r.center() + vec2(0.0, 0.5), Align2::CENTER_CENTER, &text.to_uppercase(), theme::head(8.5), col, 1.2);
    resp
}

/// A keyboard shortcut chip (`key`) followed by what it does, for hint rows.
pub fn key_hint(ui: &mut Ui, key: &str, desc: &str) {
    let galley = ui.painter().layout_job(job(&key.to_uppercase(), theme::head(8.5), BG, 0.8));
    let size = galley.size() + vec2(8.0, 4.0);
    let (r, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(r, 1.0, accent_dim());
    txt(ui.painter(), r.center() + vec2(0.0, 0.5), Align2::CENTER_CENTER, &key.to_uppercase(), theme::head(8.5), BG, 0.8);
    ui.label(egui::RichText::new(desc).color(TEXT_DIM).font(theme::body(13.5)));
}

/// Table header cell text.
pub fn th(ui: &mut Ui, text: &str) -> Response {
    ui.label(job(&text.to_uppercase(), theme::head(9.0), accent_dim(), 1.2))
}

/// Table header cell that sorts on click; `state` is `Some(ascending)` for the active column.
pub fn th_sort(ui: &mut Ui, text: &str, state: Option<bool>) -> Response {
    let arrow = match state {
        Some(true) => " ▲",
        Some(false) => " ▼",
        None => "",
    };
    let col = if state.is_some() { accent_hi() } else { accent_dim() };
    let resp = ui.add(egui::Label::new(job(&format!("{}{arrow}", text.to_uppercase()), theme::head(9.0), col, 1.2)).sense(Sense::click()));
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// The muted accent, re-exported for views drawing their own idle chrome.
pub fn mute() -> Color32 {
    accent_mute()
}

/// Size of `text` in `font`, for manual layout.
pub fn text_size(ui: &Ui, text: &str, font: FontId) -> Vec2 {
    ui.painter().layout_no_wrap(text.to_owned(), font, TEXT).size()
}
