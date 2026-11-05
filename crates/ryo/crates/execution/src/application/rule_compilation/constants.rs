//! Limits owned by the bounded rule compilation operation.

/// How many compiled plans the use cases sharing one engine keep: above it, the plan kept longest is dropped, whatever the run in
/// progress names, so a session linting under new rule ids again and again stays bounded and concurrent runs lose one plan at a time.
pub(crate) const COMPILED_PLANS: usize = 4096;
