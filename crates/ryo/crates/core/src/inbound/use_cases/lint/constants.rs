//! The texts of the `lint` use case.

/// What the answer of a task says when no worker answered it.
pub(super) const TASK_ANSWERED: &str = "a task every worker took until none was left";

/// How many rows an answer of the rules' own columns writes when its caller does not say.
pub const DEFAULT_LIMIT: u64 = 20;

/// How many compiled plans the use cases sharing one engine keep: above it, the plan kept longest is dropped, whatever the run in
/// progress names, so a session linting under new rule ids again and again stays bounded and concurrent runs lose one plan at a time.
pub(super) const COMPILED_PLANS: usize = 4096;
