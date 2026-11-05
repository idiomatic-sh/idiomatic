//! Explicit worker modes for profiling and durable rule/file diagnostics.

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
        "run-ryo" => Some(WorkerTracing::Disabled),
        "profile-ryo" => Some(WorkerTracing::Chrome),
        "diagnostic-ryo" => Some(WorkerTracing::JsonLines),
        _ => None,
    }
}
