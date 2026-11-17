// app: retained state + event handling. Draws only into the canvas.

use crate::audio;
use crate::buttons;
use crate::font;
use crate::gfx::{grey, rgb, Color, Canvas, Rect};
use crate::input::{Event, Key};
use crate::library;
use crate::widget::{Button, Item, List};

pub const BG: Color = grey(0x18); // 24,24,24
pub const FG: Color = grey(0xF0); // 240,240,240
const BAR_BG: Color = grey(0x1C);

/// List row ink: the dark bevel brown of the item art, not the chrome FG.
const ITEM_FG: Color = rgb(0x50, 0x22, 0x05);
const TRACK_BG: Color = grey(0x30);

/// Library root. Windows has no fixed user path, so derive it from the profile
/// and fall back to the working directory when the folder is missing.
#[cfg(target_os = "macos")]
fn music_dir() -> String {
    "/Users/alvindimas05/Music/The Meaning of Life".to_string()
}

#[cfg(not(target_os = "macos"))]
fn music_dir() -> String {
    // let base = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    // let music = format!("{base}\\Music");
    // if std::path::Path::new(&format!("{music}\\The Meaning of Life")).is_dir() {
    //     format!("{music}\\The Meaning of Life")
    // } else {
    //     music
    // }
    "E:\\Music".to_string()
}
const SCALE: i32 = 4;
const PAD: i32 = 8;

// Bottom bar: seek track on top, then name + transport row.
const NAME_H: i32 = 15 * SCALE; // one line of text
const SEEK_H: i32 = 8 * SCALE;
const GAP: i32 = 8;
const EM: i32 = 16;              // baked font em; play/pause is one em square
const ICON: i32 = EM * SCALE;
const BAR_H: i32 = PAD + SEEK_H + GAP + ICON + PAD;

pub enum Outcome {
    Idle,
    Redraw,
    Quit,
}

/// Which control is held down, so its clicked frame can be drawn.
#[derive(Clone, Copy, PartialEq)]
enum Hit {
    None,
    Play,
    Prev,
    Next,
}

// controls.png frames: 0 play, 1 play-down, 2 pause, 3 pause-down,
// 4 next, 5 next-down, 6 prev, 7 prev-down.
const F_PLAY: usize = 0;
const F_PAUSE: usize = 2;
const F_NEXT: usize = 4;
const F_PREV: usize = 6;

fn ctrl(c: &mut Canvas, b: &Button, frame: usize) {
    const N: usize = (buttons::CONTROLS_W * buttons::CONTROLS_H) as usize;
    let r = b.rect();
    c.blit_masked(
        &buttons::CONTROLS[frame * N..(frame + 1) * N],
        buttons::CONTROLS_W,
        buttons::CONTROLS_H,
        (r.x, r.y),
        ICON / buttons::CONTROLS_H,
    );
}

fn button(label: &'static str) -> Button {
    Button {
        x: 0,
        y: 0,
        label,
        scale: SCALE,
        pad_x: 8,
        pad_y: 4,
        bg: 0, // no chrome: the sprite is the button
        border: 0,
        border_w: 0,
        fg: FG,
        fixed_w: 0,
        fixed_h: 0,
    }
}

pub struct App {
    pub canvas: Canvas,
    list: List,
    tracks: Vec<String>, // full path per row, same order as list.items
    play: Button,
    prev: Button,
    next: Button,
    seek: Rect,   // seek track rect, design space
    playing: bool,
    press: Hit, // held control: draws its clicked frame
    current: Option<usize>,   // index of the loaded/playing track, not the list cursor
    inset: i32,               // titlebar height to keep clear (traffic lights live there)
}

