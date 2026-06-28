//! Public API shape that leaks untyped JSON payload contracts.

use serde_json::Value;

/// Accepts raw payloads without a named contract.
pub fn accept_payload(payload: Value) -> Value {
    payload
}

/// Applies untyped patches to a service resource.
pub struct Service;

impl Service {
    /// Returns a raw JSON response without a named response type.
    pub fn update(&self, patch: serde_json::Value) -> Option<Value> {
        Some(patch)
    }
}
