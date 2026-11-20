//! A colour picker in the HUD's style: a button showing the colour, which opens a field with
//! every hue (across) at every saturation (up), and a brightness strip where the brightness
//! counts. Where a fade is offered ([`fade_button`]) it picks two colours, the ends of a fade
//! from one into the other.
//!
//! The colour being dragged is kept in egui's memory and handed back each frame; `done` says
//! the pointer let go, which is when the caller saves it.

use eframe::egui::{self, pos2, vec2, Align2, Color32, Id, Mesh, Popup, PopupCloseBehavior, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui};

use super::theme::{self, accent_hi, GREY, TEXT, TEXT_DIM};
use super::widgets;

pub type Rgb = (u8, u8, u8);

/// A colour picked this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Picked {
    pub rgb: Rgb,
    /// The colour that one fades into, where a fade is picked.
    pub to: Option<Rgb>,
    /// The pointer let go: the pick is final.
    pub done: bool,
}

/// What the picker remembers between frames. The hue of a grey and the saturation of black
/// are not in the colour itself, so they stay where the pointer left them as long as the
/// stored colours are still the ones picked here.
#[derive(Clone, Copy)]
struct Pick {
    rgb: Rgb,
    to: Option<Rgb>,
    hsv: [f32; 3],
    /// The second colour's, kept while the fade is off so that it comes back as it was.
    to_hsv: [f32; 3],
    /// The field shows the second colour.
    second: bool,
    /// Being dragged: not saved yet.
    live: bool,
}

impl Pick {
    fn of(rgb: Rgb, to: Option<Rgb>) -> Self {
        let hsv = hsv_of(rgb);
        // A second colour to start a fade with: a quarter of the colour circle on.
        let to_hsv = to.map_or([(hsv[0] + 0.25).rem_euclid(1.0), hsv[1].max(0.75), hsv[2]], hsv_of);
        Pick { rgb, to, hsv, to_hsv, second: false, live: false }
    }

    fn colours(&self) -> (Rgb, Option<Rgb>) {
        (rgb_of(self.hsv), self.to.map(|_| rgb_of(self.to_hsv)))
    }
}

const BUTTON: egui::Vec2 = vec2(36.0, 24.0);
const FIELD: egui::Vec2 = vec2(264.0, 150.0);
const STRIP_H: f32 = 18.0;
/// Columns the field is drawn in: a multiple of six, so each pure colour is on an edge.
const COLUMNS: u32 = 96;

/// Hue, saturation and value of a colour, each 0–1, in the display's own encoding (as
/// `#rrggbb` is).
pub fn hsv_of((r, g, b): Rgb) -> [f32; 3] {
    let [r, g, b] = [r, g, b].map(|c| f32::from(c) / 255.0);
    let most = r.max(g).max(b);
    let span = most - r.min(g).min(b);
    let hue = if span <= 0.0 {
        0.0
    } else if most == r {
        ((g - b) / span).rem_euclid(6.0)
    } else if most == g {
        (b - r) / span + 2.0
    } else {
        (r - g) / span + 4.0
    };
    [hue / 6.0, if most > 0.0 { span / most } else { 0.0 }, most]
}

