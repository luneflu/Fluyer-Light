// theme: palette + metrics. The only place raw colours and sizes live.
//
// Naming: colours are spelled out (CANVAS_BACKGROUND, not BG), lengths carry
// their unit word (SEEK_HEIGHT, BAR_PADDING), `*_SCALE` multiplies something baked.

use crate::buttons;
use crate::gfx::{grey, Color};

/// Canvas clear colour, painted behind every widget.
pub const CANVAS_BACKGROUND: Color = grey(0x18);
/// Track name text and the seek progress fill.
pub const FOREGROUND: Color = grey(0xF0);
/// Control bar strip.
pub const BAR_BACKGROUND: Color = grey(0x1C);
/// Seek trough behind the progress fill.
pub const SEEK_TRACK_BACKGROUND: Color = grey(0x30);
/// Accent for the selected row: the light bevel colour of the item art.
pub const SELECTION_ACCENT: Color = 0xFFE4_9C6F;

/// Font atlas px -> device px. Text em = `font_atlas::SIZE * TEXT_SCALE`.
pub const TEXT_SCALE: i32 = 1;
/// Sprite art px -> device px (controls 16 px, music item 24 px).
pub const SPRITE_SCALE: i32 = 3;

pub const BAR_PADDING: i32 = 8;
pub const BAR_GAP: i32 = 8;
pub const SEEK_HEIGHT: i32 = 8 * TEXT_SCALE;
/// Transport button box side: one controls sprite at `SPRITE_SCALE`.
pub const TRANSPORT_SIZE: i32 = buttons::CONTROLS_H * SPRITE_SCALE;
pub const BAR_HEIGHT: i32 = BAR_PADDING + SEEK_HEIGHT + BAR_GAP + TRANSPORT_SIZE + BAR_PADDING;

pub const LIST_PADDING_X: i32 = 24;
pub const LIST_PADDING_Y: i32 = 12;
pub const LIST_ROW_GAP: i32 = 24;
