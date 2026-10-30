//! The `clock` port: today, the day `up` records a violation as accepted.

pub(crate) mod adapters;
pub mod factory;
pub mod interface;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
