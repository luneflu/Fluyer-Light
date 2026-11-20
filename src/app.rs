// app: glue. Input → ui::hit → core::Action; core::Changed → present. Holds only
// transient UI state (which control is held) plus the model and the layout.

use crate::core::library;
use crate::core::model::{Action, Changed, Model};
use crate::input::{Event, Key};
use fluyer_pxui::gfx::{Canvas, Rect};
use crate::ui::layout::Layout;
use fluyer_pxui::theme::{
    LIST_PADDING_X, LIST_PADDING_Y, LIST_ROW_GAP, SPRITE_SCALE, TEXT_SCALE,
};
use crate::ui::view::{self, Hit, Press};
use fluyer_pxui::widgets::list::ListView;

pub enum Outcome {
    Idle,
    Redraw,
    Quit,
}

/// Library root. Windows has no fixed user path, so derive it from the profile.
#[cfg(target_os = "macos")]
fn music_dir() -> String {
    "/Users/alvindimas05/Music/The Meaning of Life".to_string()
}

#[cfg(not(target_os = "macos"))]
fn music_dir() -> String {
    "E:\\Music".to_string()
}

pub struct App {
    model: Model,
    layout: Layout,
    list: ListView,
    press: Press,
    wheel_accum: f64, // fractional rows not yet scrolled
}

impl App {
    pub fn new(w: i32, h: i32) -> Self {
        let music = music_dir();
        let tracks = library::scan(&music);
        println!("{} audio files in {music}", tracks.len());
        let mut app = Self {
            model: Model::new(tracks),
            layout: Layout::new(w, h, 0),
            list: list_view(w, h, 0),
            press: Press::None,
            wheel_accum: 0.0,
        };
        if !app.model.player.init() {
            println!("audio: engine init failed");
        }
        app.layout(w, h, 0);
        app
    }

    pub fn resize(&mut self, w: i32, h: i32) {
        self.layout(w, h, self.layout.inset);
    }

    /// Height of the transparent titlebar strip a shell may keep above us.
    #[allow(dead_code)]
    pub fn set_inset(&mut self, px: i32) {
        self.layout(self.layout.w, self.layout.h, px);
    }

    fn layout(&mut self, w: i32, h: i32, inset: i32) {
        self.layout = Layout::new(w, h, inset);
        self.list = list_view(w, h, inset);
        self.model.set_visible(self.list.visible());
    }

    pub fn handle(&mut self, ev: Event) -> Outcome {
        let action = match ev {
            Event::Key { key, mods } => match key {
                Key::Escape => Action::Quit,
                Key::Q if mods.cmd => Action::Quit,
                Key::Up => Action::Move(-1),
                Key::Down => Action::Move(1),
                Key::Enter => Action::Play,
                Key::Space => Action::Toggle,
                _ => return Outcome::Idle,
            },
            Event::Wheel { rows, pixels } => {
                // Wheel notches are coarse; a trackpad sends pixels. Accumulate
                // so one notch is a fraction of a row instead of a whole one.
                const WHEEL_LINES_PER_ROW: f64 = 3.0;
                let step = if pixels {
                    rows / self.list.stride().max(1) as f64
                } else {
                    rows / WHEEL_LINES_PER_ROW
                };
                self.wheel_accum += step;
                let whole = self.wheel_accum.trunc();
                if whole == 0.0 {
                    return Outcome::Idle;
                }
                self.wheel_accum -= whole;
                Action::Scroll(whole as i32)
            }
            Event::Tick => Action::Tick,
            Event::MouseDown { x, y } => match view::hit(&self.layout, &self.list, &self.model, x, y) {
                Some(Hit::Transport(0)) => {
                    self.press = Press::Prev;
                    Action::Step(-1)
                }
                Some(Hit::Transport(1)) => {
                    self.press = Press::Play;
                    Action::Toggle
                }
                Some(Hit::Transport(2)) => {
                    self.press = Press::Next;
                    Action::Step(1)
                }
                Some(Hit::Seek(f)) => Action::Seek(f),
                Some(Hit::Row(i)) => Action::Select(i),
                _ => return Outcome::Idle,
            },
            Event::MouseUp => {
                if self.press == Press::None {
                    return Outcome::Idle;
                }
                self.press = Press::None;
                return Outcome::Redraw;
            }
        };
        match self.model.update(action) {
            Changed::Nothing => Outcome::Idle,
            Changed::Progress | Changed::Full => Outcome::Redraw,
            Changed::Quit => Outcome::Quit,
        }
    }

    pub fn draw(&mut self, c: &mut Canvas) {
        let progress = self.model.player.progress();
        view::render(&self.model, &self.list, &self.layout, self.press, progress, c);
    }

    #[allow(dead_code)] // available for partial-present backends
    pub fn draw_progress(&mut self, c: &mut Canvas) {
        view::render_progress(&self.layout, self.model.player.progress(), c);
    }
}

fn list_view(w: i32, h: i32, inset: i32) -> ListView {
    let l = Layout::new(w, h, inset);
    ListView {
        area: Rect::new(0, l.list_y, w, l.list_view_h),
        pad_x: LIST_PADDING_X,
        pad_y: LIST_PADDING_Y,
        gap: LIST_ROW_GAP,
        text_scale: TEXT_SCALE,
        sprite_scale: SPRITE_SCALE,
    }
}
