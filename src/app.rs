// app: retained state + event handling. Draws only into the canvas.

use crate::library;
use crate::gfx::Canvas;
use crate::input::{Event, Key};
use crate::widget::{Item, List};

pub const BG: u32 = 0xFF18_1818; // memory order BGRA
pub const FG: u32 = 0xFFF0_F0F0;

const MUSIC: &str = "/Users/alvindimas05/Music/The Meaning of Life";
const SCALE: i32 = 2;

pub enum Outcome {
    Idle,
    Redraw,
    Quit,
}

pub struct App {
    pub canvas: Canvas,
    list: List,
    tracks: Vec<String>, // full path per row, same order as list.items
}

impl App {
    pub fn new(canvas: Canvas) -> Self {
        let tracks = library::scan(MUSIC);
        let items = tracks
            .iter()
            .map(|p| Item { label: library::label(p).to_string() })
            .collect();
        println!("{} audio files in {MUSIC}", tracks.len());
        let mut app = Self {
            canvas,
            list: List {
                x: 0,
                y: 0,
                w: 0,
                items,
                scale: SCALE,
                pad_x: 8,
                pad_y: 4,
                gap: 0,
                bg: 0xFF20_2020,
                alt_bg: 0xFF24_2424,
                sel_bg: 0xFF40_4040,
                fg: FG,
                sel_fg: FG,
                selected: if tracks.is_empty() { None } else { Some(0) },
                scroll: 0,
            },
            tracks,
        };
        app.layout();
        app
    }

    /// Shell handed us a new surface; keep geometry on-canvas.
    pub fn resize(&mut self, canvas: Canvas) {
        self.canvas = canvas;
        self.layout();
    }

    fn layout(&mut self) {
        self.list.clamp(self.canvas.w(), self.canvas.h());
        if let Some(i) = self.list.selected {
            self.list.ensure_visible(i, self.canvas.h());
        }
    }

    pub fn handle(&mut self, ev: Event) -> Outcome {
        match ev {
            Event::Key { key, mods } => match key {
                Key::Escape => Outcome::Quit,
                Key::Q if mods.cmd => Outcome::Quit,
                Key::Up => self.move_by(-1),
                Key::Down => self.move_by(1),
                Key::Enter => {
                    self.print_selected();
                    Outcome::Idle
                }
                _ => Outcome::Idle,
            },
            Event::MouseDown { x, y } => {
                if let Some(i) = self.list.hit(x, y, self.canvas.w()) {
                    self.list.selected = Some(i);
                    self.print_selected();
                    return Outcome::Redraw;
                }
                Outcome::Idle
            }
        }
    }

    fn move_by(&mut self, d: i32) -> Outcome {
        let n = self.list.items.len();
        if n == 0 {
            return Outcome::Idle;
        }
        let cur = self.list.selected.unwrap_or(0) as i32;
        let next = (cur + d).clamp(0, n as i32 - 1);
        if next == cur {
            return Outcome::Idle;
        }
        self.list.selected = Some(next as usize);
        self.list.ensure_visible(next as usize, self.canvas.h());
        Outcome::Redraw
    }

    fn print_selected(&self) {
        if let Some(i) = self.list.selected {
            if let Some(path) = self.tracks.get(i) {
                println!("play: {path}");
            }
        }
    }

    /// Full redraw into the canvas: clear, then draw retained elements.
    pub fn draw(&mut self) {
        self.canvas.clear(BG);
        self.list.draw(&mut self.canvas);
        self.canvas.mark_all();
    }
}
