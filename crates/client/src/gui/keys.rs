//! Turns this frame's egui keyboard input into the key events the views handle
//! ([`crate::event_shim`]), so the views' keyboard shortcuts don't depend on egui.

use eframe::egui::{self, Event, Key};

use crate::event_shim::{KeyCode, KeyEvent};

/// Key presses for the views. Empty while a text field has focus, so typing into it
/// doesn't also trigger shortcuts.
pub fn collect(ctx: &egui::Context) -> Vec<KeyEvent> {
    if ctx.text_edit_focused() {
        return Vec::new();
    }
    ctx.input(|i| {
        let mut out = Vec::new();
        for ev in &i.events {
            match ev {
                Event::Text(t) => {
                    out.extend(t.chars().filter(|c| !c.is_control()).map(|c| KeyEvent::from(KeyCode::Char(c))));
                }
                Event::Key { key, pressed: true, modifiers, .. } => {
                    if let Some(code) = map_key(*key, modifiers.shift) {
                        out.push(KeyEvent::from(code));
                    }
                }
                _ => {}
            }
        }
        out
    })
}

/// Non-text keys. Letters, digits and punctuation arrive as `Event::Text` instead.
fn map_key(key: Key, shift: bool) -> Option<KeyCode> {
    Some(match key {
        Key::Enter => KeyCode::Enter,
        Key::Escape => KeyCode::Esc,
        Key::Tab if shift => KeyCode::BackTab,
        Key::Tab => KeyCode::Tab,
        Key::Backspace => KeyCode::Backspace,
        Key::Delete => KeyCode::Delete,
        Key::Insert => KeyCode::Insert,
        Key::ArrowUp => KeyCode::Up,
        Key::ArrowDown => KeyCode::Down,
        Key::ArrowLeft => KeyCode::Left,
        Key::ArrowRight => KeyCode::Right,
        Key::PageUp => KeyCode::PageUp,
        Key::PageDown => KeyCode::PageDown,
        Key::Home => KeyCode::Home,
        Key::End => KeyCode::End,
        _ => return None,
    })
}
