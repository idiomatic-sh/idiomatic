//! The texts of the `lint` use case.

/// What the answer of a task says when no worker answered it.
pub(super) const TASK_ANSWERED: &str = "a task every worker took until none was left";

/// How many rows an answer of the rules' own columns writes when its caller does not say.
pub const DEFAULT_LIMIT: u64 = 20;
