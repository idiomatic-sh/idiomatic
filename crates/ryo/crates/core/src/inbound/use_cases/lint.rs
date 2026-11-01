//! The `core` domain's `lint` use case.

pub mod constants;
pub mod errors;
pub mod interface;
mod observing;
mod service;
pub mod types;
pub mod use_case;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod use_case_test;
