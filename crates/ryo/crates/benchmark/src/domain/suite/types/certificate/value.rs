//! One value: its declaration and its hand-written impls, trait impls included.
use crate::domain::suite::types::{RunRequest, audit::value::Audit};
use std::path::PathBuf;
/// Evidence issued after actual cases pass, bound to exact artifacts and executables.
///
/// ```compile_fail
/// use ryo_benchmark::domain::suite::types::certificate::value::Certificate;
/// fn forge(evidence: &mut Certificate) { evidence.fingerprint = evidence.fingerprint().clone(); }
/// ```
#[derive(Clone, Debug)]
pub struct Certificate {
    pub(crate) coverage: Audit,
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) fingerprint: crate::domain::suite::types::ArtifactDigest,
    pub(crate) contestants: [PathBuf; 2],
    pub(crate) probe: RunRequest,
    pub(crate) profiles: [crate::domain::measurements::types::Invocation; 2],
    pub(crate) cases: crate::domain::suite::types::CaseCount,
}

impl Certificate {
    /// Stable content identity of profile, rule/case bytes and contestant executables.
    pub fn fingerprint(&self) -> &crate::domain::suite::types::ArtifactDigest {
        &self.fingerprint
    }
    /// Number of independently authored cases that both contestants passed.
    pub fn cases(&self) -> crate::domain::suite::types::CaseCount {
        self.cases
    }
}
