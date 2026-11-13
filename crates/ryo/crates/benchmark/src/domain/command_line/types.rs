//! A command line's arguments and the commands and request sources they name.
pub mod measure_command;
pub mod worker_command;
pub mod worker_input;

/// Text supplied as one executable command argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkArgument(pub String);

/// The JSON text of one inline worker request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineRequest(pub String);

/// How a standalone worker handles rule/file tracing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerTracing {
    /// Ordinary correctness or timing invocation.
    Disabled,
    /// Chrome trace for an explicit profiling invocation.
    Chrome,
    /// Flushed JSONL events for a bounded diagnostic invocation.
    JsonLines,
}

/// What a measure invocation selected: a comma list of rules, or the registered case inventory.
#[derive(Clone, Copy, Debug)]
pub struct Selection<'a>(pub &'a str);
