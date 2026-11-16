// app: retained state + event handling. Draws only into the canvas.

use crate::font;
use crate::gfx::Canvas;
use crate::input::{Event, Key};

pub const BG: u32 = 0xFF18_1818; // memory order BGRA
pub const FG: u32 = 0xFFF0_F0F0;

const TEXT: &str = "Hello, World!";
const SCALE: i32 = 3;
const STEP: i32 = 3;

pub enum Outcome {
    Idle,
    Redraw,
    Quit,
}

pub struct App {
    pub canvas: Canvas,
    pos: (i32, i32), // text top-left, design space
}

impl App {
    pub fn new(canvas: Canvas) -> Self {
        let (w, h) = (canvas.w(), canvas.h());
        Self {
            canvas,
            pos: (
                (w - font::text_width(TEXT, SCALE)) / 2,
                (h - font::text_height(SCALE)) / 2,
            ),
        }
    }

    pub fn handle(&mut self, ev: Event) -> Outcome {
        match ev {
            Event::Key { key, mods } => match key {
                Key::Escape => Outcome::Quit,
                Key::Q if mods.cmd => Outcome::Quit,
                Key::Left => self.nudge(-STEP, 0),
                Key::Right => self.nudge(STEP, 0),
                Key::Up => self.nudge(0, -STEP),
                Key::Down => self.nudge(0, STEP),
                _ => Outcome::Idle,
            },
            Event::MouseDown { x, y } => {
                let (tw, th) = (font::text_width(TEXT, SCALE), font::text_height(SCALE));
                self.pos = self.clamp(x - tw / 2, y - th / 2);
                Outcome::Redraw
            }
        }
    }

    fn nudge(&mut self, dx: i32, dy: i32) -> Outcome {
        let (x, y) = self.pos;
        self.pos = self.clamp(x + dx, y + dy);
        Outcome::Redraw
    }

    fn clamp(&self, x: i32, y: i32) -> (i32, i32) {
        let (tw, th) = (font::text_width(TEXT, SCALE), font::text_height(SCALE));
        (x.clamp(0, self.canvas.w() - tw), y.clamp(0, self.canvas.h() - th))
    }

    /// Full redraw into the canvas: clear, then draw retained elements.
    pub fn draw(&mut self) {
        self.canvas.clear(BG);
        let (x, y) = self.pos;
        font::draw(&mut self.canvas, TEXT, x, y, SCALE, FG);
        self.canvas.mark_all();
    }
}
