//! The `scan` use case.

pub mod errors;
pub mod interface;
pub mod use_case;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod use_case_test;
