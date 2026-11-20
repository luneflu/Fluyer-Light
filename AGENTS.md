# fluyer-low
Pixel music player. Rust workspace, no runtime UI dependency besides winit/softbuffer.

```
crates/pxui   fluyer-pxui  canvas, baked wood text, widgets, presenters
crates/audio  fluyer-audio CoreAudio (macOS) / Media Foundation (Windows)
src/          fluyer-low  bin: main(winit) → app(glue) → core(logic) + ui(composition)
```

Read only what the task touches:

| file | when |
|---|---|
| [docs/ARCH.md](docs/ARCH.md) | crate boundaries, data flow, build commands, where a change goes |
| [docs/UI.md](docs/UI.md) | canvas primitives, theme metrics, widgets, layout, list 9-slice |
| [docs/TEXT.md](docs/TEXT.md) | baked wood font atlas, gen_font variants, text/art scale grid |
| [docs/PRESENTER.md](docs/PRESENTER.md) | macOS IOSurface presenter, softbuffer fallback, resize/cursor mapping |
| [docs/AUDIO.md](docs/AUDIO.md) | playback API, C backends, link flags |

## Docs are part of the change
Adding, modifying or removing a feature means the same commit updates the doc that
describes it (`ARCH` for structure/deps, `UI` for widgets/layout/theme, `TEXT` for the
atlas, `PRESENTER` for window/present, `AUDIO` for playback). Stale docs are a bug:
if a signature, constant, file path or trap changed, fix the doc line in the same edit.
New subsystem -> new `docs/<NAME>.md` plus a row in the table above.

Generated, gitignored, never edit or commit:
`crates/pxui/src/font_atlas.rs`, `crates/pxui/src/buttons.rs`.
