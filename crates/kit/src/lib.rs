//! Comet design kit — reusable visual layer for gpui apps.
//!
//! Scope (phase 1):
//! - [`theme`] — light/dark token set (oklch neutrals + accents)
//! - [`icons`] — Solar Icons + hand-drawn glyphs + brand marks
//! - [`fonts`] — embedded Geist / Geist Mono registration
//!
//! Product screens (shell, transcript, composer, …) stay in `comet-ui`.
//! Attribution for third-party assets: [`ATTRIBUTION.md`](../ATTRIBUTION.md).

pub mod fonts;
pub mod icons;
pub mod theme;

pub use fonts::register_fonts;
pub use icons::{Assets, icon};
pub use theme::{Appearance, Theme};
