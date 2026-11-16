// input: platform-neutral events. window.m only ever sends keycodes + points.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Escape,
    Q,
    Left,
    Right,
    Up,
    Down,
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
}

impl Mods {
    pub fn from_bits(bits: u32) -> Self {
        Self {
            cmd: bits & (1 << 0) != 0,
            shift: bits & (1 << 1) != 0,
            ctrl: bits & (1 << 2) != 0,
            alt: bits & (1 << 3) != 0,
        }
    }
}

impl Key {
    /// macOS virtual keycodes (layout independent).
    pub fn from_keycode(code: i32) -> Self {
        match code {
            53 => Key::Escape,
            12 => Key::Q,
            123 => Key::Left,
            124 => Key::Right,
            125 => Key::Down,
            126 => Key::Up,
            _ => Key::Other,
        }
    }
}