/// The colour of a hue, saturation and value, each 0–1.
pub fn rgb_of([h, s, v]: [f32; 3]) -> Rgb {
    let h = h.rem_euclid(1.0) * 6.0;
    let part = |shift: f32| {
        let k = (shift + h).rem_euclid(6.0);
        let c = v - v * s * k.min(4.0 - k).clamp(0.0, 1.0);
        (c.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    (part(5.0), part(3.0), part(1.0))
}

/// A colour as `#rrggbb`.
pub fn hex((r, g, b): Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn colour((r, g, b): Rgb) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// The colour a fade from `from` into `to` has at `t` (0 at `from`, 1 at `to`), as the HUD's
/// shader mixes it: in linear light, each end and the mix with its largest part at full, so the
/// fade is as bright in the middle as at its ends.
pub fn fade_at(from: Rgb, to: Rgb, t: f32) -> Rgb {
    let full = |light: [f32; 3]| {
        let most = light.iter().copied().fold(0.0, f32::max);
        if most > 0.0 { light.map(|c| c / most) } else { [1.0; 3] }
    };
    let light = |(r, g, b): Rgb| full([r, g, b].map(|c| (f32::from(c) / 255.0).powf(2.2)));
    let (a, b) = (light(from), light(to));
    let mix = full([0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t.clamp(0.0, 1.0)));
    let [r, g, b] = mix.map(|c| (c.powf(1.0 / 2.2) * 255.0).round() as u8);
    (r, g, b)
}

/// The button showing `stored`; a click opens the picker under it. `brightness` adds the
/// brightness strip; `note` is a line under the field (empty for none). Gives the button's
/// response and the colour picked this frame, if any.
pub fn button(ui: &mut Ui, id: Id, stored: Rgb, brightness: bool, note: &str) -> (Response, Option<Picked>) {
    show(ui, id, stored, None, false, brightness, note)
}

/// As [`button`], for a colour that can fade into a second one (`to`, `None` for one colour
/// alone): the picker has a switch between one colour and a fade, and with the fade the two
/// ends to pick. Only the colours count (no brightness strip).
pub fn fade_button(ui: &mut Ui, id: Id, stored: Rgb, to: Option<Rgb>, note: &str) -> (Response, Option<Picked>) {
    show(ui, id, stored, to, true, false, note)
}

fn show(ui: &mut Ui, id: Id, stored: Rgb, stored_to: Option<Rgb>, fade: bool, brightness: bool, note: &str) -> (Response, Option<Picked>) {
    let kept = ui.ctx().data(|d| d.get_temp::<Pick>(id));
    let live = kept.is_some_and(|p| p.live);
    let mut pick = match kept {
        Some(p) if p.live || (p.rgb == stored && p.to == stored_to) => p,
        // The stored colours changed elsewhere: theirs, with the second colour kept for a fade
        // switched on again.
        Some(p) if stored_to.is_none() => Pick { to_hsv: p.to_hsv, ..Pick::of(stored, None) },
        _ => Pick::of(stored, stored_to),
    };
    let (shown, shown_to) = if live { pick.colours() } else { (stored, stored_to) };

    let popup_id = id.with("popup");
    let open = Popup::is_id_open(ui.ctx(), popup_id);
    let (rect, resp) = ui.allocate_exact_size(BUTTON, Sense::click());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let block = Rect::from_min_size(rect.min, vec2(24.0, rect.height())).shrink(2.0);
        match shown_to {
            Some(to) => p.add(Shape::mesh(fade_mesh(block, shown, to, 6))),
            None => p.rect_filled(block, 0.0, colour(shown)),
        };
        let edge = if open || resp.hovered() { accent_hi() } else { GREY };
        p.rect_stroke(rect, 0.0, Stroke::new(1.0, edge), StrokeKind::Inside);
        // A small arrow: this one opens something, unlike the preset swatches beside it.
        let c = pos2(rect.right() - 7.0, rect.center().y + 1.0);
        let arrow = vec![pos2(c.x - 3.5, c.y - 2.0), pos2(c.x + 3.5, c.y - 2.0), pos2(c.x, c.y + 2.5)];
        p.add(Shape::convex_polygon(arrow, if open || resp.hovered() { accent_hi() } else { TEXT_DIM }, Stroke::NONE));
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let resp = resp.on_hover_text(if fade { "Pick a colour, or two that fade into each other" } else { "Pick a colour" });

    let before = (pick.second, pick.to.is_some());
    let mut moved = None;
    Popup::from_toggle_button_response(&resp).id(popup_id).close_behavior(PopupCloseBehavior::CloseOnClickOutside).gap(4.0).show(|ui| {
        moved = panel(ui, &mut pick, fade, brightness, note);
    });
    // A drag that lost its picker (closed under the pointer) ends when the button is let go.
    let moved = moved.or_else(|| (live && !ui.input(|i| i.pointer.any_down())).then_some(true));
    let picked = moved.map(|done| {
        let (rgb, to) = pick.colours();
        Picked { rgb, to, done }
    });
    if let Some(p) = picked {
        ui.ctx().data_mut(|d| d.insert_temp(id, Pick { rgb: p.rgb, to: p.to, live: !p.done, ..pick }));
    } else if before != (pick.second, pick.to.is_some()) {
        ui.ctx().data_mut(|d| d.insert_temp(id, Pick { rgb: stored, to: stored_to, ..pick }));
    }
    (resp, picked)
}

/// `rect` filled with the fade from `from` into `to`, left to right, in `columns` steps.
fn fade_mesh(rect: Rect, from: Rgb, to: Rgb, columns: u32) -> Mesh {
    let mut mesh = Mesh::default();
    for i in 0..=columns {
        let t = i as f32 / columns as f32;
        let (x, c) = (rect.left() + rect.width() * t, colour(fade_at(from, to, t)));
        mesh.colored_vertex(pos2(x, rect.top()), c);
        mesh.colored_vertex(pos2(x, rect.bottom()), c);
        if i < columns {
            mesh.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
            mesh.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
        }
    }
    mesh
}

/// One end of the fade as a button: its colour and its name; the one the field shows is
/// framed.
fn end_button(ui: &mut Ui, name: &str, rgb: Rgb, selected: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2((FIELD.x - 6.0) / 2.0, 26.0), Sense::click());
    let p = ui.painter();
    let block = Rect::from_min_size(rect.min, vec2(30.0, rect.height())).shrink(3.0);
    p.rect_filled(block, 0.0, colour(rgb));
    p.text(pos2(block.right() + 8.0, rect.center().y), Align2::LEFT_CENTER, name, theme::body_semi(15.0), if selected { TEXT } else { TEXT_DIM });
    let edge = if selected { Stroke::new(2.0, accent_hi()) } else if resp.hovered() { Stroke::new(1.0, TEXT_DIM) } else { Stroke::new(1.0, GREY) };
    p.rect_stroke(rect, 0.0, edge, StrokeKind::Inside);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// The picker itself. `Some(done)` when a colour changed: `false` while the pointer is held
/// on it, `true` in the frame it lets go (or the fade is switched).
fn panel(ui: &mut Ui, pick: &mut Pick, fade: bool, brightness: bool, note: &str) -> Option<bool> {
    let mut moved = None;
    ui.spacing_mut().item_spacing.y = 8.0;

    if fade {
        let fading = pick.to.is_some();
        if widgets::tab_strip(ui, &["One Colour", "Fade"], usize::from(fading)).is_some_and(|i| (i == 1) != fading) {
            pick.to = if fading { None } else { Some(rgb_of(pick.to_hsv)) };
            pick.second = false;
            moved = Some(true);
        }
        if pick.to.is_some() {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if end_button(ui, "From", rgb_of(pick.hsv), !pick.second).clicked() {
                    pick.second = false;
                }
                if end_button(ui, "To", rgb_of(pick.to_hsv), pick.second).clicked() {
                    pick.second = true;
                }
            });
        }
    }
    let second = pick.second && pick.to.is_some();
    let hsv = if second { &mut pick.to_hsv } else { &mut pick.hsv };

    let (rect, resp) = ui.allocate_exact_size(FIELD, Sense::click_and_drag());
    if let Some(at) = resp.interact_pointer_pos() {
        hsv[0] = ((at.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        hsv[1] = ((rect.bottom() - at.y) / rect.height()).clamp(0.0, 1.0);
        // The field is every colour at its brightest: a pick in it is that colour, also
        // coming from black.
        if !brightness || hsv[2] <= 0.0 {
            hsv[2] = 1.0;
        }
        moved = Some(false);
    }
    if resp.drag_stopped() || resp.clicked() {
        moved = Some(true);
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    // Narrow columns from the hue down to white. A column is two triangles, which fade
    // along their diagonal where its two hues differ: many columns keep that out of sight.
    let mut mesh = Mesh::default();
    for i in 0..=COLUMNS {
        let t = i as f32 / COLUMNS as f32;
        let x = rect.left() + rect.width() * t;
        mesh.colored_vertex(pos2(x, rect.top()), colour(rgb_of([t, 1.0, 1.0])));
        mesh.colored_vertex(pos2(x, rect.bottom()), Color32::WHITE);
        if i < COLUMNS {
            mesh.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
            mesh.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
        }
    }
    let p = ui.painter();
    p.add(Shape::mesh(mesh));
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, GREY), StrokeKind::Outside);
    let at = pos2(rect.left() + rect.width() * hsv[0], rect.bottom() - rect.height() * hsv[1]);
    let p = p.with_clip_rect(rect.expand(1.0));
    p.circle_stroke(at, 6.0, Stroke::new(3.0, Color32::BLACK));
    p.circle_stroke(at, 6.0, Stroke::new(1.5, Color32::WHITE));

    if brightness {
        let (rect, resp) = ui.allocate_exact_size(vec2(FIELD.x, STRIP_H), Sense::click_and_drag());
        if let Some(at) = resp.interact_pointer_pos() {
            hsv[2] = ((at.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
            moved = Some(false);
        }
        if resp.drag_stopped() || resp.clicked() {
            moved = Some(true);
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        let full = colour(rgb_of([hsv[0], hsv[1], 1.0]));
        let mut mesh = Mesh::default();
        for (x, c) in [(rect.left(), Color32::BLACK), (rect.right(), full)] {
            mesh.colored_vertex(pos2(x, rect.top()), c);
            mesh.colored_vertex(pos2(x, rect.bottom()), c);
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 2, 3);
        let p = ui.painter();
        p.add(Shape::mesh(mesh));
        p.rect_stroke(rect, 0.0, Stroke::new(1.0, GREY), StrokeKind::Outside);
        let x = rect.left() + rect.width() * hsv[2];
        let mark = Rect::from_center_size(pos2(x, rect.center().y), vec2(6.0, rect.height() + 4.0));
        p.rect_stroke(mark, 0.0, Stroke::new(3.0, Color32::BLACK), StrokeKind::Middle);
        p.rect_stroke(mark, 0.0, Stroke::new(1.5, Color32::WHITE), StrokeKind::Middle);
    }

    match pick.colours() {
        // The fade as it is drawn, and its two colours as the field beside the button has them.
        (from, Some(to)) => {
            let (rect, _) = ui.allocate_exact_size(vec2(FIELD.x, 22.0), Sense::hover());
            let p = ui.painter();
            p.add(Shape::mesh(fade_mesh(rect, from, to, 48)));
            p.rect_stroke(rect, 0.0, Stroke::new(1.0, GREY), StrokeKind::Inside);
            ui.label(egui::RichText::new(format!("{} > {}", hex(from), hex(to))).color(TEXT).font(theme::body_semi(16.0)));
        }
        (now, None) => {
            let (rect, _) = ui.allocate_exact_size(vec2(FIELD.x, 22.0), Sense::hover());
            let p = ui.painter();
            let sample = Rect::from_min_size(rect.min, vec2(56.0, rect.height()));
            p.rect_filled(sample, 0.0, colour(now));
            p.rect_stroke(sample, 0.0, Stroke::new(1.0, GREY), StrokeKind::Inside);
            p.text(pos2(sample.right() + 10.0, rect.center().y), Align2::LEFT_CENTER, hex(now), theme::body_semi(16.0), TEXT);
        }
    }
    if !note.is_empty() {
        ui.set_max_width(FIELD.x);
        ui.label(egui::RichText::new(note).color(TEXT_DIM).font(theme::body(13.5)));
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pure_colours_have_their_hues() {
        assert_eq!(hsv_of((255, 0, 0)), [0.0, 1.0, 1.0]);
        assert_eq!(hsv_of((0, 255, 0)), [2.0 / 6.0, 1.0, 1.0]);
        assert_eq!(hsv_of((0, 0, 255)), [4.0 / 6.0, 1.0, 1.0]);
        assert_eq!(hsv_of((255, 255, 255)), [0.0, 0.0, 1.0]);
        assert_eq!(hsv_of((0, 0, 0)), [0.0, 0.0, 0.0]);
        assert_eq!(rgb_of([0.0, 1.0, 1.0]), (255, 0, 0));
        assert_eq!(rgb_of([1.0, 1.0, 1.0]), (255, 0, 0), "the field's right edge is red again");
        assert_eq!(rgb_of([0.5, 1.0, 1.0]), (0, 255, 255));
        assert_eq!(rgb_of([0.25, 0.0, 0.5]), (128, 128, 128));
    }

    #[test]
    fn a_colour_comes_back_from_its_hue_saturation_and_value() {
        for r in (0..=255u8).step_by(15) {
            for g in (0..=255u8).step_by(15) {
                for b in (0..=255u8).step_by(15) {
                    assert_eq!(rgb_of(hsv_of((r, g, b))), (r, g, b));
                }
            }
        }
    }

    #[test]
    fn a_fade_is_as_bright_in_the_middle_as_at_its_ends() {
        let (red, blue) = ((255, 0, 0), (0, 0, 255));
        assert_eq!((fade_at(red, blue, 0.0), fade_at(red, blue, 1.0)), (red, blue));
        assert_eq!(fade_at(red, blue, 0.5), (255, 0, 255), "half of each as light, its largest part back at full");
        // Only the colours count: a dark end is taken at its brightest, as the HUD takes it.
        assert_eq!(fade_at((128, 0, 0), blue, 0.0), red);
        assert_eq!(fade_at((0, 0, 0), (0, 0, 0), 0.5), (255, 255, 255), "black has no colour: white");
        assert_eq!(fade_at(red, blue, 7.0), blue, "past the end is the end");
    }

    #[test]
    fn hex_is_what_the_settings_read() {
        assert_eq!(hex((255, 128, 0)), "#ff8000");
        assert_eq!(crate::theme::parse_rgb(&hex((1, 2, 3))), Some((1, 2, 3)));
    }
}
