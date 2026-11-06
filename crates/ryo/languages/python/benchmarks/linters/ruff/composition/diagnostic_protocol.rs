//! Explicit worker modes for profiling and durable rule/file diagnostics.
use super::constants::{DIAGNOSTIC_RYO, PROFILE_RYO, RUN_RYO};

/// How a standalone worker handles rule/file tracing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WorkerTracing {
    /// Ordinary correctness or timing invocation.
    Disabled,
    /// Chrome trace for an explicit profiling invocation.
    Chrome,
    /// Flushed JSONL events for a bounded diagnostic invocation.
    JsonLines,
}

/// Resolve only the three documented worker protocols.
pub(super) fn tracing_for_command(command: &str) -> Option<WorkerTracing> {
    match command {
        RUN_RYO => Some(WorkerTracing::Disabled),
        PROFILE_RYO => Some(WorkerTracing::Chrome),
        DIAGNOSTIC_RYO => Some(WorkerTracing::JsonLines),
        _ => None,
    }
}
