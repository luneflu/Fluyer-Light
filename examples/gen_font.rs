// Bake assets/fonts/Cairopixel.ttf into src/font_atlas.rs as a 1bpp glyph atlas.
// Rust-only: ttf-parser outlines + exact point-in-polygon test at pixel centres.
// Pixel fonts survive this untouched; anything antialiased is refused.
//
// cargo run --release --offline --example gen_font [size]

use std::fmt::Write as _;
use std::fs;

use ttf_parser::{Face, GlyphId, OutlineBuilder};

const TTF: &str = "assets/fonts/Cairopixel.ttf";
const OUT: &str = "src/font_atlas.rs";
// Wood palette lifted from assets/buttons/controls.png.
const WOOD_CORE: u32 = 0xFF50_2205;   // v1 body
const WOOD_WALL: u32 = 0xFF50_2205;   // top/left edge inside the mask
const WOOD_FLOOR: u32 = 0xFF6B_3411;  // recessed groove floor inside the mask
const WOOD_LIP: u32 = 0xFFC1_7748;    // 1 px catchlight on the sunlit bottom/right lip
const WOOD_HI: u32 = 0xFFC1_7748;     // lit lip
const WOOD_MID: u32 = 0xFF6C_320F;    // groove floor between wall and lip

const FIRST: u8 = 32;
const LAST: u8 = 127;

struct Path {
    polys: Vec<Vec<(f32, f32)>>,
    cur: Vec<(f32, f32)>,
    start: (f32, f32),
    last: (f32, f32),
    scale: f32,     // font units -> px
    ascent_px: f32, // font y is up, image y is down
}

impl Path {
    fn new(scale: f32, ascent_px: f32) -> Self {
        Self {
            polys: Vec::new(),
            cur: Vec::new(),
            start: (0.0, 0.0),
            last: (0.0, 0.0),
            scale,
            ascent_px,
        }
    }

    /// Transform into image space here, so outlines and pixel tests share one space.
    fn xf(&self, p: (f32, f32)) -> (f32, f32) {
        (p.0 * self.scale, self.ascent_px - p.1 * self.scale)
    }

    fn push(&mut self, p: (f32, f32)) {
        let q = self.xf(p);
        self.cur.push(q);
        self.last = q;
    }

    fn flatten_quad(&mut self, c: (f32, f32), end: (f32, f32)) {
        let p0 = self.last;
        for i in 1..=8 {
            let t = i as f32 / 8.0;
            let u = 1.0 - t;
            self.push((
                u * u * p0.0 + 2.0 * u * t * c.0 + t * t * end.0,
                u * u * p0.1 + 2.0 * u * t * c.1 + t * t * end.1,
            ));
        }
    }

    fn flatten_cubic(&mut self, c1: (f32, f32), c2: (f32, f32), end: (f32, f32)) {
        let p0 = self.last;
        for i in 1..=12 {
            let t = i as f32 / 12.0;
            let u = 1.0 - t;
            self.push((
                u * u * u * p0.0 + 3.0 * u * u * t * c1.0 + 3.0 * u * t * t * c2.0 + t * t * t * end.0,
                u * u * u * p0.1 + 3.0 * u * u * t * c1.1 + 3.0 * u * t * t * c2.1 + t * t * t * end.1,
            ));
        }
    }
}

