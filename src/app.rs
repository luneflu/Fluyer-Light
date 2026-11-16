// app: retained state + event handling. Draws only into the canvas.

use crate::gfx::{Canvas, Rect};
use crate::input::{Event, Key};

pub const BG: u32 = 0xFF18_1818; // memory order BGRA
pub const FG: u32 = 0xFFF0_F0F0;

const SQUARE: i32 = 60;
const STEP: i32 = 4;

pub enum Outcome {
    Idle,
    Redraw,
    Quit,
}

pub struct App {
    pub canvas: Canvas,
    square: (i32, i32),
}

impl App {
    pub fn new(canvas: Canvas) -> Self {
        Self { canvas, square: (24, 24) }
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
                self.square = self.clamp(x - SQUARE / 2, y - SQUARE / 2);
                Outcome::Redraw
            }
        }
    }

    fn nudge(&mut self, dx: i32, dy: i32) -> Outcome {
        let (x, y) = self.square;
        self.square = self.clamp(x + dx, y + dy);
        Outcome::Redraw
    }

    fn clamp(&self, x: i32, y: i32) -> (i32, i32) {
        (
            x.clamp(0, self.canvas.w() - SQUARE),
            y.clamp(0, self.canvas.h() - SQUARE),
        )
    }

    /// Full redraw into the canvas: clear, then draw retained elements.
    pub fn draw(&mut self) {
        self.canvas.clear(BG);
        let (x, y) = self.square;
        self.canvas.fill(Rect::new(x, y, SQUARE, SQUARE), FG);
        self.canvas.mark_all();
    }
}
