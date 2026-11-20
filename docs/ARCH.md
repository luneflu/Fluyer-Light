# Architecture

One-way deps. No trait objects, no state in widgets.

```
main.rs  winit loop, Presenter, event → app
  app.rs  Event → ui::view::hit → core::model::Action → core::model::Changed → present
    core/   model.rs  Model { tracks, selected, player } + Action + Changed
            player.rs transport over fluyer-audio
            library.rs fs scan of 15 audio extensions
    ui/     layout.rs Layout::new(w,h,inset) → rects, slot(i), name_room
            view.rs   render(&Model,&Layout,&ListView,press,progress,&mut Canvas) — pure
    fluyer-pxui   gfx, font, theme, widgets, platform::Presenter
    fluyer-audio  flat fns: init/play/toggle/seek/is_playing/progress/quit
```

Rules, grep-checkable:
- `core` must not contain `crate::ui` or `winit`.
- `ui` must not contain `crate::platform` or `crate::app`.
- `crates/*` must not contain `crate::core` or `crate::app`.
- UI holds no state: list scroll is derived from `Model::selected` by `ListView::scroll_for`.
- Colours and sizes only in `crates/pxui/src/theme.rs`.
- Design px vs device px: canvas is design px everywhere; only the presenter maps points.

## Build / bake
```
cargo build --release --offline
cargo run -p fluyer-pxui --release --example gen_font -- 48 7   # atlas
cargo run -p fluyer-pxui --release --example gen_buttons        # sprites
```
Bakers resolve `CARGO_MANIFEST_DIR/../../assets` and write into `crates/pxui/src/`.

## Traps
- Verify visuals with the user on macOS; scripted capture cannot see flicker.
- Do not commit unless asked.
- Changing `font_atlas::SIZE`/`SCALE`/`ART` silently breaks layout: derive metrics from the atlas, never hardcode em.
- `panic = "abort"`, `lto = "fat"`: full rebuilds are slow, keep changes batched.
