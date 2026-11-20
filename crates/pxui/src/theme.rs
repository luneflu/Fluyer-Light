// theme: palette + metrics. The only place raw colours and sizes live.

use crate::font;
use crate::gfx::{grey, rgb, Color};

pub const BG: Color = grey(0x18);
pub const FG: Color = grey(0xF0);
pub const BAR_BG: Color = grey(0x1C);
pub const TRACK_BG: Color = grey(0x30);

/// List row ink: the item art's wood ramp, dark top-left to lit bottom-right.
pub const ITEM_FG: Color = rgb(0x50, 0x22, 0x05);
/// Accent for the selected row: the light bevel colour of the item art.
pub const SEL_BAR: Color = 0xFFE4_9C6F;

pub const SCALE: i32 = 1; // text: em x scale
pub const ART: i32 = 3; // sprites: 16 px art x 3
pub const PAD: i32 = 8;

pub const NAME_H: i32 = font::LINE_HEIGHT * SCALE;
pub const SEEK_H: i32 = 8 * SCALE;
pub const GAP: i32 = 8;
pub const ICON: i32 = crate::buttons::CONTROLS_H * ART;
pub const BAR_H: i32 = PAD + SEEK_H + GAP + ICON + PAD;

pub const LIST_PAD_X: i32 = 24;
pub const LIST_PAD_Y: i32 = 12;
pub const LIST_GAP: i32 = 0;
