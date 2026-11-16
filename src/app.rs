// app: retained state + event handling. Draws only into the canvas.

use crate::audio;
use crate::library;
use crate::gfx::Canvas;
use crate::input::{Event, Key};
use crate::widget::{Item, List};

pub const BG: u8 = 0x18; // 8-bit luminance: 24,24,24
pub const FG: u8 = 0xF0; // 240,240,240

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
                bg: 0x20,
                alt_bg: 0x24,
                sel_bg: 0x40,
                fg: FG,
                sel_fg: FG,
                selected: if tracks.is_empty() { None } else { Some(0) },
                scroll: 0,
            },
            tracks,
        };
        app.layout();
        if !audio::init() {
            println!("audio: engine init failed");
        }
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
                    self.play_selected();
                    Outcome::Redraw
                }
                Key::Space => {
                    audio::toggle();
                    println!("audio: playing={}", audio::is_playing());
                    Outcome::Redraw
                }
                _ => Outcome::Idle,
            },
            Event::MouseDown { x, y } => {
                if let Some(i) = self.list.hit(x, y, self.canvas.w()) {
                    self.list.selected = Some(i);
                    self.play_selected();
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

    fn play_selected(&self) {
        let Some(path) = self.list.selected.and_then(|i| self.tracks.get(i)) else {
            return;
        };
        let ok = audio::play(path);
        let (pos, len) = audio::progress();
        println!("play {ok}: {path} ({pos:.1}/{len:.1}s)");
    }

    /// Full redraw into the canvas: clear, then draw retained elements.
    pub fn draw(&mut self) {
        self.canvas.clear(BG);
        self.list.draw(&mut self.canvas);
        self.canvas.mark_all();
    }
}
