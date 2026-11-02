//! Execute exactly the invocation prepared by the selected linter.
pub mod service;
#[cfg(test)]
mod service_test;

mod constants;

mod discrepancy;

/// Complete pinned inventory verification, independent of provider policy.
pub mod inventory;
mod inventory_execution;
mod inventory_receipts;