impl App {
    pub fn new(w: i32, h: i32) -> Self {
        let canvas = Canvas::new(w, h);
        let music = music_dir();
        let tracks = library::scan(&music);
        let items = tracks
            .iter()
            .map(|p| Item { label: library::label(p).to_string() })
            .collect();
        println!("{} audio files in {music}", tracks.len());
        let mut app = Self {
            canvas,
            list: List {
                x: 0,
                y: 0,
                w: 0,
                items,
                scale: SCALE,
                pad_x: 8,
                pad_y: (buttons::MUSIC_ITEM_H - font::LINE_HEIGHT) * SCALE / 2, // art height
                gap: 2,
                fg: ITEM_FG,
                sel_fg: ITEM_FG,
                selected: if tracks.is_empty() { None } else { Some(0) },
                scroll: 0,
                view_h: 0,
            },
            tracks,
            play: Button { fixed_w: ICON, fixed_h: ICON, pad_x: 0, pad_y: 0, ..button(">") },
            prev: Button { fixed_w: ICON, fixed_h: ICON, pad_x: 0, pad_y: 0, ..button("<") },
            next: Button { fixed_w: ICON, fixed_h: ICON, pad_x: 0, pad_y: 0, ..button(">") },
            seek: Rect::new(0, 0, 0, 0),
            playing: false,
            press: Hit::None,
            current: None,
            inset: 0,
        };
        if !audio::init() {
            println!("audio: engine init failed");
        }
        app.layout();
        app
    }

    /// New window size: reallocate the surface and keep geometry on-canvas.
    pub fn resize(&mut self, w: i32, h: i32) {
        self.canvas = Canvas::new(w, h);
        self.layout();
    }

    /// Height of the transparent titlebar strip the shell keeps above us.
    pub fn set_inset(&mut self, px: i32) {
        if px != self.inset {
            self.inset = px;
            self.layout();
        }
    }

    fn layout(&mut self) {
        let (w, h) = (self.canvas.w(), self.canvas.h());
        self.list.y = self.inset;
        self.list.view_h = (h - BAR_H - self.inset).max(0);
        if let Some(i) = self.list.selected {
            self.list.ensure_visible(i, h);
        }

        // Seek track on top, transport row under it.
        self.seek = Rect::new(PAD, h - BAR_H + PAD, (w - 2 * PAD).max(0), SEEK_H);
        let row_y = self.seek.y + SEEK_H + GAP;

        // Right-aligned, one em squares: [prev][play][next]
        self.prev.label = "<";
        self.next.label = ">";
        self.next.x = w - PAD - self.next.w();
        self.play.x = self.next.x - GAP - self.play.w();
        self.prev.x = self.play.x - GAP - self.prev.w();
        self.play.y = row_y;
        self.prev.y = row_y;
        self.next.y = row_y;
    }

    fn name_room(&self) -> i32 {
        self.play.x - GAP - PAD
    }

