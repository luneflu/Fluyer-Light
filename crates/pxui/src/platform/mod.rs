// platform: window surface presenters. The only part of the framework that knows
// about winit; drawing modules stay OS-free.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod softbuffer_presenter;

#[cfg(target_os = "macos")]
pub use macos::{reexec_nano_off, Presenter};
#[cfg(not(target_os = "macos"))]
pub use softbuffer_presenter::Presenter;

/// Drop the malloc nano zone before anything allocates. No-op off macOS.
#[cfg(not(target_os = "macos"))]
pub fn reexec_nano_off() {}
