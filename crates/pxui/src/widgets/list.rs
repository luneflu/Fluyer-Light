// list: a stateless projection. Config + item labels + cursor in, pixels out.
// Scroll is derived from the cursor, so the UI owns no list state.

use crate::font;
use crate::gfx::{Canvas, Rect};
use crate::theme::SELECTION_ACCENT;
use crate::widgets::art;

/// Row geometry + look. No state: same struct for every frame.
#[derive(Clone, Copy)]
pub struct ListView {
    pub area: Rect, // rows live here; y is the first row's top
    pub pad_x: i32,
    pub pad_y: i32,
    pub gap: i32,
    pub text_scale: i32,   // font atlas px -> device px
    pub sprite_scale: i32, // plank sprite art px -> device px
}

impl ListView {
    pub fn row_h(&self) -> i32 {
        font::LINE_HEIGHT * self.text_scale + 2 * self.pad_y
    }

    pub fn stride(&self) -> i32 {
        self.row_h() + self.gap
    }

    pub fn visible(&self) -> usize {
        (self.area.h.max(0) / self.stride()) as usize
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

    pub fn draw(&self, c: &mut Canvas, items: &[&str], selected: Option<usize>, scroll: usize) {
        for (i, label) in items.iter().enumerate().skip(scroll) {
            let r = self.row(i, scroll);
            if r.y >= self.area.y + self.area.h {
                break;
            }
            art::draw_row_art(c, r);
            if selected == Some(i) {
                // no selected art yet: a hot left accent reads as focus
                c.fill(
                    Rect::new(r.x + 2 * self.sprite_scale, r.y, self.sprite_scale, r.h),
                    SELECTION_ACCENT,
                );
            }
            let ink = font::ink_bounds(label, self.text_scale);
            font::draw_wood(
                c,
                label,
                r.x + (r.w - ink.w) / 2 - ink.x, // centred in the item, by ink
                r.y + (r.h - ink.h) / 2 - ink.y,
                self.text_scale,
            );
        }
    }
}
