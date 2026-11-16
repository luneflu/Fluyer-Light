use std::process::Command;

fn main() {
    let out = std::env::var("OUT_DIR").unwrap();
    let obj = format!("{out}/window.o");
    let lib = format!("{out}/libwindow.a");

    let ok = Command::new("clang")
        .args(["-O2", "-fobjc-arc", "-c", "window.m", "-o", &obj])
        .status()
        .expect("clang")
        .success();
    assert!(ok, "window.m failed to compile");

    let ok = Command::new("ar")
        .args(["crs", &lib, &obj])
        .status()
        .expect("ar")
        .success();
    assert!(ok, "ar failed");

    println!("cargo:rustc-link-search=native={out}");
    println!("cargo:rustc-link-lib=static=window");
    for f in ["Cocoa", "QuartzCore", "IOSurface"] {
        println!("cargo:rustc-link-lib=framework={f}");
    }
    println!("cargo:rerun-if-changed=window.m");
    println!("cargo:rerun-if-changed=window.h");
}
