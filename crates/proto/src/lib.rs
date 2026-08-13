//! comet-proto — wire types shared by engine, UI, and RPC.
//!
//! Ported from comet's `packages/control/src/wire.ts` + `packages/harness/src/types.ts`.
//! Token-usage *display* types are excluded by design; the `Usage` agent event is kept as a
//! harness-level passthrough (rate-limit meters), never persisted into docs.

pub mod agent;
pub mod deploy;
pub mod entities;
pub mod motion;
pub mod networks;
pub mod view;
pub mod walletconnect;
pub mod wallets;

pub use agent::*;
pub use deploy::*;
pub use entities::*;
pub use networks::*;
pub use walletconnect::*;
pub use wallets::*;
