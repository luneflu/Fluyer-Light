// Audio only: the window shell is winit + softbuffer now.
use std::process::Command;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // MSVC toolchain: cc finds the VS headers/libs, cl compiles, lib archives.
        cc::Build::new()
            .flag("/O2")
            .file("audio_win.c")
            .compile("flaudio");
        for lib in ["mfplat", "mfreadwrite", "mfuuid", "ole32", "uuid", "ksuser"] {
            println!("cargo:rustc-link-lib={lib}");
        }
        println!("cargo:rerun-if-changed=audio_win.c");
        return;
    }

    let out = std::env::var("OUT_DIR").unwrap();
    let obj = format!("{out}/audio.o");
    let lib = format!("{out}/libaudio.a");
    let ok = Command::new("clang")
        .args(["-O2", "-pthread", "-c", "audio.c", "-o", &obj])
        .status()
        .expect("clang")
        .success();
    assert!(ok, "audio.c failed to compile");
    assert!(
        Command::new("ar").args(["crs", &lib, &obj]).status().expect("ar").success(),
        "ar failed"
    );

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
    println!("cargo:rustc-link-lib=static=audio");
    println!("cargo:rustc-link-lib=static=present");
    println!("cargo:rustc-link-lib=objc");
    for f in ["Cocoa", "QuartzCore", "IOSurface"] {
        println!("cargo:rustc-link-lib=framework={f}");
    }
    for f in ["CoreAudio", "AudioToolbox", "AudioUnit", "CoreFoundation"] {
        println!("cargo:rustc-link-lib=framework={f}");
    }
    println!("cargo:rerun-if-changed=audio.c");
    println!("cargo:rerun-if-changed=present.m");
}
