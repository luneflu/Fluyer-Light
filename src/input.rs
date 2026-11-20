// input: platform-neutral events. The shell only ever hands us keys and points.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Escape,
    Q,
    Left,
    Right,
    Up,
    Down,
    Enter,
    Space,
    Other,
}

#[derive(Clone, Copy, Default, Debug)]
#[allow(dead_code)] // shift/ctrl/alt are wired, unused until an accelerator needs them
pub struct Mods {
    pub cmd: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Key { key: Key, mods: Mods },
    MouseDown { x: i32, y: i32 },
    MouseUp,
    // Scroll amount, positive = down. `pixels` means the value is a pixel
    // delta (trackpad) rather than wheel lines.
    Wheel { rows: f64, pixels: bool },
    Tick, // main-run-loop timer; app decides whether it needs a repaint
}

impl Mods {
    /// Named because winit is now the only source of modifiers.
    pub fn new(cmd: bool, shift: bool, ctrl: bool, alt: bool) -> Self {
        Self { cmd, shift, ctrl, alt }
    }
}
