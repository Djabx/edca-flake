//! Debug builds only: drive the window from a script and save screenshots, for checking
//! layouts without a person at the keyboard (and without stealing desktop focus).
//!
//! `EDCAS_GUI_SCRIPT` holds whitespace-separated steps, run one per ~300 ms after a 3 s
//! start-up delay (so the journal snapshot has arrived):
//!
//! - `2`, `s`, `Tab`, `Enter`, `Esc`, `Up`, `Down`, `Left`, `Right`, `PageUp`, `PageDown`,
//!   `Backspace`, `Space` — a key press (single characters are typed as text)
//! - `down:W` / `up:W` — holds a key down / lets it go (egui key names), e.g.
//!   `down:D wait:1000 up:D` for a second of held D
//! - `type:some_text` — types text (`_` becomes a space), e.g. into a focused field
//! - `click:x,y` — a left click at window coordinates (logical points)
//! - `scroll:x,y,dy` — mouse wheel at a point (negative `dy` scrolls down)
//! - `drag:x0,y0,x1,y1` / `rdrag:…` — a left / right button drag over a few frames
//! - `wait:ms` — pause
//! - `shot:/path/file.png` — save a screenshot
//! - `quit` — close the window
//!
//! Example: `EDCAS_GUI_SCRIPT="2 wait:500 shot:/tmp/explorer.png quit"`.
//! `EDCAS_GUI_SIZE=960x600` opens the window at another size (default 1440x900).

use std::time::{Duration, Instant};

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, RawInput};

pub struct DevScript {
    steps: Vec<String>,
    next: usize,
    due: Instant,
    pending_shot: Option<String>,
    /// Input still to come, one entry per frame (a drag spans several).
    queued: std::collections::VecDeque<Vec<Event>>,
}

impl DevScript {
    pub fn from_env() -> Option<Self> {
        let s = std::env::var("EDCAS_GUI_SCRIPT").ok()?;
        Some(Self {
            steps: s.split_whitespace().map(str::to_owned).collect(),
            next: 0,
            due: Instant::now() + Duration::from_secs(3),
            pending_shot: None,
            queued: Default::default(),
        })
    }

    /// Injects the next due step's input into this frame.
    pub fn inject(&mut self, ctx: &egui::Context, raw: &mut RawInput) {
        ctx.request_repaint_after(Duration::from_millis(50));
        if let Some(events) = self.queued.pop_front() {
            raw.events.extend(events);
            return;
        }
        if self.pending_shot.is_some() || self.next >= self.steps.len() || Instant::now() < self.due {
            return;
        }
        let step = self.steps[self.next].clone();
        self.next += 1;
        self.due = Instant::now() + Duration::from_millis(300);

        let key = |raw: &mut RawInput, key: Key| {
            for pressed in [true, false] {
                raw.events.push(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE });
            }
        };
        if let Some(ms) = step.strip_prefix("wait:") {
            self.due = Instant::now() + Duration::from_millis(ms.parse().unwrap_or(500));
        } else if let Some(path) = step.strip_prefix("shot:") {
            self.pending_shot = Some(path.to_owned());
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        } else if step == "quit" {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if let Some(t) = step.strip_prefix("type:") {
            raw.events.push(Event::Text(t.replace('_', " ")));
        } else if let Some(xy) = step.strip_prefix("click:") {
            if let Some(pos) = parse_pos(xy) {
                raw.events.push(Event::PointerMoved(pos));
                for pressed in [true, false] {
                    raw.events.push(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
                }
            }
        } else if let Some((button, args)) = step
            .strip_prefix("drag:")
            .map(|a| (PointerButton::Primary, a))
            .or_else(|| step.strip_prefix("rdrag:").map(|a| (PointerButton::Secondary, a)))
        {
            let v: Vec<f32> = args.split(',').filter_map(|v| v.parse().ok()).collect();
            if let [x0, y0, x1, y1] = v[..] {
                let at = |t: f32| egui::pos2(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
                let button_event = |pos, pressed| Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE };
                raw.events.push(Event::PointerMoved(at(0.0)));
                raw.events.push(button_event(at(0.0), true));
                const STEPS: usize = 8;
                for i in 1..=STEPS {
                    self.queued.push_back(vec![Event::PointerMoved(at(i as f32 / STEPS as f32))]);
                }
                self.queued.push_back(vec![button_event(at(1.0), false)]);
            }
        } else if let Some(args) = step.strip_prefix("scroll:") {
            let parts: Vec<f32> = args.split(',').filter_map(|v| v.parse().ok()).collect();
            if let [x, y, dy] = parts[..] {
                raw.events.push(Event::PointerMoved(egui::pos2(x, y)));
                raw.events.push(Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, dy), phase: egui::TouchPhase::Move, modifiers: Modifiers::NONE });
            }
        } else if let Some((name, pressed)) = step.strip_prefix("down:").map(|k| (k, true)).or_else(|| step.strip_prefix("up:").map(|k| (k, false))) {
            match Key::from_name(name) {
                Some(key) => raw.events.push(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE }),
                None => eprintln!("EDCAS_GUI_SCRIPT: unknown key {name:?}"),
            }
        } else if let Some(k) = named_key(&step) {
            key(raw, k);
            if k == Key::Space {
                raw.events.push(Event::Text(" ".into()));
            }
        } else if step.chars().count() == 1 {
            raw.events.push(Event::Text(step));
        } else {
            eprintln!("EDCAS_GUI_SCRIPT: unknown step {step:?}");
        }
    }

    /// Writes a requested screenshot once it arrives.
    pub fn collect_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.pending_shot.clone() else { return };
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = image {
            use eframe::icon_data::IconDataExt;
            let [w, h] = img.size;
            let icon = egui::IconData { rgba: img.as_raw().to_vec(), width: w as u32, height: h as u32 };
            match icon.to_png_bytes() {
                Ok(png) => match std::fs::write(&path, png) {
                    Ok(()) => eprintln!("EDCAS_GUI_SCRIPT: saved {path}"),
                    Err(e) => eprintln!("EDCAS_GUI_SCRIPT: write {path}: {e}"),
                },
                Err(e) => eprintln!("EDCAS_GUI_SCRIPT: encode {path}: {e}"),
            }
            self.pending_shot = None;
        }
    }
}

fn parse_pos(s: &str) -> Option<egui::Pos2> {
    let (x, y) = s.split_once(',')?;
    Some(egui::pos2(x.parse().ok()?, y.parse().ok()?))
}

fn named_key(s: &str) -> Option<Key> {
    Some(match s {
        "Tab" => Key::Tab,
        "Enter" => Key::Enter,
        "Esc" => Key::Escape,
        "Up" => Key::ArrowUp,
        "Down" => Key::ArrowDown,
        "Left" => Key::ArrowLeft,
        "Right" => Key::ArrowRight,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "Home" => Key::Home,
        "End" => Key::End,
        "Space" => Key::Space,
        _ => return None,
    })
}
