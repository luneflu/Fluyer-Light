// input: platform-neutral events. window.m only ever sends keycodes + points.

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
    Tick, // main-run-loop timer; app decides whether it needs a repaint
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
    #[cfg(target_os = "macos")]
    pub fn from_keycode(code: i32) -> Self {
        match code {
            53 => Key::Escape,
            12 => Key::Q,
            36 => Key::Enter,
            49 => Key::Space,
            123 => Key::Left,
            124 => Key::Right,
            125 => Key::Down,
            126 => Key::Up,
            _ => Key::Other,
        }
    }

    /// Win32 virtual-key codes.
    #[cfg(not(target_os = "macos"))]
    pub fn from_keycode(code: i32) -> Self {
        match code {
            0x1B => Key::Escape,
            0x51 => Key::Q,
            0x0D => Key::Enter,
            0x20 => Key::Space,
            0x25 => Key::Left,
            0x27 => Key::Right,
            0x28 => Key::Down,
            0x26 => Key::Up,
            _ => Key::Other,
        }
    }
}
