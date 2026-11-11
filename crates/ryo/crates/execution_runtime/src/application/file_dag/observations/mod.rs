//! Cross-file publication leases; evaluation belongs to the executable program.
pub(crate) mod constants;
pub(crate) mod cross;
pub(crate) mod restore;
pub mod types;
#[cfg(test)]
mod service_test;
