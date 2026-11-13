//! The cockpit-HUD look: palette, fonts and the egui style built from them.
//!
//! The accent (titles, frames, selection, bars) follows `settings.appearance.color` through
//! [`crate::theme`], so changing it in Settings recolours the whole window; every other
//! colour is fixed.

use std::sync::Arc;

use eframe::egui::{
    self, epaint::text::FontTweak, epaint::text::VariationCoords, Color32, FontData,
    FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};

// ---------------------------------------------------------------- fixed palette

/// Window ground: near-black with a warm bias toward the orange accent.
pub const BG: Color32 = Color32::from_rgb(9, 7, 5);
/// Panel fill, a step up from the ground.
pub const PANEL: Color32 = Color32::from_rgb(15, 11, 7);
/// Primary text.
pub const TEXT: Color32 = Color32::from_rgb(236, 222, 202);
/// Secondary text: labels, units, hints.
pub const TEXT_DIM: Color32 = Color32::from_rgb(168, 154, 138);
/// Disabled / tertiary text, guide lines.
pub const GREY: Color32 = Color32::from_rgb(112, 102, 92);
/// Field labels and icy bodies (the in-game blue-cyan).
pub const CYAN: Color32 = Color32::from_rgb(63, 169, 214);
pub const GREEN: Color32 = Color32::from_rgb(111, 207, 127);
pub const YELLOW: Color32 = Color32::from_rgb(232, 184, 74);
pub const RED: Color32 = Color32::from_rgb(232, 92, 66);
pub const BLUE: Color32 = Color32::from_rgb(80, 140, 255);
pub const MAGENTA: Color32 = Color32::from_rgb(255, 90, 200);

// ---------------------------------------------------------------- accent family

/// The configured accent (default the EDCAS orange).
pub fn accent() -> Color32 {
    let (r, g, b) = crate::theme::accent_rgb();
    Color32::from_rgb(r, g, b)
}

/// A brighter accent for headings drawn on dark ground.
pub fn accent_hi() -> Color32 {
    mix(accent(), Color32::WHITE, 0.18)
}

/// The accent at reduced strength: secondary labels, inactive tabs.
pub fn accent_dim() -> Color32 {
    mix(accent(), BG, 0.32)
}

/// The accent barely above the ground: idle frames, empty bar segments.
pub fn accent_mute() -> Color32 {
    mix(accent(), BG, 0.58)
}

/// `c` with its alpha replaced (unmultiplied).
pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Linear mix of two opaque colours, `t` = share of `b`.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Maps a named text colour (the `ratatui` styles the views build lines and colours with)
/// onto the HUD palette, so styled text (help pages, status lines) keeps its meaning.
pub fn from_ratatui(c: ratatui::style::Color) -> Color32 {
    use ratatui::style::Color as C;
    match c {
        C::Reset | C::White => TEXT,
        C::Black => BG,
        C::Gray => TEXT_DIM,
        C::DarkGray => GREY,
        C::Cyan => CYAN,
        C::LightCyan => mix(CYAN, Color32::WHITE, 0.3),
        C::Yellow => YELLOW,
        C::LightYellow => mix(YELLOW, Color32::WHITE, 0.3),
        C::Green => GREEN,
        C::LightGreen => mix(GREEN, Color32::WHITE, 0.3),
        C::Red => RED,
        C::LightRed => mix(RED, Color32::WHITE, 0.3),
        C::Blue => BLUE,
        C::LightBlue => mix(BLUE, Color32::WHITE, 0.3),
        C::Magenta => MAGENTA,
        C::LightMagenta => mix(MAGENTA, Color32::WHITE, 0.3),
        C::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
        C::Indexed(i) => Color32::from_gray(i),
    }
}

// ---------------------------------------------------------------- fonts

const ORBITRON: &[u8] = include_bytes!("../../assets/fonts/Orbitron.ttf");
const RAJDHANI: &[u8] = include_bytes!("../../assets/fonts/Rajdhani-Medium.ttf");
const RAJDHANI_SEMI: &[u8] = include_bytes!("../../assets/fonts/Rajdhani-SemiBold.ttf");
/// Fallback for symbols the display fonts lack (● ◆ ⊕ ★ ⚠ arrows, box drawing).
const DEJAVU: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");

fn fam(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}

