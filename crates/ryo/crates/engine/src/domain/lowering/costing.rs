//! The `costing` part of the lowering: the cost model with its fixed priors. It orders bounded accesses; it never bounds one.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
