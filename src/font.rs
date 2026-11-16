// font: draws the baked Cairopixel atlas (src/font_atlas.rs) as integer-scaled pixel rects.

use crate::font_atlas::{ASCENT, BITMAP, DESCENT, FIRST, GLYPHS};
use crate::gfx::{Canvas, Rect};

pub const LINE_HEIGHT: i32 = ASCENT + DESCENT;

fn glyph(c: char) -> &'static crate::font_atlas::Glyph {
    let idx = (c as u32).wrapping_sub(FIRST as u32) as usize;
    &GLYPHS[idx.min(GLYPHS.len() - 1)]
}

pub fn text_width(text: &str, scale: i32) -> i32 {
    text.chars().map(|c| glyph(c).adv as i32 * scale).sum()
}

pub fn text_height(scale: i32) -> i32 {
    LINE_HEIGHT * scale
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
