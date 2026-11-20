# Audio (`crates/audio/`)

One flat surface, seconds in/out. No UI, no app model, no winit.

```rust
fluyer_audio::init() -> bool
play(&str) -> bool          toggle()          seek(f64 seconds) -> bool
is_playing() -> bool        progress() -> (pos, len)          quit()
```

## Backends
- macOS: `audio.c`, ExtAudioFile decode inside a default-output AUHAL render callback, no worker thread, device opened on first play, loop-on-EOF.
- Windows: `audio_win.c`, Media Foundation (`mfplat/mfreadwrite/mfuuid/ole32/uuid/ksuser`).
- `src/lib.rs` cfg-includes `mac.rs`/`win.rs`; both declare the same `au_*` externs.
- Built by `crates/audio/build.rs` (clang on macOS, `cc` on MSVC).

## Traps
- Device init is lazy: footprint stays low while nothing plays, don't "fix" it by initialising at startup.
- `progress().1 == 0.0` means nothing loaded — `Player::toggle` treats it as "play the selection" instead of resuming.
- Generated/framework crates must not depend on the app; keep `use fluyer_audio as audio` inside `core/player.rs`.
