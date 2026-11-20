// seek: a track bar plus its progress fill.

use crate::gfx::{Canvas, Rect};
use crate::theme::{FOREGROUND, SEEK_TRACK_BACKGROUND};

#[derive(Clone, Copy)]
pub struct SeekBar {
    pub rect: Rect,
}

impl SeekBar {
    pub fn progress_px(&self, pos: f64, len: f64) -> i32 {
        if len <= 0.0 || self.rect.w <= 0 {
            return 0;
        }
        ((pos / len).clamp(0.0, 1.0) * self.rect.w as f64) as i32
    }

    /// Fraction of the track under a click.
    pub fn frac_at(&self, x: i32) -> f64 {
        if self.rect.w <= 0 {
            return 0.0;
        }
        ((x - self.rect.x) as f64 / self.rect.w as f64).clamp(0.0, 1.0)
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        let r = self.rect;
        r.w > 0 && r.h > 0 && x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h
    }

    pub fn draw(&self, c: &mut Canvas, pos: f64, len: f64) {
        if self.rect.w <= 0 {
            return;
        }
        c.fill(self.rect, SEEK_TRACK_BACKGROUND);
        let wpx = self.progress_px(pos, len);
        if wpx > 0 {
            c.fill(Rect::new(self.rect.x, self.rect.y, wpx, self.rect.h), FOREGROUND);
        }
        c.mark(self.rect);
    }
}
