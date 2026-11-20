//! Styled text as the views describe it: a colour by name or value, bold or not, and lines made
//! of spans. The views build these without knowing egui; `gui::theme::hud_color` puts the named
//! colours onto the HUD palette and `gui::widgets::line_job` lays the lines out.

/// A text colour: one of the palette's names, or a value of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    Rgb(u8, u8, u8),
}

/// A span's look. An unset colour means "the text colour".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub fg: Option<Color>,
    pub bold: bool,
}

impl Style {
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

/// A run of text in one style.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub content: String,
    pub style: Style,
}

impl Span {
    pub fn styled(content: impl Into<String>, style: Style) -> Self {
        Self { content: content.into(), style }
    }
}

/// One line of text, as its spans.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    pub spans: Vec<Span>,
}

impl From<Span> for Line {
    fn from(span: Span) -> Self {
        Self { spans: vec![span] }
    }
}

impl From<Vec<Span>> for Line {
    fn from(spans: Vec<Span>) -> Self {
        Self { spans }
    }
}

impl From<&str> for Line {
    fn from(text: &str) -> Self {
        Span::styled(text, Style::default()).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_style_is_plain_until_told_otherwise() {
        assert_eq!(Style::default(), Style { fg: None, bold: false });
        assert_eq!(Style::default().fg(Color::Cyan).bold(), Style { fg: Some(Color::Cyan), bold: true });
    }

    #[test]
    fn a_plain_line_is_one_unstyled_span() {
        let line = Line::from("  note");
        assert_eq!(line.spans, vec![Span { content: "  note".into(), style: Style::default() }]);
    }
}
