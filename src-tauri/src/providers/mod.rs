//! Read-only providers used by the dashboard.
//!
//! Provider code deliberately returns only `ProviderSnapshot`s.  In particular,
//! the JSON-RPC responses and WinHTTP error bodies never cross this module's
//! boundary.

pub(crate) mod codex;
pub(crate) mod reset;
mod worker;

use std::path::Path;

use serde_json::Value;

pub use codex::{read_codex, CodexAppServerClient, CodexError, JsonlFramer, READ_ONLY_METHODS};
pub use reset::{read_reset, ResetClient, ResetError};
pub use worker::spawn;

/// Make a model snapshot in one place.  Keeping this small also makes the
/// provider implementations usable while the model is being evolved by the
/// root agent.
pub(crate) fn snapshot(
    state: &str,
    last_success: Option<f64>,
    error: Option<&str>,
    data: Option<Value>,
) -> crate::model::ProviderSnapshot {
    crate::model::ProviderSnapshot {
        state: state.to_owned(),
        last_success,
        error: error.map(str::to_owned),
        data,
    }
}

/// The public entry point requested by the shared contract.
pub fn read_codex_snapshot(cli: Option<&Path>) -> crate::model::ProviderSnapshot {
    codex::read_codex(cli)
}

/// The public entry point requested by the shared contract.
pub fn read_reset_snapshot() -> crate::model::ProviderSnapshot {
    reset::read_reset()
}
