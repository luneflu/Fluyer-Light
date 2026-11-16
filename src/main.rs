// main: FFI to the Cocoa shell, event pump bridge, lifetime wiring.

mod app;
mod font;
mod font_atlas;
mod gfx;
mod input;

use std::ffi::{c_char, c_int, c_void, CString};
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use app::{App, Outcome};
use gfx::Canvas;
use input::{Event, Key, Mods};

#[repr(C)]
struct FlWindow {
    _private: [u8; 0],
}

// Design space = window points. Retina handled by AppKit.
const W: i32 = 800;
const H: i32 = 450;

extern "C" {
    fn fl_open(w: c_int, h: c_int, title: *const c_char) -> *mut FlWindow;
    fn fl_close(f: *mut FlWindow);
    fn fl_buffer(f: *mut FlWindow) -> *mut c_void;
    fn fl_flush(f: *mut FlWindow);
    fn fl_resize_ensure(f: *mut FlWindow, w: c_int, h: c_int, stride: *mut c_int) -> *mut c_void;
    fn fl_log(msg: *const c_char);
    fn fl_on_key(f: *mut FlWindow, cb: extern "C" fn(c_int, u32, *mut c_void), ud: *mut c_void);
    fn fl_on_mouse(f: *mut FlWindow, cb: extern "C" fn(c_int, c_int, *mut c_void), ud: *mut c_void);
    fn fl_on_resize(f: *mut FlWindow, cb: extern "C" fn(c_int, c_int, *mut c_void), ud: *mut c_void);
    fn fl_run(f: *mut FlWindow);
    fn execv(path: *const c_char, argv: *const *const c_char) -> c_int;
}

struct AppRef(App, *mut FlWindow);

static app_ref: AtomicPtr<AppRef> = AtomicPtr::new(ptr::null_mut());

fn app() -> &'static mut AppRef {
    // SAFETY: single-threaded main-thread runloop; the Box lives for the process.
    unsafe { &mut *app_ref.load(Ordering::Relaxed) }
}

fn present() {
    let ar = app();
    ar.0.draw();
    if ar.0.canvas.take_dirty().w > 0 {
        unsafe { fl_flush(ar.1) };
    }
}

/// One place where a platform event becomes an app action.
fn dispatch(ev: Event) {
    match app().0.handle(ev) {
        Outcome::Quit => std::process::exit(0),
        Outcome::Redraw => present(),
        Outcome::Idle => {}
    }
}

extern "C" fn on_key(keycode: c_int, mod_bits: u32, _ud: *mut c_void) {
    dispatch(Event::Key {
        key: Key::from_keycode(keycode),
        mods: Mods::from_bits(mod_bits),
    });
}

extern "C" fn on_mouse(x: c_int, y: c_int, _ud: *mut c_void) {
    dispatch(Event::MouseDown { x, y });
}

fn log(msg: &str) {
    if let Ok(c) = CString::new(msg) {
        unsafe { fl_log(c.as_ptr()) };
    }
}

extern "C" fn on_resize(w: c_int, h: c_int, _ud: *mut c_void) {
    let ar = app();
    let mut stride: c_int = 0;
    let px = unsafe { fl_resize_ensure(ar.1, w, h, &mut stride) };
    if px.is_null() || stride < w {
        log(&format!("rust resize rejected {w}x{h}"));
        return;
    }
    // SAFETY: surface holds at least w*h BGRA pixels at `stride` per row.
    ar.0.resize(unsafe { Canvas::new(px as *mut u32, w, h, stride) });
    ar.0.draw();
    log(&format!("rust drawn {w}x{h} stride={stride}"));
    unsafe { fl_flush(ar.1) };
}

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

    // Ask the shell for the real stride: the surface is screen-sized, not W*H.
    let mut stride: c_int = 0;
    let px = unsafe { fl_resize_ensure(win, W, H, &mut stride) };
    assert!(!px.is_null() && stride >= W, "fl_resize_ensure failed");
    // SAFETY: surface holds at least W*H BGRA pixels at `stride` per row.
    let canvas = unsafe { Canvas::new(px as *mut u32, W, H, stride) };
    let mut holder = Box::new(AppRef(App::new(canvas), win));
    app_ref.store(&mut *holder as *mut AppRef, Ordering::Relaxed);

    present();

    unsafe {
        fl_on_key(win, on_key, ptr::null_mut());
        fl_on_mouse(win, on_mouse, ptr::null_mut());
        fl_on_resize(win, on_resize, ptr::null_mut());
        fl_flush(win);
        fl_run(win);
        fl_close(win);
    }
    drop(holder);
}
