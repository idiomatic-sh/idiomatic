//! Matching contract with opaque caller-owned compiled values.
pub mod compiled;
pub mod errors;
pub mod interface;
pub mod types;
pub mod use_case;

#[cfg(test)]
mod use_case_test;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
