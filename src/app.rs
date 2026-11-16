// app: retained state + event handling. Draws only into the canvas.

use crate::font;
use crate::gfx::Canvas;
use crate::input::{Event, Key};
use crate::widget::Button;

pub const BG: u32 = 0xFF18_1818; // memory order BGRA
pub const FG: u32 = 0xFFF0_F0F0;

const TEXT: &str = "Hello, World!";
const BTN: Button = Button {
    x: 16,
    y: 16,
    label: "OK",
    scale: SCALE,
    pad_x: 8,
    pad_y: 4,
    bg: 0xFF3C_3C3C,
    border: FG,
    border_w: 1,
    fg: FG,
};
const SCALE: i32 = 2;
const STEP: i32 = 3;

pub enum Outcome {
    Idle,
    Redraw,
    Quit,
}

pub struct App {
    pub canvas: Canvas,
    pos: (i32, i32), // text top-left, design space
    button: Button,
}

impl App {
    pub fn new(canvas: Canvas) -> Self {
        let mut app = Self { canvas, pos: (0, 0), button: BTN };
        app.layout();
        app
    }

    /// Shell handed us a new surface; re-centre and keep drawing.
    pub fn resize(&mut self, canvas: Canvas) {
        self.canvas = canvas;
        self.button.clamp(self.canvas.w(), self.canvas.h());
        self.layout();
    }

    fn layout(&mut self) {
        let ink = font::ink_bounds(TEXT, SCALE);
        self.pos = (
            (self.canvas.w() - ink.w) / 2 - ink.x,
            (self.canvas.h() - ink.h) / 2 - ink.y,
        );
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
                if self.button.contains(x, y) {
                    println!("clicked: {}", self.button.label);
                    return Outcome::Redraw;
                }
                let ink = font::ink_bounds(TEXT, SCALE);
                self.pos = self.clamp(x - ink.w / 2 - ink.x, y - ink.h / 2 - ink.y);
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
        let ink = font::ink_bounds(TEXT, SCALE);
        (
            x.clamp(-ink.x, self.canvas.w() - ink.x - ink.w),
            y.clamp(-ink.y, self.canvas.h() - ink.y - ink.h),
        )
    }

    /// Full redraw into the canvas: clear, then draw retained elements.
    pub fn draw(&mut self) {
        self.canvas.clear(BG);
        let (x, y) = self.pos;
        font::draw(&mut self.canvas, TEXT, x, y, SCALE, FG);
        self.button.draw(&mut self.canvas);
        self.canvas.mark_all();
    }
}
