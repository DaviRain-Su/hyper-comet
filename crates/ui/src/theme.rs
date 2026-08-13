//! Re-export of [`comet_kit::theme`] so existing `crate::theme` paths keep working.
//!
//! The token set lives in `comet-kit` so other gpui apps can share the design
//! system without depending on product UI (`shell`, `transcript`, …).

pub use comet_kit::theme::*;
