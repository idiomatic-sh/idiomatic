//! The `reports` concept: what one `check` leaves for `up`, `down` and the next `check`: each tool's run under the
//! stamp of its inputs, and the digest of every file it fingerprinted.

pub mod constants;
pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
