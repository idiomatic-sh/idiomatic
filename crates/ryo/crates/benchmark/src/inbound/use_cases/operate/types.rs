//! What one operation is asked for and what it answers.
use crate::domain::operations::types::{BenchDirectory, BenchOperation};
use ryo_benchmark_contracts::domain::requests::types::case_shard::value::ShardCount;

/// Whether an operation passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationVerdict {
    /// The operation's worker, or its in-process verification, passed.
    Passed,
    /// It ran and failed.
    Failed,
}

/// One operation of one bench.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationRequest {
    /// The operation to run.
    pub operation: BenchOperation,
    /// The bench it runs on.
    pub bench: BenchDirectory,
    /// How many shards the correctness operation verifies the inventory in at once; one verifies it serially, and every
    /// other operation takes one.
    pub shards: ShardCount,
}

/// What an operation answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationReport {
    /// Whether it passed.
    pub verdict: OperationVerdict,
}

impl From<crate::outbound::executables::types::RunVerdict> for OperationVerdict {
    fn from(run: crate::outbound::executables::types::RunVerdict) -> Self {
        match run {
            crate::outbound::executables::types::RunVerdict::Passed => Self::Passed,
            crate::outbound::executables::types::RunVerdict::Failed => Self::Failed,
        }
    }
}
