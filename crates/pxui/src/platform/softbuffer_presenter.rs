// Windows/Linux presenter: softbuffer. Buffers are backing-store sized because
// softbuffer blits 1:1, and its buffer contents are not persistent, so every
// present repaints the whole frame.

use std::num::NonZeroU32;
use std::rc::Rc;

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::Window;

use crate::gfx::Canvas;

pub struct Presenter {
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    size: (i32, i32),
}

impl Presenter {
    pub fn attach(window: &Rc<Window>, size: PhysicalSize<u32>) -> Option<Self> {
        let surface = softbuffer::Context::new(window.clone())
            .and_then(|ctx| softbuffer::Surface::new(&ctx, window.clone()))
            .ok()?;
        let mut p = Self { surface, size: (0, 0) };
        p.resize(window, size);
        Some(p)
    }

    pub fn size(&self) -> (i32, i32) {
        self.size
    }

    pub fn resize(&mut self, _window: &Window, size: PhysicalSize<u32>) {
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if self.surface.resize(w, h).is_ok() {
            self.size = (w.get() as i32, h.get() as i32);
        }
    }

    /// Cursor in design px: softbuffer canvases are physical px.
    pub fn cursor(&self, p: PhysicalPosition<f64>) -> (i32, i32) {
        (p.x as i32, p.y as i32)
    }

    pub fn draw(&mut self, f: impl FnOnce(&mut Canvas)) {
        let (w, h) = self.size;
        let Ok(mut buf) = self.surface.buffer_mut() else { return };
        let mut c = Canvas::new(&mut buf, w, h);
        f(&mut c);
        let _ = buf.present();
    }
}
