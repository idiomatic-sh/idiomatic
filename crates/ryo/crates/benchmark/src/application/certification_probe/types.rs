//! Data owned by the preparation identity probe.
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;

/// The probe request and the invocation each contestant prepared for it.
pub struct ProbedProfiles {
    /// The request both contestants prepared.
    pub probe: RunRequest,
    /// The reference's and the candidate's prepared invocation, in that order.
    pub profiles: [Invocation; 2],
}