    pub fn handle(&mut self, ev: Event) -> Outcome {
        match ev {
            Event::Key { key, mods } => match key {
                Key::Escape => Outcome::Quit,
                Key::Q if mods.cmd => Outcome::Quit,
                Key::Up => self.move_by(-1),
                Key::Down => self.move_by(1),
                Key::Enter => self.play_selected(),
                Key::Space => self.toggle(),
                _ => Outcome::Idle,
            },
            Event::Tick => {
                self.playing = audio::is_playing();
                if self.playing {
                    Outcome::Redraw
                } else {
                    Outcome::Idle
                }
            }
            Event::MouseDown { x, y } => {
                if self.play.contains(x, y) {
                    self.press = Hit::Play;
                    return self.toggle();
                }
                if self.prev.contains(x, y) {
                    self.press = Hit::Prev;
                    return self.step_track(-1);
                }
                if self.next.contains(x, y) {
                    self.press = Hit::Next;
                    return self.step_track(1);
                }
                if self.seek.w > 0 && self.seek.h > 0 && inside(self.seek, x, y) {
                    let (_, len) = audio::progress();
                    let f = ((x - self.seek.x) as f64 / self.seek.w as f64).clamp(0.0, 1.0);
                    audio::seek(f * len);
                    return Outcome::Redraw;
                }
                if let Some(i) = self.list.hit(x, y, self.canvas.w()) {
                    self.list.selected = Some(i);
                    return self.play_selected();
                }
                Outcome::Idle
            }
            Event::MouseUp => {
                if self.press == Hit::None {
                    return Outcome::Idle;
                }
                self.press = Hit::None;
                Outcome::Redraw
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

    /// prev/next move relative to the playing track and start it.
    fn step_track(&mut self, d: i32) -> Outcome {
        let n = self.list.items.len();
        if n == 0 {
            return Outcome::Idle;
        }
        let cur = self.current.or(self.list.selected).unwrap_or(0) as i32;
        let next = (cur + d).clamp(0, n as i32 - 1);
        self.list.selected = Some(next as usize);
        self.list.ensure_visible(next as usize, self.canvas.h());
        self.play_selected()
    }

    fn play_selected(&mut self) -> Outcome {
        let Some(path) = self.list.selected.and_then(|i| self.tracks.get(i)) else {
            return Outcome::Idle;
        };
        let ok = audio::play(path);
        self.playing = audio::is_playing();
        self.current = self.list.selected;
        let (_, len) = audio::progress();
        println!("play {ok}: {path} ({len:.1}s)");
        Outcome::Redraw
    }

    fn toggle(&mut self) -> Outcome {
        if audio::progress().1 == 0.0 {
            return self.play_selected(); // nothing loaded yet
        }
        audio::toggle();
        self.playing = audio::is_playing();
        println!("audio: playing={}", self.playing);
        Outcome::Redraw
    }

    /// Full redraw into the canvas: clear, then draw retained elements.
    pub fn draw(&mut self) {
        self.canvas.clear(BG);
        self.list.draw(&mut self.canvas);

        let (w, h) = (self.canvas.w(), self.canvas.h());
        self.canvas.fill(Rect::new(0, h - BAR_H, w, BAR_H), BAR_BG);

        // Track name, clipped to the room left of the buttons.
        if let Some(path) = self.current.and_then(|i| self.tracks.get(i)) {
            let room = self.name_room();
            let name = fit(library::label(path), room, SCALE);
            let ink = font::ink_bounds(&name, SCALE);
            font::draw(
                &mut self.canvas,
                &name,
                PAD,
                self.play.y + (ICON - ink.h) / 2 - ink.y,
                SCALE,
                FG,
            );
        }

        let down = |hit: Hit| (self.press == hit) as usize;
        ctrl(&mut self.canvas, &self.prev, F_PREV + down(Hit::Prev));
        ctrl(
            &mut self.canvas,
            &self.play,
            if self.playing { F_PAUSE } else { F_PLAY } + down(Hit::Play),
        );
        ctrl(&mut self.canvas, &self.next, F_NEXT + down(Hit::Next));

        // Seek track + progress.
        if self.seek.w > 0 {
            self.canvas.fill(self.seek, TRACK_BG);
            let (pos, len) = audio::progress();
            if len > 0.0 {
                let f = (pos / len).clamp(0.0, 1.0);
                let wpx = (f * self.seek.w as f64) as i32;
                if wpx > 0 {
                    self.canvas.fill(Rect::new(self.seek.x, self.seek.y, wpx, self.seek.h), FG);
                }
            }
        }
        self.canvas.mark_all();
    }
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h
}

/// Longest prefix that fits `room` design px, ellipsised with "..".
fn fit(text: &str, room: i32, scale: i32) -> String {
    if font::text_width(text, scale) <= room {
        return text.to_string();
    }
    let mut out = String::new();
    for c in text.chars() {
        let mut probe = out.clone();
        probe.push(c);
        probe.push_str("..");
        if font::text_width(&probe, scale) > room {
            break;
        }
        out.push(c);
    }
    out.push_str("..");
    out
}
