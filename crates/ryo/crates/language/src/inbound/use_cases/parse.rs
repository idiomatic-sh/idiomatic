//! The `language` domain's `parse` use case.

pub mod constants;
pub mod errors;
pub mod interface;
pub mod service;
pub mod types;
pub mod use_case;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
#[cfg(test)]
mod use_case_test;
