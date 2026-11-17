//! The `instance_params` concept: the value of each param of a spec at one of its instances.

pub mod service;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;

#[cfg(test)]
mod service_test;
