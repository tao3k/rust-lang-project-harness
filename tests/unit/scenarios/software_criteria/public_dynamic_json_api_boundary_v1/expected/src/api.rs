//! Public API shape that exposes named request and response contracts.

/// Stable identifier carried by public payload contracts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadId(String);

impl PayloadId {
    /// Creates a payload identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Request payload accepted by the public API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadRequest {
    id: PayloadId,
}

impl PayloadRequest {
    /// Creates a typed request payload.
    pub fn new(id: PayloadId) -> Self {
        Self { id }
    }

    /// Returns the payload identifier.
    pub fn id(&self) -> &PayloadId {
        &self.id
    }
}

/// Response payload returned by the public API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadResponse {
    accepted: bool,
}

impl PayloadResponse {
    /// Creates a typed response payload.
    pub fn accepted() -> Self {
        Self { accepted: true }
    }

    /// Reports whether the payload was accepted.
    pub fn is_accepted(&self) -> bool {
        self.accepted
    }
}

/// Accepts a named request payload and returns a named response.
pub fn accept_payload(payload: PayloadRequest) -> PayloadResponse {
    if payload.id().as_str().is_empty() {
        return PayloadResponse { accepted: false };
    }
    PayloadResponse::accepted()
}

/// Applies typed patches to a service resource.
pub struct Service;

impl Service {
    /// Returns a typed response instead of raw JSON.
    pub fn update(&self, patch: PayloadRequest) -> Option<PayloadResponse> {
        if patch.id().as_str().is_empty() {
            return None;
        }
        Some(PayloadResponse::accepted())
    }
}
