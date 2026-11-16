// Bake fonts/Cairopixel.ttf into src/font_atlas.rs as a 1bpp glyph atlas.
// Rust-only: ttf-parser outlines + exact point-in-polygon test at pixel centres.
// Pixel fonts survive this untouched; anything antialiased is refused.
//
// cargo run --release --offline --example gen_font [size]

use std::fmt::Write as _;
use std::fs;

use ttf_parser::{Face, GlyphId, OutlineBuilder};

const TTF: &str = "fonts/Cairopixel.ttf";
const OUT: &str = "src/font_atlas.rs";
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
    let size: f32 = args.next().map(|s| s.parse().unwrap()).unwrap_or(16.0);

    let data = fs::read(TTF).expect("read ttf");
    let face = Face::parse(&data, 0).expect("parse ttf");
    let scale = size / face.units_per_em() as f32;

    let ascent = (face.ascender() as f32 * scale).round() as i32;
    let descent = (-face.descender() as f32 * scale).round() as i32;

    let mut bitmap: Vec<u8> = Vec::new();
    let mut glyphs: Vec<(i32, i32, i32, i32, i32, usize)> = Vec::new();
    let mut partial = 0;

    for code in FIRST..LAST {
        let ch = code as char;
        let gid = face.glyph_index(ch).unwrap_or(GlyphId(0));
        let adv = (face.glyph_hor_advance(gid).unwrap_or(0) as f32 * scale).round() as i32;

        let mut path = Path::new(scale, ascent as f32);
        if face.outline_glyph(gid, &mut path).is_none() {
            glyphs.push((adv, 0, 0, 0, 0, bitmap.len()));
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
    }

    if partial > 0 {
        eprintln!("warning: {partial} partial pixels at size {size} — pick a grid size");
    }

    let mut s = String::new();
    s.push_str("// Generated by examples/gen_font.rs from fonts/Cairopixel.ttf — do not edit.\n");
    let _ = writeln!(s, "#[allow(dead_code)]\npub const SIZE: i32 = {size};");
    let _ = writeln!(s, "pub const ASCENT: i32 = {ascent};");
    let _ = writeln!(s, "pub const DESCENT: i32 = {descent};");
    let _ = writeln!(s, "pub const FIRST: u8 = {FIRST};");
    s.push_str("\n#[derive(Clone, Copy)]\npub struct Glyph {\n");
    s.push_str("    pub adv: i8,   // pen advance\n    pub x0: i8,    // left bearing from pen\n");
    s.push_str("    pub y0: i8,    // top from ascender line\n    pub w: u8,\n    pub h: u8,\n");
    s.push_str("    pub off: u32,  // index into BITMAP\n}\n\n");
    let _ = writeln!(s, "pub const GLYPHS: [Glyph; {}] = [", glyphs.len());
    for (adv, x0, y0, w, h, off) in &glyphs {
        let _ = writeln!(
            s,
            "    Glyph {{ adv: {adv}, x0: {x0}, y0: {y0}, w: {w}, h: {h}, off: {off} }},"
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
    s.push_str("];\n");


    fs::write(OUT, s).expect("write atlas");
    println!(
        "size={size} ascent={ascent} descent={descent} glyphs={} bytes={} -> {OUT}",
        glyphs.len(),
        bitmap.len()
    );
}
