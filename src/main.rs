// main: winit window + softbuffer present. The shell only allocates the window
// and forwards keys/points; Canvas owns the pixels and is copied out on present.

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
use winit::window::{Window, WindowId};

use app::{App, Outcome};
use input::{Event, Key, Mods};

const W: u32 = 800;
const H: u32 = 450;
const TICK: Duration = Duration::from_millis(250);

struct Shell {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    app: App,
    cursor: (f64, f64), // logical px
    mods: ModifiersState,
    last_tick: Instant,
}

impl Shell {
    fn present(&mut self) {
        self.app.draw();
        if self.app.canvas.take_dirty().w <= 0 {
            return;
        }
        let size = self.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        let Ok(mut buf) = self.surface.buffer_mut() else { return };
        // The canvas is the same size as the window, so this is one memcpy.
        let px = self.app.canvas.pixels();
        if buf.len() == px.len() {
            buf.copy_from_slice(px);
        }
        let _ = buf.present();
        let _ = (w, h);
    }

    fn dispatch(&mut self, ev: Event) {
        match self.app.handle(ev) {
            Outcome::Quit => std::process::exit(0),
            Outcome::Redraw => self.present(),
            Outcome::Idle => {}
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if self.surface.resize(w, h).is_err() {
            return;
        }
        self.app.resize(w.get() as i32, h.get() as i32);
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
        let surface = softbuffer::Context::new(window.clone())
            .and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone()));
        let surface = match surface {
            Ok(s) => s,
            Err(e) => {
                eprintln!("softbuffer: {e}");
                el.exit();
                return;
            }
        };
        let size = window.inner_size();
        let mut sh = Shell {
            window,
            surface,
            app: App::new(size.width as i32, size.height as i32),
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
                let s = sh.window.scale_factor();
                sh.cursor = (position.x / s, position.y / s);
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

fn main() {
    let el = EventLoop::new().expect("event loop");
    let mut h = Handler { shell: None };
    el.run_app(&mut h).expect("run");
    audio::quit();
}
