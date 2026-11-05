//! Explicit identity of the isolated resource collector.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use std::path::PathBuf;
/// Executable bytes pinned by the benchmark certificate.
#[derive(Clone, Debug)]
pub(crate) struct ExecutionTimedSettings {
    /// Absolute immutable collector executable.
    pub executable: PathBuf,
    /// SHA256 of the exact executable bytes, without framing.
    pub digest: ArtifactDigest,
}
