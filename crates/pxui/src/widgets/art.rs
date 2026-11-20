// art: the row plank sprite, 9-sliced into a row box.

use crate::buttons;
use crate::gfx::{Canvas, Rect};

/// Top/bottom art rows that stay fixed; the band between them repeats.
pub const CAP_T: i32 = 8;
pub const CAP_B: i32 = 8;

/// Draw the item plank into `r`, growing in both axes without stretching bevels.
pub fn draw_row_art(c: &mut Canvas, r: Rect, scale: i32) {
    let t = (buttons::MUSIC_ITEM_W * buttons::MUSIC_ITEM_H) as usize;
    let art = &buttons::MUSIC_ITEM;
    c.blit_9slice(
        &art[0..t],
        &art[t..2 * t],
        &art[2 * t..3 * t],
        buttons::MUSIC_ITEM_W,
        buttons::MUSIC_ITEM_H,
        CAP_T,
        CAP_B,
        (r.x, r.y),
        r.w,
        r.h,
        scale,
    );
}
