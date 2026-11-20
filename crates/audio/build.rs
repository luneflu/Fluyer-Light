// Audio only. CoreAudio/AudioToolbox decode + output on macOS, Media Foundation on Windows.
use std::process::Command;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
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

    println!("cargo:rustc-link-search=native={out}");
    println!("cargo:rustc-link-lib=static=audio");
    for f in ["CoreAudio", "AudioToolbox", "AudioUnit", "CoreFoundation"] {
        println!("cargo:rustc-link-lib=framework={f}");
    }
    println!("cargo:rerun-if-changed=audio.c");
}
