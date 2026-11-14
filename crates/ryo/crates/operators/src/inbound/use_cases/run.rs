//! The `run` use case: one node of a physical plan made for the open file.
pub mod interface;
pub mod use_case;
#[cfg(test)]
mod use_case_test;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
