// art: the row plank sprite, 9-sliced into a row box.

use crate::buttons;
use crate::gfx::{Canvas, Rect};

/// Corner/edge band that stays fixed; the middle repeats.
pub const CAP: i32 = buttons::MUSIC_ITEM_W / 3;
pub const PLANK: i32 = 2;

/// Draw the item plank into `r`, growing in both axes without stretching bevels.
/// The sheet is one 72×72 plank at 1:1, so a row is 3×24 caps plus a repeating middle.
pub fn draw_row_art(c: &mut Canvas, r: Rect) {
    c.blit_9(
        &buttons::MUSIC_ITEM,
        buttons::MUSIC_ITEM_W,
        buttons::MUSIC_ITEM_H,
        CAP,
        (r.x, r.y),
        r.w,
        r.h,
        PLANK,
    );
}
