// icons: transport frames from controls.png, one art px = SPRITE_SCALE device px.

use crate::buttons;
use crate::gfx::{Canvas, Rect};
use crate::theme::SPRITE_SCALE;

// controls.png frames: 0 play, 1 play-down, 2 pause, 3 pause-down,
// 4 next, 5 next-down, 6 prev, 7 prev-down.
pub const F_PLAY: usize = 0;
pub const F_PAUSE: usize = 2;
pub const F_NEXT: usize = 4;
pub const F_PREV: usize = 6;

/// Draw one control frame into `r`, scaled to fill it.
pub fn draw(c: &mut Canvas, r: Rect, frame: usize) {
    const N: usize = (buttons::CONTROLS_W * buttons::CONTROLS_H) as usize;
    c.blit_masked(
        &buttons::CONTROLS[frame * N..(frame + 1) * N],
        buttons::CONTROLS_W,
        buttons::CONTROLS_H,
        (r.x, r.y),
        (r.w / buttons::CONTROLS_W).max(SPRITE_SCALE),
    );
}