/// Body text (Rajdhani Medium). The `Proportional` family, so plain `ui.label` uses it.
pub fn body(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

/// Emphasised body text (Rajdhani SemiBold): values, selected rows.
pub fn body_semi(size: f32) -> FontId {
    FontId::new(size, fam("semi"))
}

/// Display face (Orbitron 600) for headings, tabs and labels. Use uppercase, sparingly.
pub fn head(size: f32) -> FontId {
    FontId::new(size, fam("head"))
}

/// Heavy display face (Orbitron 800) for panel titles and the logo.
pub fn head_bold(size: f32) -> FontId {
    FontId::new(size, fam("head_bold"))
}

/// Fixed-width text (egui's Hack), for aligned code-like data.
pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fd = FontDefinitions::default();
    let wght = |w: f32| FontTweak { coords: VariationCoords::new([(b"wght", w)]), ..Default::default() };
    // Rajdhani sits high in its line box; nudge it down so it centres next to icons.
    let raj_tweak = FontTweak { y_offset_factor: 0.04, ..Default::default() };
    fd.font_data.insert("orb".into(), Arc::new(FontData::from_static(ORBITRON).tweak(wght(600.0))));
    fd.font_data.insert("orbb".into(), Arc::new(FontData::from_static(ORBITRON).tweak(wght(800.0))));
    fd.font_data.insert("raj".into(), Arc::new(FontData::from_static(RAJDHANI).tweak(raj_tweak.clone())));
    fd.font_data.insert("rajs".into(), Arc::new(FontData::from_static(RAJDHANI_SEMI).tweak(raj_tweak)));
    fd.font_data.insert("dejavu".into(), Arc::new(FontData::from_static(DEJAVU)));

    // egui's defaults (Ubuntu-Light, emoji) stay as later fallbacks; DejaVu comes right after
    // the display font so symbols render in a consistent style.
    let defaults = fd.families[&FontFamily::Proportional].clone();
    let chain = |first: &str| {
        let mut v = vec![first.to_string(), "dejavu".to_string()];
        v.extend(defaults.iter().cloned());
        v
    };
    fd.families.insert(FontFamily::Proportional, chain("raj"));
    fd.families.insert(fam("semi"), chain("rajs"));
    fd.families.insert(fam("head"), chain("orb"));
    fd.families.insert(fam("head_bold"), chain("orbb"));
    fd.families.get_mut(&FontFamily::Monospace).unwrap().push("dejavu".into());
    ctx.set_fonts(fd);
}

// ---------------------------------------------------------------- style

/// Applies the HUD style. Called at startup and whenever the accent changes.
pub fn apply_style(ctx: &egui::Context) {
    let acc = accent();
    ctx.all_styles_mut(|s| {
        s.text_styles = [
            (TextStyle::Small, body(13.0)),
            (TextStyle::Body, body(16.0)),
            (TextStyle::Button, body_semi(15.0)),
            (TextStyle::Heading, head_bold(17.0)),
            (TextStyle::Monospace, mono(13.0)),
        ]
        .into();

        let v = &mut s.visuals;
        *v = egui::Visuals::dark();
        v.override_text_color = None;
        v.panel_fill = BG;
        v.window_fill = PANEL;
        v.window_stroke = Stroke::new(1.0, alpha(acc, 110));
        v.window_corner_radius = egui::CornerRadius::ZERO;
        v.menu_corner_radius = egui::CornerRadius::ZERO;
        v.extreme_bg_color = Color32::from_rgb(4, 3, 2);
        v.faint_bg_color = alpha(acc, 10);
        v.code_bg_color = PANEL;
        v.hyperlink_color = CYAN;
        v.warn_fg_color = YELLOW;
        v.error_fg_color = RED;
        v.selection.bg_fill = alpha(acc, 150);
        v.selection.stroke = Stroke::new(1.0, TEXT);
        v.text_cursor.stroke = Stroke::new(2.0, acc);

        let w = &mut v.widgets;
        w.noninteractive.bg_fill = PANEL;
        w.noninteractive.weak_bg_fill = PANEL;
        w.noninteractive.bg_stroke = Stroke::new(1.0, alpha(acc, 60));
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        w.inactive.bg_fill = alpha(acc, 28);
        w.inactive.weak_bg_fill = alpha(acc, 22);
        w.inactive.bg_stroke = Stroke::new(1.0, alpha(acc, 90));
        w.inactive.fg_stroke = Stroke::new(1.0, accent_hi());
        w.hovered.bg_fill = alpha(acc, 70);
        w.hovered.weak_bg_fill = alpha(acc, 60);
        w.hovered.bg_stroke = Stroke::new(1.0, acc);
        w.hovered.fg_stroke = Stroke::new(1.5, TEXT);
        w.active.bg_fill = acc;
        w.active.weak_bg_fill = acc;
        w.active.bg_stroke = Stroke::new(1.0, accent_hi());
        w.active.fg_stroke = Stroke::new(1.5, BG);
        w.open = w.hovered;
        for ws in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            ws.corner_radius = egui::CornerRadius::ZERO;
            ws.expansion = 0.0;
        }

        s.spacing.item_spacing = egui::vec2(8.0, 4.0);
        s.spacing.button_padding = egui::vec2(10.0, 4.0);
        s.spacing.interact_size.y = 22.0;
        s.spacing.scroll = egui::style::ScrollStyle::thin();
        // Labels mustn't take clicks for text selection: rows are clickable underneath them.
        s.interaction.selectable_labels = false;
    });
}
