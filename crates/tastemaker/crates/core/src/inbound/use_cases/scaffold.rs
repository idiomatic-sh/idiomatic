//! The core's `scaffold` use case.

pub mod errors;
pub mod interface;
pub mod service;
pub mod types;
pub mod use_case;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod use_case_test;
