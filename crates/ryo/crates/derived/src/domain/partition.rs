//! Typed partition order; clustered order is deliberately distinct from RawKeyOrder.
pub(crate) mod service;

#[cfg(test)]
mod service_test;

pub mod types;
