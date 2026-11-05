//! One value: its declaration and its hand-written impls, trait impls included.
#![expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "crate-scoped fields let checked workflows issue evidence while preventing external callers from constructing certified values"
)]
use crate::domain::suite::types::audit::value::Audit;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use std::path::PathBuf;
/// Evidence issued after actual cases pass, bound to exact artifacts and executables.
///
/// ```compile_fail
/// use ryo_benchmark::domain::suite::types::certificate::value::Certificate;
/// fn forge(evidence: &mut Certificate) { evidence.fingerprint = evidence.fingerprint().clone(); }
/// ```
#[derive(Clone, Debug)]
pub struct Certificate {
    /// Exact registered settings/evidence contexts; never flattened into measured profile semantics.
    pub(crate) case_source: ryo_benchmark_contracts::domain::requests::types::CertificationCases,
    /// Audited subset covered by the executed case proof.
    pub(crate) coverage: Audit,
    /// Exact artifacts whose framed content fingerprint must remain unchanged.
    pub(crate) paths: Vec<PathBuf>,
    /// Content identity captured before and after certification.
    pub(crate) fingerprint:
        ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest,
    /// Reference and candidate executable paths in that order.
    pub(crate) contestants: [PathBuf; 2],
    /// Fixed request used to bind contestant invocation configuration.
    pub(crate) probe: RunRequest,
    /// Prepared reference and candidate invocations for the probe.
    pub(crate) profiles: [ryo_benchmark_contracts::domain::measurements::types::Invocation; 2],
    /// Exact collection mode and collector identity used during certification.
    pub(crate) execution: ryo_benchmark_contracts::domain::measurements::types::ExecutionProfile,
    /// Number of independent cases passed by both contestants.
    pub(crate) cases:
        ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount,
}

impl Certificate {
    /// Context model under which independent cases were certified.
    #[must_use]
    pub fn case_contexts(
        &self,
    ) -> &ryo_benchmark_contracts::domain::requests::types::CertificationCases {
        &self.case_source
    }
    /// Stable content identity of profile, rule/case bytes and contestant executables.
    #[must_use]
    pub fn fingerprint(
        &self,
    ) -> &ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest {
        &self.fingerprint
    }
    /// Number of independently authored cases that both contestants passed.
    #[must_use]
    pub fn cases(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount {
        self.cases
    }
}
