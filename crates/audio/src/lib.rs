// fluyer-audio: one flat playback surface over the per-OS backend. No UI, no app
// model, no winit. Seconds in, seconds out.

#[cfg(target_os = "macos")]
mod backend {
    include!("mac.rs");
}

#[cfg(not(target_os = "macos"))]
mod backend {
    include!("win.rs");
}

pub use backend::*;
