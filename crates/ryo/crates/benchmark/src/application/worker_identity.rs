//! Check the identity a worker request carries against the identity its executable expects.
pub(crate) mod constants;
pub mod service;

#[cfg(test)]
mod service_test;