impl OutlineBuilder for Path {
    fn move_to(&mut self, x: f32, y: f32) {
        let q = self.xf((x, y));
        self.cur.clear();
        self.cur.push(q);
        self.start = q;
        self.last = q;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.push((x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.flatten_quad((x1, y1), (x, y));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.flatten_cubic((x1, y1), (x2, y2), (x, y));
    }

    fn close(&mut self) {
        if self.cur.len() > 2 {
            self.polys.push(std::mem::take(&mut self.cur));
        }
        self.cur.clear();
        self.last = self.start;
    }
}

/// Even-odd crossing test.
fn inside(polys: &[Vec<(f32, f32)>], x: f32, y: f32) -> bool {
    let mut cross = false;
    for poly in polys {
        let n = poly.len();
        for i in 0..n {
            let (x0, y0) = poly[i];
            let (x1, y1) = poly[(i + 1) % n];
            if (y0 > y) != (y1 > y) {
                let t = (y - y0) / (y1 - y0);
                if x < x0 + t * (x1 - x0) {
                    cross = !cross;
                }
            }
        }
    }
    cross
}

fn main() {
    let mut args = std::env::args().skip(1);
    let size: f32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(32.0);
    let variant: u8 = args
        .next()
        .map(|s| s.trim_start_matches('v').parse().unwrap())
        .unwrap_or(2);

    let data = fs::read(TTF).expect("read ttf");
    let face = Face::parse(&data, 0).expect("parse ttf");
    let scale = size / face.units_per_em() as f32;

    let ascent = (face.ascender() as f32 * scale).round() as i32;
    let descent = (-face.descender() as f32 * scale).round() as i32;

    let mut bitmap: Vec<u8> = Vec::new();
    let mut glyphs: Vec<(i32, i32, i32, i32, i32, usize)> = Vec::new();
    let mut shaded: Vec<u32> = Vec::new();
    let mut sprites: Vec<(i32, i32, usize, i32, i32)> = Vec::new(); // (w, h, off, pad_l, pad_t)
    let mut partial = 0;

    for code in FIRST..LAST {
        let ch = code as char;
        let gid = face.glyph_index(ch).unwrap_or(GlyphId(0));
        let adv = (face.glyph_hor_advance(gid).unwrap_or(0) as f32 * scale).round() as i32;

        let mut path = Path::new(scale, ascent as f32);
        if face.outline_glyph(gid, &mut path).is_none() {
            glyphs.push((adv, 0, 0, 0, 0, bitmap.len()));
            sprites.push((0, 0, shaded.len(), 0, 0));
            continue;
        }

        // outlines are already in image space
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for poly in &path.polys {
            for &(x, y) in poly {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
        if !min_x.is_finite() {
            glyphs.push((adv, 0, 0, 0, 0, bitmap.len()));
            sprites.push((0, 0, shaded.len(), 0, 0));
            continue;
        }

        let x0 = min_x.floor() as i32;
        let y0 = min_y.floor() as i32;
        let w = (max_x.ceil() as i32 - x0).max(0);
        let h = (max_y.ceil() as i32 - y0).max(0);

        let off = bitmap.len();
        for row in 0..h {
            for col in 0..w {
                let px = (x0 + col) as f32 + 0.5;
                let py = (y0 + row) as f32 + 0.5;
                if inside(&path.polys, px, py) {
                    bitmap.push(1);
                } else {
                    bitmap.push(0);
                }
            }
        }
        // pixel fonts are strictly binary; a partially covered pixel means we are off-grid
        for row in 0..h {
            for col in 0..w {
                let px = (x0 + col) as f32 + 0.5;
                let py = (y0 + row) as f32 + 0.5;
                let bit = bitmap[off + (row * w + col) as usize];
                if bit == 0 && inside(&path.polys, px, py) {
                    partial += 1;
                }
            }
        }
        glyphs.push((adv, x0, y0, w, h, off));

        // Debossed stamp, matching the transport icons: the glyph stays solid dark and
        // only the empty pixel to its right and below catches the light. At em 32 a
        // stroke is 2 px and counters are >= 2 px, so the 1 px lip never bridges them.
        let (pad_l, pad_t) = if variant >= 4 && variant <= 6 { (1, 1) } else { (0, 0) };
        let (sw, sh) = (w + 1 + pad_l, h + 1 + pad_t);
        sprites.push((sw, sh, shaded.len(), pad_l, pad_t));
        let ink = |xx: i32, yy: i32| -> bool {
            let (mx, my) = (xx - pad_l, yy - pad_t);
            mx >= 0 && my >= 0 && mx < w && my < h && bitmap[off + (my * w + mx) as usize] != 0
        };
        // The lip may only sit on the glyph's outer silhouette: flood the empty cells
        // reachable from the sprite border, so counters stay wood face and stay open.
        let ext_w = (sw + 2) as usize;
        let mut outside = vec![false; ext_w * (sh + 2) as usize];
        let mut stack: Vec<(i32, i32)> = Vec::new();
        for x in -1..=sw {
            stack.push((x, -1));
            stack.push((x, sh));
        }
        for y in -1..=sh {
            stack.push((-1, y));
            stack.push((sw, y));
        }
        while let Some((x, y)) = stack.pop() {
            let (bx, by) = (x + 1, y + 1);
            let i = by as usize * ext_w + bx as usize;
            if x < -1 || y < -1 || x > sw || y > sh || outside[i] {
                continue;
            }
            if ink(x, y) {
                continue;
            }
            outside[i] = true;
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
        }
        let is_outside = |x: i32, y: i32| outside[(y + 1) as usize * ext_w + (x + 1) as usize];
        for row in 0..sh {
            for col in 0..sw {
                // v1: flat body + 2-sided lip. v2/v3: shadow wall on the top/left edge of
                // the mask, groove floor behind it; v3 adds a top-right stem glint.
                let px = if variant >= 7 {
                    // Three-role carve, all inside the mask: wall where plank is above or
                    // left, lip where plank is below or right, floor between them.
                    if !ink(col, row) {
                        0
                    } else if !ink(col, row - 1) || !ink(col - 1, row) {
                        WOOD_WALL
                    } else if !ink(col, row + 1) || !ink(col + 1, row) {
                        WOOD_HI
                    } else {
                        WOOD_MID
                    }
                } else if variant >= 6 {
                    // Flat dark body + sunlit lip on the bottom/right only.
                    if ink(col, row) {
                        WOOD_WALL
                    } else if is_outside(col, row) && (ink(col - 1, row) || ink(col, row - 1)) {
                        WOOD_HI
                    } else {
                        0
                    }
                } else if variant >= 5 {
                    // Light lettering with a dark 1 px outline on the silhouette.
                    if ink(col, row) {
                        WOOD_HI
                    } else if is_outside(col, row)
                        && (ink(col + 1, row)
                            || ink(col - 1, row)
                            || ink(col, row + 1)
                            || ink(col, row - 1))
                    {
                        WOOD_WALL
                    } else {
                        0
                    }
                } else if variant >= 4 {
                    // Balanced emboss: body flat, 1 px dark rim outside the top/left of
                    // the silhouette, 1 px light rim outside the bottom/right. Rims only
                    // on the outside so counters stay open.
                    if ink(col, row) {
                        WOOD_FLOOR
                    } else if is_outside(col, row) && (ink(col + 1, row) || ink(col, row + 1)) {
                        WOOD_WALL
                    } else if is_outside(col, row) && (ink(col - 1, row) || ink(col, row - 1)) {
                        WOOD_HI
                    } else {
                        0
                    }
                } else {
                    let wall =
                        variant >= 2 && ink(col, row) && (!ink(col - 1, row) || !ink(col, row - 1));
                    let lip = is_outside(col, row)
                        && match variant {
                            1 => ink(col - 1, row) || ink(col, row - 1),
                            2 => ink(col, row - 1),
                            _ => ink(col, row - 1) || (ink(col - 1, row) && !ink(col - 1, row - 1)),
                        };
                    if ink(col, row) {
                        if wall {
                            WOOD_WALL
                        } else if variant >= 2 {
                            WOOD_FLOOR
                        } else {
                            WOOD_CORE
                        }
                    } else if lip {
                        WOOD_LIP
                    } else {
                        0
                    }
                };
                shaded.push(px);
            }
        }
    }

    if partial > 0 {
        eprintln!("warning: {partial} partial pixels at size {size} — pick a grid size");
    }

    let mut s = String::new();
    s.push_str("// Generated by examples/gen_font.rs from assets/fonts/Cairopixel.ttf — do not edit.\n");
    let _ = writeln!(s, "#[allow(dead_code)]\npub const SIZE: i32 = {size};");
    let _ = writeln!(s, "pub const ASCENT: i32 = {ascent};");
    let _ = writeln!(s, "pub const DESCENT: i32 = {descent};");
    let _ = writeln!(s, "pub const FIRST: u8 = {FIRST};");
    s.push_str("\n#[derive(Clone, Copy)]\npub struct Glyph {\n");
    s.push_str("    pub adv: i8,   // pen advance\n    pub x0: i8,    // left bearing from pen\n");
    s.push_str("    pub y0: i8,    // top from ascender line\n    pub w: u8,\n    pub h: u8,\n");
    s.push_str("    pub off: u32,  // index into BITMAP\n");
    s.push_str("    pub sw: u8,    // beveled sprite size (w+2, h+2), includes the ring\n");
    s.push_str("    pub sh: u8,\n    pub soff: u32, // index into SHADED\n");
    s.push_str("    pub pad_l: i8, // sprite starts this far left of x0\n    pub pad_t: i8,\n}\n\n");
    let _ = writeln!(s, "pub const GLYPHS: [Glyph; {}] = [", glyphs.len());
    for (i, (adv, x0, y0, w, h, off)) in glyphs.iter().enumerate() {
        let (sw, sh, soff, pad_l, pad_t) = sprites[i];
        let _ = writeln!(
            s,
            "    Glyph {{ adv: {adv}, x0: {x0}, y0: {y0}, w: {w}, h: {h}, off: {off}, \
             sw: {sw}, sh: {sh}, soff: {soff}, pad_l: {pad_l}, pad_t: {pad_t} }},"
        );
    }
    s.push_str("];\n\n");
    let _ = writeln!(s, "pub const BITMAP: [u8; {}] = [", bitmap.len());
    for chunk in bitmap.chunks(32) {
        s.push_str("    ");
        for (i, b) in chunk.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let _ = write!(s, "{b}");
        }
        s.push_str(",\n");
    }
    s.push_str("];\n\n");
    let _ = writeln!(s, "pub const SHADED: [u32; {}] = [", shaded.len());
    for chunk in shaded.chunks(12) {
        s.push_str("    ");
        for (i, px) in chunk.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let _ = write!(s, "0x{px:08X}");
        }
        s.push_str(",\n");
    }
    s.push_str("];\n");

    fs::write(OUT, s).expect("write atlas");
    println!(
        "size={size} ascent={ascent} descent={descent} glyphs={} bytes={} -> {OUT}",
        glyphs.len(),
        bitmap.len()
    );
}
