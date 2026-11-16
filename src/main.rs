// main: FFI to the Cocoa shell, event pump bridge, lifetime wiring.

mod app;
mod font;
mod font_atlas;
mod gfx;
mod input;

use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;

use app::{App, Outcome};
use gfx::Canvas;
use input::{Event, Key, Mods};

#[repr(C)]
struct FlWindow {
    _private: [u8; 0],
}

const W: i32 = 400;
const H: i32 = 225;

extern "C" {
    fn fl_open(w: c_int, h: c_int, title: *const c_char) -> *mut FlWindow;
    fn fl_close(f: *mut FlWindow);
    fn fl_buffer(f: *mut FlWindow) -> *mut c_void;
    fn fl_flush(f: *mut FlWindow);
    fn fl_on_key(f: *mut FlWindow, cb: extern "C" fn(c_int, u32, *mut c_void), ud: *mut c_void);
    fn fl_on_mouse(f: *mut FlWindow, cb: extern "C" fn(c_int, c_int, *mut c_void), ud: *mut c_void);
    fn fl_run(f: *mut FlWindow);
    fn execv(path: *const c_char, argv: *const *const c_char) -> c_int;
}

/// One place where a platform event becomes an app action.
fn dispatch(app: &mut App, win: *mut FlWindow, ev: Event) {
    match app.handle(ev) {
        Outcome::Quit => std::process::exit(0),
        Outcome::Redraw => {
            app.draw();
            if app.canvas.take_dirty().w > 0 {
                unsafe { fl_flush(win) };
            }
        }
        Outcome::Idle => {}
    }
}

extern "C" fn on_key(keycode: c_int, mod_bits: u32, ud: *mut c_void) {
    // SAFETY: ud is the Box<App> leaked in main; valid for the whole runloop.
    let ar = unsafe { &mut *(ud as *mut AppRef) };
    dispatch(
        &mut ar.0,
        ar.1,
        Event::Key {
            key: Key::from_keycode(keycode),
            mods: Mods::from_bits(mod_bits),
        },
    );
}

extern "C" fn on_mouse(x: c_int, y: c_int, ud: *mut c_void) {
    // SAFETY: same contract as on_key.
    let ar = unsafe { &mut *(ud as *mut AppRef) };
    dispatch(&mut ar.0, ar.1, Event::MouseDown { x, y });
}

struct AppRef(App, *mut FlWindow);

fn reexec_nano_off() {
    // nanov2 keeps partially used slabs (~1 MB). Only settable before malloc init -> re-exec.
    if std::env::var("MallocNanoZone").as_deref() == Ok("0") {
        return;
    }
    let exe = std::env::current_exe().expect("exe path");
    let Ok(cpath) = CString::new(exe.to_str().expect("utf8 path")) else {
        return;
    };
    let argv = [cpath.as_ptr(), ptr::null()];
    unsafe {
        std::env::set_var("MallocNanoZone", "0");
        execv(cpath.as_ptr(), argv.as_ptr());
    }
}

fn main() {
    reexec_nano_off();

    let title = CString::new("fluyer-low").unwrap();
    let win = unsafe { fl_open(W, H, title.as_ptr()) };
    assert!(!win.is_null(), "fl_open failed");

    // SAFETY: surface is W*H BGRA pixels, alive until fl_close.
    let canvas = unsafe { Canvas::new(fl_buffer(win) as *mut u32, W, H) };
    let mut app = Box::new(AppRef(App::new(canvas), win));

    app.0.draw();
    let ud: *mut c_void = &mut *app as *mut AppRef as *mut c_void;

    unsafe {
        fl_on_key(win, on_key, ud);
        fl_on_mouse(win, on_mouse, ud);
        fl_flush(win);
        fl_run(win);
        fl_close(win);
    }
    drop(app);
}
