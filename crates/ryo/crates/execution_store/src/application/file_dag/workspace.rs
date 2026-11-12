//! The application owns each invocation's mutable allocations and release boundary.
pub mod interface;
pub mod service;
#[cfg(test)]
mod service_test;
pub mod types;
