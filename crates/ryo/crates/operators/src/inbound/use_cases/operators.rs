//! Portable primitive semantics available to the engine's adapter.
pub mod interface;
pub mod types;
pub mod use_case;
#[cfg(test)]
mod use_case_test;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
