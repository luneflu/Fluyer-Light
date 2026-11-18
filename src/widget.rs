// widget: retained widgets drawn into the canvas. Position is absolute design px.

use crate::buttons;
use crate::font;
use crate::gfx::{Canvas, Color, Rect};

/// Accent for the selected row: the light bevel colour of the item art.
const SEL_BAR: Color = 0xFFE4_9C6F;

#[derive(Clone, Copy)]
pub struct Button {
    pub x: i32, // top-left, design px
    pub y: i32,
    pub label: &'static str,
    pub scale: i32,
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
        c.fill(r, self.bg); // 0 = no chrome, sprite only

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

/// One row. Label only for now; add fields when rows carry state.
#[derive(Clone)]
pub struct Item {
    pub label: String,
}

/// Vertical stack of items. Row height derives from the font, so no stored sizes.
pub struct List {
    pub x: i32,
    pub y: i32,
    pub w: i32, // <= 0: fill canvas width minus the left/right margin
    pub items: Vec<Item>,
    pub scale: i32,
    pub pad_x: i32,
    pub pad_y: i32,
    pub gap: i32,
    pub fg: Color,
    pub sel_fg: Color,
    pub selected: Option<usize>,
    pub scroll: usize, // first visible row
    pub view_h: i32,   // rows are clipped below this y (0 = whole canvas)
}

impl List {
    pub fn row_h(&self) -> i32 {
        font::LINE_HEIGHT * self.scale + 2 * self.pad_y
    }

    pub fn stride(&self) -> i32 {
        self.row_h() + self.gap
    }

    /// Rows that fit between `y` and the canvas bottom.
    pub fn visible(&self, ch: i32) -> usize {
        let bottom = if self.view_h > 0 { self.y + self.view_h } else { ch };
        ((bottom - self.y).max(0) / self.stride()) as usize
    }

    /// Scroll the least amount that keeps row `i` on screen.
    pub fn ensure_visible(&mut self, i: usize, ch: i32) {
        let vis = self.visible(ch).max(1);
        if i < self.scroll {
            self.scroll = i;
        } else if i >= self.scroll + vis {
            self.scroll = i + 1 - vis;
        }
        self.scroll = self.scroll.min(self.items.len().saturating_sub(1));
    }

    pub fn width(&self, canvas_w: i32) -> i32 {
        if self.w > 0 {
            self.w
        } else {
            (canvas_w - 2 * self.x).max(0)
        }
    }

    pub fn rect(&self, canvas_w: i32) -> Rect {
        let n = (self.items.len() - self.scroll.min(self.items.len())) as i32;
        let h = if n <= 0 { 0 } else { n * self.row_h() + (n - 1) * self.gap };
        Rect::new(self.x, self.y, self.width(canvas_w), h)
    }

    pub fn row(&self, i: usize, canvas_w: i32) -> Rect {
        Rect::new(
            self.x,
            self.y + (i - self.scroll) as i32 * self.stride(),
            self.width(canvas_w),
            self.row_h(),
        )
    }

    /// Which row contains a design-space point (gap counts as dead space).
    pub fn hit(&self, px: i32, py: i32, canvas_w: i32) -> Option<usize> {
        let w = self.width(canvas_w);
        if px < self.x || px >= self.x + w {
            return None;
        }
        let dy = py - self.y;
        if dy < 0 {
            return None;
        }
        let i = self.scroll + (dy / self.stride()) as usize;
        if i >= self.items.len() || dy % self.stride() >= self.row_h() {
            return None;
        }
        Some(i)
    }

    pub fn clamp(&mut self, cw: i32, ch: i32) {
        let (w, h) = (self.width(cw), self.rect(cw).h);
        self.x = self.x.clamp(0, (cw - w).max(0));
        self.y = self.y.clamp(0, (ch - h).max(0));
    }

    pub fn draw(&self, c: &mut Canvas) {
        let cw = c.w();
        for i in self.scroll..self.items.len() {
            let item = &self.items[i];
            let r = self.row(i, cw);
            if r.y >= c.h() || (self.view_h > 0 && r.y + r.h > self.y + self.view_h) {
                break;
            }
            let sel = self.selected == Some(i);
            let t = (buttons::MUSIC_ITEM_W * buttons::MUSIC_ITEM_H) as usize;
            let art = &buttons::MUSIC_ITEM;
            c.blit_sliced(
                &art[0..t],
                &art[t..2 * t],
                &art[2 * t..3 * t],
                buttons::MUSIC_ITEM_W,
                buttons::MUSIC_ITEM_H,
                // vertically centred in the row, whatever pad_y is
                (r.x, r.y + (r.h - buttons::MUSIC_ITEM_H * self.scale) / 2),
                r.w,
                self.scale,
            );
            if sel {
                // no selected art yet: a hot left accent reads as focus
                c.fill(Rect::new(r.x + 2 * self.scale, r.y, self.scale, r.h), SEL_BAR);
            }
            let ink = font::ink_bounds(&item.label, self.scale);
            font::draw_wood(
                c,
                &item.label,
                r.x + (r.w - ink.w) / 2 - ink.x, // centred in the item, by ink
                r.y + (r.h - ink.h) / 2 - ink.y,
                self.scale,
            );
        }
    }
}
