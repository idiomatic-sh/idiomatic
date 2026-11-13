//! What a certified measurement remembers from before its paired run.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use std::path::PathBuf;

/// The sources a run reads and their content identity before the contestants touch them.
#[derive(Clone, Debug)]
pub struct RunBaseline {
    /// Every source path of the request, under its root.
    pub sources: Vec<PathBuf>,
    /// The content identity of those sources before the run.
    pub before: ArtifactDigest,
}
