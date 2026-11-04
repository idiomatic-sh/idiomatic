//! Application-owned result of classifying one complete worker request.
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::types::PreparedMissingInputs;

/// The request and its sealed missing-input decision remain paired until dispatch.
pub struct PreparedRequest {
    /// Original ordered transport inventory.
    pub request: WorkerRequest,
    /// Paths proved absent by the matching sealed context.
    pub missing: PreparedMissingInputs,
}
