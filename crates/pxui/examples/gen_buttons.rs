// Regenerate crates/pxui/src/buttons.rs from assets/buttons/*.png.
//
// cargo run -p fluyer-pxui --release --example gen_buttons

#[path = "../gen/buttons.rs"]
mod gen;

fn main() {
    gen::run();
}
