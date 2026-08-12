//! Native ProofShip Launch Studio services.
//!
//! The engine side is intentionally small and deterministic: a machine gate
//! runner that mirrors `proofship/bridge/server.mjs` and a local JSON launch
//! store. Agent drafting (harness lanes) and repair loops are layered on top.

pub mod draft;
pub mod gate;
mod launch_run;
pub mod store;

pub use draft::{DraftError, DraftRunner};
pub use gate::{GateConfig, GateError, StudioGate, StudioPaths};
pub use launch_run::StudioLaunchRunner;
pub use store::{StoreError, StudioStore};
