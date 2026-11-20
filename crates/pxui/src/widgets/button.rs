// button: a rect sized from its label (or fixed for icon buttons) with a centred label.

use crate::font;
use crate::gfx::{Canvas, Color, Rect};

#[derive(Clone, Copy)]
pub struct Button {
    pub x: i32, // top-left, design px
    pub y: i32,
    pub label: &'static str,
    pub scale: i32, // text scale (font em x scale)
    pub pad_x: i32,
    pub pad_y: i32,
    pub bg: Color,
    pub border: Color,
    pub border_w: i32,
    pub fg: Color,
    pub fixed_w: i32, // > 0: ignore text-derived size (square icon buttons)
    pub fixed_h: i32,
}

impl Button {
    /// Text width + horizontal padding. Never stored: resize/relabel free.
    pub fn w(&self) -> i32 {
        if self.fixed_w > 0 {
            return self.fixed_w;
        }
        font::text_width(self.label, self.scale) + 2 * self.pad_x
    }

    /// Line height + vertical padding.
    pub fn h(&self) -> i32 {
        if self.fixed_h > 0 {
            return self.fixed_h;
        }
        font::LINE_HEIGHT * self.scale + 2 * self.pad_y
    }

    pub fn rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.w(), self.h())
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w() && py < self.y + self.h()
    }

    /// Keep the whole button on canvas after a shrink.
    pub fn clamp(&mut self, cw: i32, ch: i32) {
        let (w, h) = (self.w(), self.h());
        self.x = self.x.clamp(0, (cw - w).max(0));
        self.y = self.y.clamp(0, (ch - h).max(0));
    }

    pub fn draw(&self, c: &mut Canvas) {
        let r = self.rect();
        c.fill(r, self.bg); // 0 = no chrome: sprite only

        let bw = self.border_w.max(0).min(r.w / 2).min(r.h / 2);
        if bw > 0 {
            c.fill(Rect::new(r.x, r.y, r.w, bw), self.border);
            c.fill(Rect::new(r.x, r.y + r.h - bw, r.w, bw), self.border);
            c.fill(Rect::new(r.x, r.y, bw, r.h), self.border);
            c.fill(Rect::new(r.x + r.w - bw, r.y, bw, r.h), self.border);
        }

        // Centre by ink, not by line metrics (side bearings lie).
        let ink = font::ink_bounds(self.label, self.scale);
        let tx = r.x + (r.w - ink.w) / 2 - ink.x;
        let ty = r.y + (r.h - ink.h) / 2 - ink.y;
        font::draw(c, self.label, tx, ty, self.scale, self.fg);
    }
}
