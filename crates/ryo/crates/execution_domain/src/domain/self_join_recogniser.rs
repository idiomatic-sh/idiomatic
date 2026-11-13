//! Pure recognition of a dominated-key subtree; execution and admission remain application work.
pub(crate) mod service;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
