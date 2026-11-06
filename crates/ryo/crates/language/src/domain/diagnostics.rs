//! The `language` domain's `diagnostics` concept: what a parser reported about a file, with stable metadata and source ranges.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
