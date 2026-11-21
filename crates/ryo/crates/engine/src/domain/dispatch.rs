//! The `dispatch` concept: a file's rules, each branch run when its need holds.

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod service;

#[cfg(test)]
mod service_test;
