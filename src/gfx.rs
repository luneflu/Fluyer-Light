// gfx: a fixed 32-bit colour canvas over the platform surface. No allocator, no traits.
// Colours are packed 0xAARRGGBB and stored BGRA, so the bytes match every backend
// (Windows DIB32 and IOSurface 'BGRA') without a conversion pass.

/// Packed colour. 0 is fully transparent and is skipped by `fill`/blits.
pub type Color = u32;

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

/// Opaque grey, for the chrome that stays monochrome.
pub const fn grey(v: u8) -> Color {
    rgb(v, v, v)
}

#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn union(self, o: Rect) -> Rect {
        if self.w <= 0 || self.h <= 0 {
            return o;
        }
        if o.w <= 0 || o.h <= 0 {
            return self;
        }
        let x0 = self.x.min(o.x);
        let y0 = self.y.min(o.y);
        let x1 = (self.x + self.w).max(o.x + o.w);
        let y1 = (self.y + self.h).max(o.y + o.h);
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }
}

pub struct Canvas {
    w: i32,
    h: i32,
    stride: i32,   // pixels per row in the backing surface (>= w)
    px: *mut u32,
    dirty: Rect,
}

impl Canvas {
    /// SAFETY: `px` must point at `w * h` contiguous pixels for the process lifetime,
    /// `stride` pixels apart per row.
    pub unsafe fn new(px: *mut u8, w: i32, h: i32, stride: i32) -> Self {
        Self { w, h, stride, px: px as *mut u32, dirty: Rect::default() }
    }

    pub fn stride(&self) -> i32 {
        self.stride
    }

    pub fn w(&self) -> i32 {
        self.w
    }

    pub fn h(&self) -> i32 {
        self.h
    }

    #[allow(dead_code)] // for partial redraws; app.draw currently marks all
    pub fn mark(&mut self, r: Rect) {
        self.dirty = self.dirty.union(self.clip(r));
    }

    pub fn mark_all(&mut self) {
        self.dirty = self.dirty.union(Rect::new(0, 0, self.w, self.h));
    }

    #[allow(dead_code)]
    pub fn dirty(&self) -> Rect {
        self.dirty
    }

    pub fn take_dirty(&mut self) -> Rect {
        std::mem::take(&mut self.dirty)
    }

    fn clip(&self, r: Rect) -> Rect {
        let x0 = r.x.max(0);
        let y0 = r.y.max(0);
        let x1 = (r.x + r.w).min(self.w);
        let y1 = (r.y + r.h).min(self.h);
        Rect::new(x0, y0, (x1 - x0).max(0), (y1 - y0).max(0))
    }

    fn row(&mut self, y: i32) -> &mut [u32] {
        // SAFETY: y is clipped to [0, h) by callers; rows live `stride` pixels apart.
        unsafe { std::slice::from_raw_parts_mut(self.px.add((y * self.stride) as usize), self.w as usize) }
    }

    /// Solid fill. One `fill` per row; fully transparent colour is a no-op.
    pub fn fill(&mut self, r: Rect, color: Color) {
        if color == 0 {
            return;
        }
        let r = self.clip(r);
        if r.w == 0 || r.h == 0 {
            return;
        }
        for y in r.y..r.y + r.h {
            self.row(y)[r.x as usize..(r.x + r.w) as usize].fill(color);
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.fill(Rect::new(0, 0, self.w, self.h), color);
    }

    /// Copy a same-format w*h image at (x, y), clipped.
    #[allow(dead_code)]
    pub fn blit(&mut self, src: &[u32], sw: i32, sh: i32, at: (i32, i32)) {
        for sy in 0..sh {
            let dy = at.1 + sy;
            if dy < 0 || dy >= self.h {
                continue;
            }
            let dst_x = at.0;
            let sx0 = (-dst_x).max(0);
            let sx1 = sw.min(self.w - dst_x);
            if sx0 >= sx1 {
                continue;
            }
            let s = &src[(sy * sw + sx0) as usize..(sy * sw + sx1) as usize];
            let row_start = (dst_x + sx0) as usize;
            self.row(dy)[row_start..row_start + s.len()].copy_from_slice(s);
        }
    }

    /// 1-bit glyph blit: bit set -> fg, clear -> nothing (bg untouched).
    #[allow(dead_code)]
    pub fn blit_1bpp(&mut self, src: &[u8], sw: i32, sh: i32, at: (i32, i32), fg: Color) {
        for sy in 0..sh {
            let dy = at.1 + sy;
            if dy < 0 || dy >= self.h {
                continue;
            }
            let bits = &src[(sy * sw) as usize..(sy * sw + sw) as usize];
            for sx in 0..sw {
                if bits[sx as usize] == 0 {
                    continue;
                }
                let dx = at.0 + sx;
                if dx < 0 || dx >= self.w {
                    continue;
                }
                self.row(dy)[dx as usize] = fg;
            }
        }
    }

    /// Sprite blit with integer nearest upscale; 0 means transparent.
    pub fn blit_masked(&mut self, src: &[u32], sw: i32, sh: i32, at: (i32, i32), scale: i32) {
        let scale = scale.max(1);
        for sy in 0..sh {
            for sx in 0..sw {
                let px = src[(sy * sw + sx) as usize];
                if px == 0 {
                    continue;
                }
                self.fill(Rect::new(at.0 + sx * scale, at.1 + sy * scale, scale, scale), px);
            }
        }
    }
}
