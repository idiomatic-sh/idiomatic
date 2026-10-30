//! What the `tools` port asks of the outside, in the core's own vocabulary.

use super::types::{Completion, Invocation};

/// What every `tools` adapter implements.
pub trait ToolsPort: std::fmt::Debug + Send + Sync {
    /// Runs `invocation` to its end: an exit, a signal, its timeout, or a failure to start.
    fn run(&self, invocation: &Invocation) -> Completion;
}
