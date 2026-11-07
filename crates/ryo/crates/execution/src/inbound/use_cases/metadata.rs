//! Read-only metadata and preflight decisions for an execution request.
pub mod interface;
pub mod types;
pub mod use_case;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
#[cfg(test)]
mod use_case_test;
