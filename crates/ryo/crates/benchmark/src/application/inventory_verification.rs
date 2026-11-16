//! Verify a complete pinned inventory.
pub mod authority;
pub(crate) mod constants;
#[cfg(test)]
mod fixtures;
mod rule;
pub mod service;
pub(crate) mod types;
#[cfg(test)]
mod service_test;
