//! Measure every project of a certified corpus in both orders, cold, retained and edited.
mod constants;
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod interface;
mod service;
pub mod types;
pub mod use_case;

#[cfg(test)]
mod use_case_test;
