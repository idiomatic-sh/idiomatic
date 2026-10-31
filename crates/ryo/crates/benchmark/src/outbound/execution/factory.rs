//! Builds the execution adapters from dependencies chosen by the composition.
use super::adapters::process::adapter::ExecutionProcessAdapter;
/// Construct the process implementation of the execution contract.
#[must_use]
pub fn process() -> impl super::interface::ExecutionPort {
    ExecutionProcessAdapter
}
