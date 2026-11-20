# UI: canvas, theme, widgets

## Canvas (`pxui/src/gfx.rs`)
```rust
RGB 0x00RRGGBB, Color = u32; rgb(r,g,b), grey(v)
Rect::new(x,y,w,h) / contains(x,y) / union(o)
Canvas::new(&mut [u32], w, h) / with_stride(px, w, h, stride)   // unsafe, padded rows
c.w(), c.h(), c.clear(color), c.fill(rect, color)
c.blit(src, sw, sh, (x,y))                       // opaque, no scale
c.blit_1bpp(src, sw, sh, (x,y), fg)              // 1bpp mask + flat colour
c.blit_masked(src, sw, sh, (x,y), scale)         // 0xAARRGGBB, 0 = skip, integer scale
c.blit_sliced(src, sw, sh, rect, scale)          // 3 horizontal tiles
c.blit_9slice(src, tw, th, rect, scale, cap_t, cap_b)
c.mark(rect), c.mark_all(), c.dirty(), c.take_dirty()
```
`blit_9slice`: 3 tiles = left/mid/right; top `cap_t` and bottom `cap_b` art rows copy 1:1, every middle dest row maps to the single source row `cap_t` (repeat one flat row, never stretch — stretching smears, repeating the band seams).

## Theme (`pxui/src/theme.rs`) — only place with colours/sizes
```
BG grey(0x18)  FG grey(0xF0)  BAR_BG grey(0x1C)  TRACK_BG grey(0x30)
ITEM_FG rgb(0x50,0x22,0x05)   SEL_BAR 0xFFE49C6F
SCALE 1 (text)  ART 3 (16 px sprites)  PAD 8  GAP 8
SEEK_H 8*SCALE  ICON buttons::CONTROLS_H*ART  BAR_H PAD+SEEK_H+GAP+ICON+PAD
LIST_PAD_X 24  LIST_PAD_Y 12  LIST_GAP 0
```

## Widgets (`pxui/src/widgets/`, one file each, no widget imports another)
- `button.rs Button { rect, label, scale, fg, fixed_w, fixed_h }` — `w/h` derive from font when fixed = 0; `draw` = 9-slice frame + centred ink.
- `list.rs ListView` — config only: `area, pad_x, pad_y, gap, scale, art_scale`. `row_h = LINE_HEIGHT*scale + 2*pad_y`, `stride = row_h + gap`, `visible()`, `scroll_for(selected)`, `row(i, scroll)`, `hit(x,y,len,scroll)`, `draw(c, &[&str], selected)`.
- `art.rs` — `CAP_T/CAP_B = 8`, `draw_row_art(c, rect, scale)` = 9-slice of MUSIC_ITEM.
- `seek.rs SeekBar` — `progress_px(pos,len)`, `frac_at(x)`, `contains`, `draw(c,pos,len)`.
- `icons.rs` — frames `F_PLAY 0, F_PAUSE 2, F_NEXT 4, F_PREV 6` (+1 = pressed), `draw(c, rect, frame)`.

## Composition (app side, `src/ui/`)
- `layout.rs` builds every rect from `(w, h, inset)`; `slot(0..2)` = prev/play/next, `name_room()` = room left of the buttons.
- `view.rs` is a pure function: clear → list → bar strip → track name (ellipsised `..` via `fit`) → icons → seek → `mark_all()`. `render_progress` repaints only the seek strip.

## Traps
- `Rect::contains` is inclusive-exclusive; hit-testing and drawing must use the same rect source or clicks drift.
- List hit-testing must apply `pad_x` via `inner_x()`; ignoring it makes rows clickable in the margin.
- One art px = `ART` device px, one font px = `SCALE` device px; mismatch shows as text thicker than the sprites.
