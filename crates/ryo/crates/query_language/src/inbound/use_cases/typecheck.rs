//! The `query_language` domain's `typecheck` use case.

pub mod errors;
pub mod interface;
pub mod service;
pub mod use_case;

#[cfg(test)]
mod use_case_test;
