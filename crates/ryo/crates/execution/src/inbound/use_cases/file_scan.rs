//! Runs a request's files with the new engine: its rule sets loaded once, each file scanned in the worker that took it.
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod interface;
pub mod types;
pub mod use_case;

#[cfg(test)]
mod use_case_test;
