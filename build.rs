use std::process::Command;

fn main() {
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    if target == "windows" {
        // MSVC toolchain: cc finds the VS headers/libs, cl compiles, lib archives.
        cc::Build::new()
            .flag("/O2")
            .file("window_win32.c")
            .file("audio_win.c")
            .compile("flplat");
        for lib in ["mfplat", "mfreadwrite", "mfuuid", "mmdevapi", "ole32", "uuid", "user32", "gdi32", "shlwapi"] {
            println!("cargo:rustc-link-lib={lib}");
        }
        for f in ["window_win32.c", "audio_win.c", "window.h"] {
            println!("cargo:rerun-if-changed={f}");
        }
        return;
    }

    let out = std::env::var("OUT_DIR").unwrap();

    let build = |src: &str, obj: &str, extra: &[&str], lib: &str| {
        let obj = format!("{out}/{obj}");
        let lib = format!("{out}/{lib}");
        let mut args: Vec<&str> = vec!["-O2"];
        args.extend(extra);
        args.extend(["-c", src, "-o", &obj]);
        assert!(Command::new("clang").args(&args).status().expect("clang").success(),
                "{src} failed to compile");
        assert!(Command::new("ar").args(["crs", &lib, &obj]).status().expect("ar").success(),
                "ar failed");
    };

    build("window.m", "window.o", &["-fobjc-arc"], "libwindow.a");
    build("audio.c", "audio.o", &["-pthread"], "libaudio.a");

    println!("cargo:rustc-link-search=native={out}");
    println!("cargo:rustc-link-lib=static=window");
    println!("cargo:rustc-link-lib=static=audio");
    for f in ["Cocoa", "QuartzCore", "IOSurface"] {
        println!("cargo:rustc-link-lib=framework={f}");
    }
    for f in ["CoreAudio", "AudioToolbox", "AudioUnit", "CoreFoundation"] {
        println!("cargo:rustc-link-lib=framework={f}");
    }
    println!("cargo:rerun-if-changed=window.m");
    println!("cargo:rerun-if-changed=window.h");
    println!("cargo:rerun-if-changed=audio.c");
}
