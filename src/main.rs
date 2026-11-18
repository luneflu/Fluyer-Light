// main: winit window + platform presenter.
// macOS: one BGRA IOSurface at design resolution assigned as the view's layer
// contents (present.m) — the compositor nearest-upscales, so the backing buffer is
// 800x450x4 instead of softbuffer's two 1600x900x4 ARGB buffers.
// Other platforms: softbuffer, repainted in full on every present.

mod app;
mod audio;
mod buttons;
mod font;
mod font_atlas;
mod gfx;
mod input;
mod library;
mod widget;

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowId};

use app::{App, Outcome};
use gfx::Canvas;
use input::{Event, Key, Mods};

const W: u32 = 800;
const H: u32 = 450;
const TICK: Duration = Duration::from_millis(250);

#[cfg(target_os = "macos")]
extern "C" {
    fn flp_attach(nsview: *mut c_void, w: c_int, h: c_int) -> c_int;
    fn flp_resize(w: c_int, h: c_int) -> c_int;
    fn flp_buffer() -> *mut c_void;
    fn flp_stride() -> c_int;
    fn flp_flush();
}

/// The winit window's NSView, for the macOS presenter.
#[cfg(target_os = "macos")]
fn ns_view(window: &Window) -> Option<*mut c_void> {
    let h = window.window_handle().ok()?;
    match h.as_raw() {
        RawWindowHandle::AppKit(a) => Some(a.ns_view.as_ptr()),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
use std::ffi::c_int;
#[cfg(target_os = "macos")]
use std::ffi::c_void;

struct Shell {
    window: Rc<Window>,
    #[cfg(not(target_os = "macos"))]
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    app: App,
    size: (i32, i32),   // logical px == design px, matches the surface
    cursor: (f64, f64), // logical px
    mods: ModifiersState,
    last_tick: Instant,
}

impl Shell {
    #[cfg(target_os = "macos")]
    fn present(&mut self) {
        let (w, h) = self.size;
        // SAFETY: presenter owns a surface of at least stride * h px for its lifetime.
        unsafe {
            let stride = flp_stride() as usize;
            let base = flp_buffer() as *mut u32;
            if base.is_null() || stride == 0 {
                return;
            }
            let px = std::slice::from_raw_parts_mut(base, stride * h as usize);
            let mut c = Canvas::with_stride(px, w, h, stride as i32);
            self.app.draw(&mut c);
            flp_flush();
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn present(&mut self) {
        let (w, h) = self.size;
        let Ok(mut buf) = self.surface.buffer_mut() else { return };
        let mut c = Canvas::new(&mut buf, w, h);
        self.app.draw(&mut c);
        let _ = buf.present();
    }

    fn dispatch(&mut self, ev: Event) {
        match self.app.handle(ev) {
            Outcome::Quit => std::process::exit(0),
            Outcome::Redraw | Outcome::Repaint(_) => self.present(),
            Outcome::Idle => {}
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        // macOS: design px == window points; the compositor nearest-upscales, so the
        // surface stays 800x450x4 instead of the backing store's 1600x900x4.
        #[cfg(target_os = "macos")]
        let (w, h) = {
            let s = self.window.scale_factor();
            (
                (size.width as f64 / s).round().max(1.0) as u32,
                (size.height as f64 / s).round().max(1.0) as u32,
            )
        };
        #[cfg(not(target_os = "macos"))]
        let (w, h) = (size.width, size.height);
        let (Some(w), Some(h)) = (NonZeroU32::new(w), NonZeroU32::new(h)) else {
            return;
        };
        #[cfg(target_os = "macos")]
        if unsafe { flp_resize(w.get() as c_int, h.get() as c_int) } == 0 {
            return;
        }
        #[cfg(not(target_os = "macos"))]
        if self.surface.resize(w, h).is_err() {
            return;
        }
        self.size = (w.get() as i32, h.get() as i32);
        self.app.resize(self.size.0, self.size.1);
        self.present();
    }
}

fn map_key(k: PhysicalKey) -> Key {
    let PhysicalKey::Code(c) = k else { return Key::Other };
    match c {
        KeyCode::Escape => Key::Escape,
        KeyCode::KeyQ => Key::Q,
        KeyCode::Enter => Key::Enter,
        KeyCode::Space => Key::Space,
        KeyCode::ArrowLeft => Key::Left,
        KeyCode::ArrowRight => Key::Right,
        KeyCode::ArrowUp => Key::Up,
        KeyCode::ArrowDown => Key::Down,
        _ => Key::Other,
    }
}

struct Handler {
    shell: Option<Shell>,
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.shell.is_some() {
            return;
        }
        let window = match el.create_window(
            Window::default_attributes()
                .with_title("fluyer-low")
                .with_inner_size(LogicalSize::new(W, H))
                .with_min_inner_size(LogicalSize::new(64, 48)),
        ) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("window: {e}");
                el.exit();
                return;
            }
        };
        #[cfg(not(target_os = "macos"))]
        let surface = {
            let surface = softbuffer::Context::new(window.clone())
                .and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone()));
            match surface {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("softbuffer: {e}");
                    el.exit();
                    return;
                }
            }
        };

        let s = window.scale_factor();
        let size = window.inner_size();
        #[cfg(target_os = "macos")]
        let design = (
            (size.width as f64 / s).round().max(1.0) as i32,
            (size.height as f64 / s).round().max(1.0) as i32,
        );
        #[cfg(not(target_os = "macos"))]
        let design = (size.width as i32, size.height as i32);

        #[cfg(target_os = "macos")]
        {
            let Some(view) = ns_view(&window) else {
                eprintln!("macos: no NSView");
                el.exit();
                return;
            };
            if unsafe { flp_attach(view, design.0 as c_int, design.1 as c_int) } == 0 {
                eprintln!("macos: presenter attach failed");
                el.exit();
                return;
            }
        }

        let mut sh = Shell {
            window,
            #[cfg(not(target_os = "macos"))]
            surface,
            app: App::new(design.0, design.1),
            size: design,
            cursor: (0.0, 0.0),
            mods: ModifiersState::empty(),
            last_tick: Instant::now(),
        };
        sh.resize(size);
        self.shell = Some(sh);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(sh) = self.shell.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => sh.resize(size),
            WindowEvent::RedrawRequested => sh.present(),
            WindowEvent::ModifiersChanged(m) => sh.mods = m.state(),
            WindowEvent::CursorMoved { position, .. } => {
                // macOS canvas is in points (design px); softbuffer's is in physical px.
                #[cfg(target_os = "macos")]
                let p = (position.x, position.y);
                #[cfg(not(target_os = "macos"))]
                let p = {
                    let s = sh.window.scale_factor();
                    (position.x * s, position.y * s)
                };
                sh.cursor = p;
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => match state {
                ElementState::Pressed => {
                    let (x, y) = (sh.cursor.0 as i32, sh.cursor.1 as i32);
                    sh.dispatch(Event::MouseDown { x, y });
                }
                ElementState::Released => sh.dispatch(Event::MouseUp),
            },
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed || event.repeat {
                    return;
                }
                let key = map_key(event.physical_key);
                let m = sh.mods;
                let mods = Mods::new(m.super_key(), m.shift_key(), m.control_key(), m.alt_key());
                sh.dispatch(Event::Key { key, mods });
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if let Some(sh) = self.shell.as_mut() {
            if sh.last_tick.elapsed() >= TICK {
                sh.last_tick = Instant::now();
                sh.dispatch(Event::Tick);
            }
        }
        el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + TICK));
    }
}

/// macOS: drop the malloc nano zone before anything allocates (~1.5 MB).
#[cfg(target_os = "macos")]
fn reexec_nano_off() {
    use std::ffi::{CString, c_char};
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

#[cfg(not(target_os = "macos"))]
fn reexec_nano_off() {}

fn main() {
    reexec_nano_off();
    let el = EventLoop::new().expect("event loop");
    let mut h = Handler { shell: None };
    el.run_app(&mut h).expect("run");
    audio::quit();
}
