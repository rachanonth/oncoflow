//! LAN transport. Business rules remain in the existing Rust services.
pub(crate) mod client;
pub(crate) mod host;
mod locking;
mod protocol;
mod server;
pub(crate) use client::ConnectionState;
pub(crate) use locking::lock_database;
#[cfg(feature = "server-cli")]
pub use server::{backup_server, initialize_server, run_server};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
pub(super) type RpcResult = Result<Value, Value>;

pub(super) fn error(code: &str, message: &str) -> Value {
    json!({"code": code, "message": message})
}

pub(super) fn argument<T: DeserializeOwned>(args: &Value, name: &str) -> Result<T, Value> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null)).map_err(|_| {
        error(
            "invalid_argument",
            "A required argument is missing or invalid.",
        )
    })
}

pub(super) fn encode<T: Serialize>(value: T) -> RpcResult {
    serde_json::to_value(value)
        .map_err(|_| error("server_error", "The response could not be encoded."))
}

pub(super) fn encode_result<T: Serialize, E: Serialize>(value: Result<T, E>) -> RpcResult {
    match value {
        Ok(v) => encode(v),
        Err(e) => Err(serde_json::to_value(e)
            .unwrap_or_else(|_| error("server_error", "The operation failed."))),
    }
}
