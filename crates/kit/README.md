# comet-kit

Reusable **design kit** for gpui apps extracted from ProofShip / comet-ui.

## In scope

| Module | What |
|--------|------|
| `theme` | Light/dark token set (`Theme`, `Appearance`, washes, glass helpers) |
| `icons` | Solar Icons + hand glyphs + brand marks; `Assets` (`AssetSource`) |
| `fonts` | Embedded Geist / Geist Mono via `register_fonts` |

## Out of scope

Shell, sidebar, transcript, composer, terminal, settings, motion primitives, frost, loaders, popovers — those stay in `comet-ui`.

## Use

```rust
use comet_kit::{Assets, register_fonts, theme::Theme};

let app = gpui_platform::application().with_assets(Assets);
app.run(|cx| {
    register_fonts(cx);
    Theme::install(Appearance::Dark, cx);
});
```

Attribution for Solar Icons and Geist: see [`ATTRIBUTION.md`](ATTRIBUTION.md).
