//! Native ProofShip Launch Studio services.
//!
//! The engine side is intentionally small and deterministic: a machine gate
//! runner that mirrors `proofship/bridge/server.mjs` and a local JSON launch
//! store. Agent drafting (harness lanes) and repair loops are layered on top.

pub mod deploy;
pub mod draft;
pub mod gate;
pub mod interact;
mod launch_run;
pub mod networks;
pub mod preview;
pub mod store;
pub mod walletconnect;
pub mod wallets;

pub use deploy::{
    DeployStore, DeployStoreError, StudioDeployer, artifact_bin_path, preflight,
};
pub use draft::{DraftError, DraftRunner};
pub use gate::{GateConfig, GateError, StudioGate, StudioPaths};
pub use interact::StudioInteract;
pub use launch_run::StudioLaunchRunner;
pub use networks::{NetworkError, NetworkStore};
pub use preview::StudioPreview;
pub use store::{StoreError, StudioStore};
pub use walletconnect::{WalletConnectBridge, resolve_project_id};
pub use wallets::{WalletError, WalletStore};
