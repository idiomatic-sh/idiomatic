//! Native case receipt validation against the registered cases.
pub(crate) mod errors;
#[cfg(test)]
pub mod fixtures;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
