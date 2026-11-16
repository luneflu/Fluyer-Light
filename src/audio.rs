// audio: miniaudio engine behind audio.c. Non-blocking; playback runs on its own thread.

use std::ffi::{c_char, c_double, c_float, c_int, CString};

extern "C" {
    fn au_init() -> c_int;
    fn au_play(path: *const c_char) -> c_int;
    fn au_stop();
    fn au_toggle() -> c_int;
    fn au_playing() -> c_int;
    fn au_pos() -> c_double;
    fn au_len() -> c_double;
    fn au_set_volume(v: c_float);
    fn au_quit();
}

pub fn init() -> bool {
    unsafe { au_init() != 0 }
}

pub fn play(path: &str) -> bool {
    let Ok(c) = CString::new(path) else {
        return false;
    };
    unsafe { au_play(c.as_ptr()) != 0 }
}

pub fn toggle() {
    unsafe { au_toggle() };
}

pub fn is_playing() -> bool {
    unsafe { au_playing() != 0 }
}

/// (position, length) in seconds.
pub fn progress() -> (f64, f64) {
    unsafe { (au_pos(), au_len()) }
}

pub fn quit() {
    unsafe { au_quit() };
}
