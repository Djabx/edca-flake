//! Small drawn pictures of the exobiology genera, so a pilot sees what to look for on the
//! ground besides the genus name. The drawings are edcas-common's codex pictures, the same
//! the website shows, painted with the egui painter in the caller's colour.

use edcas_common::codex::art::{self, Fill, Pen, Shape};
use eframe::egui::{self, pos2, vec2, Color32, CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind};

use super::theme::alpha;

/// The genera a surface signal scan (`SAASignalsFound`) can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Genus {
    Aleoida,
    Bacterium,
    Cactoida,
    Clypeus,
    Concha,
    Electricae,
    Fonticulua,
    Fumerola,
    Fungoida,
    Osseus,
    Recepta,
    Frutexa,
    Stratum,
    Tubus,
    Tussock,
    CrystallineShards,
    AmphoraPlant,
    Anemone,
    BrainTree,
    BarkMound,
    SinuousTubers,
}

impl Genus {
    /// The genus a codex symbol names (`$Codex_Ent_Bacterial_Genus_Name;`). The symbols are
    /// the same in every game language, unlike `Genus_Localised`.
    pub fn from_symbol(symbol: &str) -> Option<Genus> {
        let key = symbol.trim_start_matches('$').trim_start_matches("Codex_Ent_");
        let key = key.trim_end_matches(';').trim_end_matches("_Name").trim_end_matches("_Genus");
        Some(match key {
            "Aleoids" => Genus::Aleoida,
            "Bacterial" => Genus::Bacterium,
            "Cactoid" => Genus::Cactoida,
            "Clypeus" => Genus::Clypeus,
            "Conchas" => Genus::Concha,
            "Electricae" => Genus::Electricae,
            "Fonticulus" => Genus::Fonticulua,
            "Fumerolas" => Genus::Fumerola,
            "Fungoids" => Genus::Fungoida,
            "Osseus" => Genus::Osseus,
            "Recepta" => Genus::Recepta,
            "Shrubs" => Genus::Frutexa,
            "Stratum" => Genus::Stratum,
            "Tubus" => Genus::Tubus,
            "Tussocks" => Genus::Tussock,
            "Ground_Struct_Ice" => Genus::CrystallineShards,
            "Vents" => Genus::AmphoraPlant,
            "Brancae" => Genus::BrainTree,
            "Cone" => Genus::BarkMound,
            "Tube" => Genus::SinuousTubers,
            k if k.starts_with("Sphere") => Genus::Anemone,
            _ => return None,
        })
    }

    /// The genus' codex page and picture (`/codex/<slug>`).
    pub fn slug(self) -> &'static str {
        match self {
            Genus::Aleoida => "aleoida",
            Genus::Bacterium => "bacterium",
            Genus::Cactoida => "cactoida",
            Genus::Clypeus => "clypeus",
            Genus::Concha => "concha",
            Genus::Electricae => "electricae",
            Genus::Fonticulua => "fonticulua",
            Genus::Fumerola => "fumerola",
            Genus::Fungoida => "fungoida",
            Genus::Osseus => "osseus",
            Genus::Recepta => "recepta",
            Genus::Frutexa => "frutexa",
            Genus::Stratum => "stratum",
            Genus::Tubus => "tubus",
            Genus::Tussock => "tussock",
            Genus::CrystallineShards => "crystalline-shards",
            Genus::AmphoraPlant => "amphora-plant",
            Genus::Anemone => "anemone",
            Genus::BrainTree => "brain-tree",
            Genus::BarkMound => "bark-mound",
            Genus::SinuousTubers => "sinuous-tubers",
        }
    }

    /// What it looks like on the ground, for the picture's tooltip.
    pub fn looks(self) -> &'static str {
        art::looks(self.slug()).unwrap_or_default()
    }
}

/// Paints `genus` (or a generic sprout for `None`) into the square `r`.
pub fn paint(p: &Painter, r: Rect, genus: Option<Genus>, col: Color32) {
    let marks = genus.and_then(|g| art::picture(g.slug())).unwrap_or_else(art::sprout);
    let w = (r.width() / 30.0).max(1.0);
    let at = |(x, y): art::Pt| pos2(r.left() + x * r.width(), r.top() + y * r.height());
    for m in marks {
        let stroke = Stroke::new(w * m.pen.width(), alpha(col, m.pen.alpha()));
        let fill = if m.fill == Fill::None { Color32::TRANSPARENT } else { alpha(col, m.fill.alpha()) };
        match m.shape {
            Shape::Path { pts, closed } => {
                let pts: Vec<Pos2> = pts.into_iter().map(at).collect();
                match (closed, m.fill) {
                    (false, _) => p.add(egui::Shape::line(pts, stroke)),
                    (true, Fill::None) => p.add(egui::Shape::closed_line(pts, stroke)),
                    // Every filled polygon in a genus picture is convex.
                    (true, _) => p.add(egui::Shape::convex_polygon(pts, fill, stroke)),
                };
            }
            Shape::Ellipse { c, r: (rx, ry) } => {
                let rad = vec2(rx * r.width(), ry * r.height());
                if m.fill != Fill::None {
                    p.add(egui::Shape::ellipse_filled(at(c), rad, fill));
                }
                if m.pen != Pen::None {
                    p.add(egui::Shape::ellipse_stroke(at(c), rad, stroke));
                }
            }
            Shape::Rect { min, max, round } => {
                let corner = CornerRadius::from((round * r.width()).min(255.0) as u8);
                p.rect(Rect::from_min_max(at(min), at(max)), corner, fill, stroke, StrokeKind::Middle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genus_by_codex_symbol_in_any_language() {
        assert_eq!(Genus::from_symbol("$Codex_Ent_Bacterial_Genus_Name;"), Some(Genus::Bacterium));
        assert_eq!(Genus::from_symbol("$Codex_Ent_Tussocks_Genus_Name;"), Some(Genus::Tussock));
        assert_eq!(Genus::from_symbol("$Codex_Ent_Shrubs_Genus_Name;"), Some(Genus::Frutexa));
        assert_eq!(Genus::from_symbol("$Codex_Ent_Brancae_Name;"), Some(Genus::BrainTree));
        assert_eq!(Genus::from_symbol("$Codex_Ent_Ground_Struct_Ice_Name;"), Some(Genus::CrystallineShards));
        assert_eq!(Genus::from_symbol("$Codex_Ent_Tube_Name;"), Some(Genus::SinuousTubers));
        assert_eq!(Genus::from_symbol("$Codex_Ent_Tubus_Genus_Name;"), Some(Genus::Tubus));
        assert_eq!(Genus::from_symbol("$Codex_Ent_Foo_Genus_Name;"), None);
    }
}
