//! The key events views handle. The window turns egui's keyboard input into these (see
//! `gui::keys`), so view shortcuts don't depend on egui.

/// A key, after the window has resolved text input into characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Insert,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
}

/// One key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub code: KeyCode,
}

impl From<KeyCode> for KeyEvent {
    fn from(code: KeyCode) -> Self {
        Self { code }
    }
}
