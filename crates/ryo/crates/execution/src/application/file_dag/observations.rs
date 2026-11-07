//! Cross-file publication leases; evaluation belongs to the executable program.
pub(crate) mod constants;
pub(crate) mod cross;
pub(crate) mod restore;
pub(crate) mod types;
#[cfg(test)]
mod union_test;
