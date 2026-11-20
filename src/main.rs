// main: winit wiring only. Framework + presenters are the `fluyer-pxui` crate, the
// app's composition lives in `ui`, its logic in `core` + `app`.

mod app;
mod core;
mod input;
mod ui;

use std::rc::Rc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Window, WindowId};

use app::{App, Outcome};
use fluyer_pxui::platform::{self, Presenter};
use input::{Event, Key, Mods};

const W: u32 = 800;
const H: u32 = 450;
const TICK: Duration = Duration::from_millis(250);

struct Shell {
    window: Rc<Window>,
    presenter: Presenter,
    app: App,
    cursor: (f64, f64), // design px
    mods: ModifiersState,
    last_tick: Instant,
    early_frames: u8, // the window can be shown after our first present
}

impl Shell {
    fn present(&mut self) {
        self.presenter.draw(|c| self.app.draw(c));
    }

    fn dispatch(&mut self, ev: Event) {
        match self.app.handle(ev) {
            Outcome::Quit => std::process::exit(0),
            Outcome::Redraw => self.present(),
            Outcome::Idle => {}
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.presenter.resize(&self.window, size);
        self.app.resize(self.presenter.size().0, self.presenter.size().1);
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

        let size = window.inner_size();
        let Some(presenter) = Presenter::attach(&window, size) else {
            eprintln!("presenter attach failed");
            el.exit();
            return;
        };

        let design = presenter.size();
        let mut sh = Shell {
            window,
            presenter,
            app: App::new(design.0, design.1),
            cursor: (0.0, 0.0),
            mods: ModifiersState::empty(),
            last_tick: Instant::now(),
            early_frames: 4,
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
                let (x, y) = sh.presenter.cursor(position);
                sh.cursor = (x as f64, y as f64);
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
            if sh.early_frames > 0 {
                // Window can come on screen after the first present; repaint a few
                // times so the first paint is never lost.
                sh.early_frames -= 1;
                sh.present();
            }
            if sh.last_tick.elapsed() >= TICK {
                sh.last_tick = Instant::now();
                sh.dispatch(Event::Tick);
            }
        }
        el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + TICK));
    }
}

fn main() {
    platform::reexec_nano_off();
    let el = EventLoop::new().expect("event loop");
    let mut h = Handler { shell: None };
    el.run_app(&mut h).expect("run");
    core::player::Player::new().quit();
}
