// Regenerate crates/pxui/src/font_atlas.rs from assets/fonts/Cairopixel.ttf.
//
// cargo run -p fluyer-pxui --release --example gen_font -- <size>

#[path = "../gen/font_atlas.rs"]
mod gen;

fn main() {
    let size: f32 = std::env::args()
        .nth(1)
        .map(|s| s.parse().unwrap())
        .unwrap_or(48.0);
    gen::run(size);
}
