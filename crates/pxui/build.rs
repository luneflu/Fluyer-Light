// macOS pixel presenter only; audio builds in the `fluyer-audio` crate.
use std::process::Command;

fn main() {
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
