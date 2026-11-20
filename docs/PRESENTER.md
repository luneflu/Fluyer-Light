# Presenters (`crates/pxui/src/platform/`)

`platform/mod.rs` cfg-selects one backend; only this module knows winit.

```rust
Presenter::attach(&Rc<Window>, PhysicalSize<u32>) -> Option<Self>
presenter.size() -> (i32, i32)                        // design px
presenter.resize(&Window, PhysicalSize<u32>)          // mapping inside the backend
presenter.cursor(PhysicalPosition<f64>) -> (i32, i32) // design px, for hit-testing
presenter.draw(|&mut Canvas|)
platform::reexec_nano_off()
```

## macOS (`macos.rs` + `present.m`)
- One `BGRA` IOSurface (int `0x42475241`) at design px as the NSView layer `contents`; compositor nearest-upscales. Buffer = `w*h*4`, not backing store `w*h*16`.
- `contentsScale 1.0`, `kCAGravityTopLeft`, nearest min/mag, `setContentsChanged` inside a disabled-actions transaction.
- Canvas in points: `Canvas::with_stride(px, w, h, flp_stride())`.
- Built by `crates/pxui/build.rs` with `clang -O2 -fobjc-arc`; links `Cocoa, QuartzCore, IOSurface` + `objc` as a lib (framework `objc` does not exist).

## Windows/Linux (`softbuffer_presenter.rs`)
- Buffers are physical-px sized (softbuffer blits 1:1); repeat `present()` each frame.

## Traps
- Resizing on macOS must allocate a new IOSurface and a fresh CALayer; assigning `layer.contents` in place stops CoreAnimation repainting.
- Cursor mapping is per backend: points on macOS, physical px on softbuffer. Scaling twice makes every click land at double coords.
- `CALayer -setContentsChanged` is SPI: needs a category declaration to compile.
- softbuffer's buffer is not persistent (freshly zeroed on macOS), so never rely on previous contents for partial repaints.
