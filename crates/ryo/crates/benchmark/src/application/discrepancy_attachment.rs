//! Convert boundary-owned wire data into canonical case expectations before verification.
pub(crate) mod errors;
#[cfg(test)]
mod fixtures;
pub mod service;
#[cfg(test)]
mod service_test;
