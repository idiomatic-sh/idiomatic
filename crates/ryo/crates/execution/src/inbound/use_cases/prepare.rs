//! Prepares a request's program for execution.
pub mod interface;
pub mod types;
pub mod use_case;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod use_case_test;
