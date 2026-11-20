// view: pure projection. Model + Layout in, pixels out. Owns no state.

use crate::core::library;
use crate::core::model::Model;
use fluyer_pxui::font;
use fluyer_pxui::gfx::{Canvas, Rect};
use crate::ui::layout::Layout;
use fluyer_pxui::theme::{BAR_BG, BG, FG, PAD, SCALE};
use fluyer_pxui::widgets::icons::{self, F_NEXT, F_PAUSE, F_PLAY, F_PREV};
use fluyer_pxui::widgets::list::ListView;

/// Which control is held down, so its clicked frame can be drawn.
#[derive(Clone, Copy, PartialEq)]
pub enum Press {
    None,
    Play,
    Prev,
    Next,
}

/// What the pointer landed on. The app turns this into a `core::model::Action`.
#[derive(Clone, Copy, Debug)]
pub enum Hit {
    Transport(usize), // 0 prev, 1 play, 2 next
    Seek(f64),        // 0..1
    Row(usize),
}

pub fn hit(l: &Layout, list: &ListView, model: &Model, x: i32, y: i32) -> Option<Hit> {
    for slot in 0..3 {
        if l.slot(slot).contains(x, y) {
            return Some(Hit::Transport(slot as usize));
        }
    }
    if l.seek.contains(x, y) {
        return Some(Hit::Seek(l.seek.frac_at(x)));
    }
    let scroll = list.scroll_for(model.selected);
    list.hit(x, y, model.len(), scroll).map(Hit::Row)
}

pub fn render(
    model: &Model,
    list: &ListView,
    l: &Layout,
    press: Press,
    progress: (f64, f64),
    c: &mut Canvas,
) {
    c.clear(BG);

    let labels: Vec<&str> = model.tracks.iter().map(|p| library::label(p)).collect();
    list.draw(c, &labels, model.selected);

    c.fill(
        Rect::new(0, l.h - fluyer_pxui::theme::BAR_H, l.w, fluyer_pxui::theme::BAR_H),
        BAR_BG,
    );

    // Track name, clipped to the room left of the buttons.
    if let Some(path) = model.current_path() {
        let room = l.name_room();
        let name = fit(library::label(path), room, SCALE);
        let ink = font::ink_bounds(&name, SCALE);
        font::draw(c, &name, PAD, l.row_y + (l.icon - ink.h) / 2 - ink.y, SCALE, FG);
    }

    let down = |hit: Press| (press == hit) as usize;
    icons::draw(c, l.slot(0), F_PREV + down(Press::Prev));
    icons::draw(
        c,
        l.slot(1),
        if model.player.playing { F_PAUSE } else { F_PLAY } + down(Press::Play),
    );
    icons::draw(c, l.slot(2), F_NEXT + down(Press::Next));

    l.seek.draw(c, progress.0, progress.1);
    c.mark_all();
}

/// Seek strip only: a playing tick costs one strip, not a full frame.
pub fn render_progress(l: &Layout, progress: (f64, f64), c: &mut Canvas) {
    l.seek.draw(c, progress.0, progress.1);
}

/// Longest prefix that fits `room` design px, ellipsised with "..".
fn fit(text: &str, room: i32, scale: i32) -> String {
    if font::text_width(text, scale) <= room {
        return text.to_string();
    }
    let mut out = String::new();
    for ch in text.chars() {
        let mut probe = out.clone();
        probe.push(ch);
        probe.push_str("..");
        if font::text_width(&probe, scale) > room {
            break;
        }
        out.push(ch);
    }
    out.push_str("..");
    out
}
