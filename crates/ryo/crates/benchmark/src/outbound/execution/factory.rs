//! Builds the execution adapters from dependencies chosen by the composition.
use super::adapters::process::adapter::ExecutionProcessAdapter;
/// Construct the process implementation of the execution contract.
#[must_use]
pub fn process() -> impl super::interface::ExecutionPort {
    ExecutionProcessAdapter
}

/// Construct direct per-child Linux CPU and peak-RSS instrumentation.
/// Other platforms refuse measurement rather than report cumulative process usage.
#[must_use]
pub fn timed() -> impl super::interface::ExecutionPort {
    super::adapters::timed::adapter::ExecutionTimedAdapter
}
