//! Portable publication data without exposing execution authority.
pub mod errors;
#[cfg(test)]
mod fixtures;
pub mod interface;
pub mod types;
pub mod use_case;
#[cfg(test)]
mod use_case_test;
