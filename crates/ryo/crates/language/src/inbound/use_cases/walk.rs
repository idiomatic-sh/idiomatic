//! The `language` domain's `walk` use case: the contract a language implements for the engine, beside `parse` until the swap.

pub mod errors;
pub mod interface;
pub mod types;
pub mod use_case;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod use_case_test;
