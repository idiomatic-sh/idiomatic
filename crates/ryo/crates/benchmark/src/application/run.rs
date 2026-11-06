//! Execute exactly the invocation prepared by the selected linter.
pub mod service;
#[cfg(test)]
mod service_test;

mod constants;

/// Complete pinned inventory verification, independent of provider policy.
pub mod inventory;
pub(crate) mod inventory_execution;

mod case_execution;

mod case_observations;

/// Offline replay of exactly the same native receipt checks.
pub mod receipt_validation;

#[cfg(test)]
mod process_discrepancy_test;

mod input_metadata;

pub(crate) mod registered_context;

pub(crate) mod publication;
