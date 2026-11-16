// font: draws the baked Cairopixel atlas (src/font_atlas.rs) as integer-scaled pixel rects.

use crate::font_atlas::{ASCENT, BITMAP, DESCENT, FIRST, GLYPHS};
use crate::gfx::{Canvas, Rect};

#[allow(dead_code)]
pub const LINE_HEIGHT: i32 = ASCENT + DESCENT;

fn glyph(c: char) -> &'static crate::font_atlas::Glyph {
    let idx = (c as u32).wrapping_sub(FIRST as u32) as usize;
    &GLYPHS[idx.min(GLYPHS.len() - 1)]
}

#[allow(dead_code)]
pub fn text_width(text: &str, scale: i32) -> i32 {
    text.chars().map(|c| glyph(c).adv as i32 * scale).sum()
}

#[allow(dead_code)]
pub fn text_height(scale: i32) -> i32 {
    LINE_HEIGHT * scale
}

/// Ink bounds of `text` relative to the draw origin (ascender line).
/// Line metrics lie (descent + side bearings); centring needs real ink.
pub fn ink_bounds(text: &str, scale: i32) -> Rect {
    let mut pen = 0;
    let (mut x0, mut y0) = (i32::MAX, i32::MAX);
    let (mut x1, mut y1) = (i32::MIN, i32::MIN);
    for c in text.chars() {
        let g = glyph(c);
        let (gw, gh) = (g.w as i32 * scale, g.h as i32 * scale);
        if gw > 0 && gh > 0 {
            x0 = x0.min(pen + g.x0 as i32 * scale);
            y0 = y0.min(g.y0 as i32 * scale);
            x1 = x1.max(pen + (g.x0 as i32 * scale + gw));
            y1 = y1.max(g.y0 as i32 * scale + gh);
        }
        pen += g.adv as i32 * scale;
    }
    if x0 == i32::MAX {
        return Rect::default();
    }
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

/// `y` is the ascender line (top of the line box), not the glyph top.
pub fn draw(canvas: &mut Canvas, text: &str, x: i32, y: i32, scale: i32, color: u32) {
    let mut pen = x;
    for c in text.chars() {
        let g = glyph(c);
        let (gw, gh) = (g.w as i32, g.h as i32);
        for row in 0..gh {
            for col in 0..gw {
                let bit = BITMAP[(g.off as usize) + (row * gw + col) as usize];
                if bit == 0 {
                    continue;
                }
                canvas.fill(
                    Rect::new(
                        pen + (g.x0 as i32 + col) * scale,
                        y + (g.y0 as i32 + row) * scale,
                        scale,
                        scale,
                    ),
                    color,
                );
            }
        }
        pen += g.adv as i32 * scale;
    }
}
