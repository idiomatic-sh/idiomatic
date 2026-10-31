//! Builds the suite adapters from dependencies chosen by the composition.
use super::adapters::local::adapter::SuiteLocalAdapter;
/// Construct the local implementation of the suite contract.
#[must_use]
pub fn local() -> impl super::interface::SuitePort {
    SuiteLocalAdapter
}
