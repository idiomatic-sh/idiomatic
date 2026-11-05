//! Builds the suite adapters from dependencies chosen by the composition.
use super::adapters::local::adapter::SuiteLocalAdapter;
/// Construct the local implementation of the suite contract.
#[must_use]
pub fn local() -> impl super::interface::SuitePort {
    SuiteLocalAdapter::new(None)
}

/// Construct the local suite with an explicit raw case-receipt destination.
#[must_use]
pub fn local_with_case_receipts(path: std::path::PathBuf) -> impl super::interface::SuitePort {
    SuiteLocalAdapter::new(Some(path))
}
