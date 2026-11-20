// list: a stateless projection. Config + item labels + cursor in, pixels out.
// Scroll is derived from the cursor, so the UI owns no list state.

use crate::font;
use crate::gfx::{Canvas, Color, Rect};
use crate::theme::SEL_BAR;
use crate::widgets::art;

/// Row geometry + look. No state: same struct for every frame.
#[derive(Clone, Copy)]
pub struct ListView {
    pub area: Rect, // rows live here; y is the first row's top
    pub pad_x: i32,
    pub pad_y: i32,
    pub gap: i32,
    pub scale: i32,     // text scale
    pub art_scale: i32, // plank sprite scale
    pub fg: Color,
    pub sel_fg: Color,
}

impl ListView {
    pub fn row_h(&self) -> i32 {
        font::LINE_HEIGHT * self.scale + 2 * self.pad_y
    }

    pub fn stride(&self) -> i32 {
        self.row_h() + self.gap
    }

    pub fn visible(&self) -> usize {
        (self.area.h.max(0) / self.stride()) as usize
    }

    /// First visible row for a cursor: derived, never stored.
    pub fn scroll_for(&self, selected: Option<usize>) -> usize {
        let vis = self.visible().max(1);
        match selected {
            Some(i) if i >= vis => i + 1 - vis,
            _ => 0,
        }
    }

    pub fn inner_x(&self) -> i32 {
        self.area.x + self.pad_x
    }

    pub fn row_width(&self) -> i32 {
        (self.area.w - 2 * self.pad_x).max(0)
    }

    pub fn row(&self, i: usize, scroll: usize) -> Rect {
        Rect::new(
            self.inner_x(),
            self.area.y + (i - scroll) as i32 * self.stride(),
            self.row_width(),
            self.row_h(),
        )
    }

    /// Index under a design-space point, honouring the current scroll.
    pub fn hit(&self, x: i32, y: i32, len: usize, scroll: usize) -> Option<usize> {
        if x < self.inner_x() || x >= self.inner_x() + self.row_width() {
            return None;
        }
        let dy = y - self.area.y;
        if dy < 0 {
            return None;
        }
        let i = scroll + (dy / self.stride()) as usize;
        if i >= len || dy % self.stride() >= self.row_h() {
            return None;
        }
        Some(i)
    }

    pub fn draw(&self, c: &mut Canvas, items: &[&str], selected: Option<usize>) {
        let scroll = self.scroll_for(selected);
        for (i, label) in items.iter().enumerate().skip(scroll) {
            let r = self.row(i, scroll);
            if r.y >= self.area.y + self.area.h {
                break;
            }
            art::draw_row_art(c, r, self.art_scale);
            if selected == Some(i) {
                // no selected art yet: a hot left accent reads as focus
                c.fill(
                    Rect::new(r.x + 2 * self.art_scale, r.y, self.art_scale, r.h),
                    SEL_BAR,
                );
            }
            let ink = font::ink_bounds(label, self.scale);
            font::draw_wood(
                c,
                label,
                r.x + (r.w - ink.w) / 2 - ink.x, // centred in the item, by ink
                r.y + (r.h - ink.h) / 2 - ink.y,
                self.scale,
            );
        }
    }
}
