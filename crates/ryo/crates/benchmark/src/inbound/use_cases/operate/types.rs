//! What one operation is asked for and what it answers.
use crate::domain::operations::types::{BenchDirectory, Operation};

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
    pub operation: Operation,
    /// The bench it runs on.
    pub bench: BenchDirectory,
}

/// What an operation answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationReport {
    /// Whether it passed.
    pub verdict: OperationVerdict,
}
