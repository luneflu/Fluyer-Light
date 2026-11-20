// Bakers (bootstrap the gitignored src/*.rs when missing) + the macOS pixel
// presenter. audio builds in the `fluyer-audio` crate.

#[path = "gen/buttons.rs"]
mod gen_buttons;
#[path = "gen/font_atlas.rs"]
mod gen_font_atlas;

use std::path::Path;
use std::process::Command;

// Baked default; re-bake with `--example gen_font -- <size>`.
const ATLAS_SIZE: f32 = 48.0;

fn bake_if_missing() {
    if !Path::new("src/buttons.rs").exists() {
        gen_buttons::run();
    }
    if !Path::new("src/font_atlas.rs").exists() {
        gen_font_atlas::run(ATLAS_SIZE);
    }
    println!("cargo:rerun-if-changed=../../assets/buttons");
    println!("cargo:rerun-if-changed=../../assets/fonts");
}

fn main() {
    bake_if_missing();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        return; // Windows uses softbuffer; no C presenter.
    }

    let out = std::env::var("OUT_DIR").unwrap();

    // macOS pixel presenter: IOSurface layer instead of softbuffer's two ARGB buffers.
    let pobj = format!("{out}/present.o");
    let plib = format!("{out}/libpresent.a");
    let ok = Command::new("clang")
        .args(["-O2", "-fobjc-arc", "-c", "present.m", "-o", &pobj])
        .status()
        .expect("clang")
        .success();
    assert!(ok, "present.m failed to compile");
    assert!(
        Command::new("ar").args(["crs", &plib, &pobj]).status().expect("ar").success(),
        "ar failed"
    );

    println!("cargo:rustc-link-search=native={out}");
    println!("cargo:rustc-link-lib=static=present");
    println!("cargo:rustc-link-lib=objc");
    for f in ["Cocoa", "QuartzCore", "IOSurface"] {
        println!("cargo:rustc-link-lib=framework={f}");
    }
    println!("cargo:rerun-if-changed=present.m");
}
