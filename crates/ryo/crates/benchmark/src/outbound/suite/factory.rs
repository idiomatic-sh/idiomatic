//! Builds the suite adapter from dependencies chosen by the composition.
use super::adapters::local::adapter::SuiteLocalAdapter;
/// Construct the local implementation of the suite contract with the case-receipt destination the edge chose.
#[must_use]
pub fn local(case_receipts: super::types::CaseReceipts) -> impl super::interface::SuitePort {
    SuiteLocalAdapter::new(case_receipts)
}
