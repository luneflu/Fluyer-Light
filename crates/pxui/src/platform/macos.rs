// macOS presenter: one BGRA IOSurface at design resolution as the view's layer
// contents (present.m). The compositor nearest-upscales, so the buffer is
// w*h*4 instead of the backing store's w*h*16.

use std::ffi::{c_int, c_void};
use std::rc::Rc;

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::gfx::Canvas;

extern "C" {
    fn flp_attach(nsview: *mut c_void, w: c_int, h: c_int) -> c_int;
    fn flp_resize(w: c_int, h: c_int) -> c_int;
    fn flp_buffer() -> *mut c_void;
    fn flp_stride() -> c_int;
    fn flp_flush();
}

pub struct Presenter {
    size: (i32, i32),
}

fn ns_view(window: &Window) -> Option<*mut c_void> {
    let h = window.window_handle().ok()?;
    match h.as_raw() {
        RawWindowHandle::AppKit(a) => Some(a.ns_view.as_ptr()),
        _ => None,
    }
}

/// macOS canvases are in window points, so physical pixels come back to points.
fn design(window: &Window, size: PhysicalSize<u32>) -> (i32, i32) {
    let s = window.scale_factor();
    (
        (size.width as f64 / s).round().max(1.0) as i32,
        (size.height as f64 / s).round().max(1.0) as i32,
    )
}

impl Presenter {
    pub fn attach(window: &Rc<Window>, size: PhysicalSize<u32>) -> Option<Self> {
        let view = ns_view(window)?;
        let (w, h) = design(window, size);
        if unsafe { flp_attach(view, w as c_int, h as c_int) } == 0 {
            return None;
        }
        Some(Self { size: (w, h) })
    }

    pub fn size(&self) -> (i32, i32) {
        self.size
    }

    pub fn resize(&mut self, window: &Window, size: PhysicalSize<u32>) {
        let (w, h) = design(window, size);
        if (w, h) == self.size {
            return;
        }
        if unsafe { flp_resize(w as c_int, h as c_int) } != 0 {
            self.size = (w, h);
        }
    }

    /// Cursor in design px: points, unscaled.
    pub fn cursor(&self, p: PhysicalPosition<f64>) -> (i32, i32) {
        (p.x as i32, p.y as i32)
    }

    /// Hand the caller the back buffer, then flip it to the screen.
    pub fn draw(&mut self, f: impl FnOnce(&mut Canvas)) {
        let (w, h) = self.size;
        // SAFETY: the presenter owns a surface of at least stride * h px while alive.
        unsafe {
            let stride = flp_stride() as usize;
            let base = flp_buffer() as *mut u32;
            if base.is_null() || stride == 0 {
                return;
            }
            let px = std::slice::from_raw_parts_mut(base, stride * h as usize);
            let mut c = Canvas::with_stride(px, w, h, stride as i32);
            f(&mut c);
            flp_flush();
        }
    }
}

/// Drop the malloc nano zone before anything allocates (~1.5 MB).
pub fn reexec_nano_off() {
    use std::ffi::{c_char, CString};
    extern "C" {
        fn getenv(name: *const c_char) -> *mut c_char;
        fn setenv(name: *const c_char, value: *const c_char, over: i32) -> i32;
        fn _NSGetExecutablePath(buf: *mut c_char, size: *mut u32) -> i32;
        fn execv(path: *const c_char, argv: *const *const c_char) -> i32;
    }
    let key = CString::new("MallocNanoZone").unwrap();
    unsafe {
        if !getenv(key.as_ptr()).is_null() {
            return;
        }
        setenv(key.as_ptr(), c"0".as_ptr(), 1);
        let mut buf = vec![0 as c_char; 4096];
        let mut len = buf.len() as u32;
        _NSGetExecutablePath(buf.as_mut_ptr(), &mut len);
        let args: Vec<CString> = std::env::args_os()
            .map(|a| CString::new(a.to_string_lossy().as_bytes()).unwrap_or_default())
            .collect();
        let mut argv: Vec<*const c_char> = args.iter().map(|a| a.as_ptr()).collect();
        argv.push(std::ptr::null());
        execv(buf.as_ptr(), argv.as_ptr());
    }
}
