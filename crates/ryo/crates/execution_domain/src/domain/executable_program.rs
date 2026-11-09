//! The sole immutable executable form. Original plans are preparation inputs only.
pub mod kernel;
pub mod service;
pub mod types;

mod aliases;
mod merge;
mod observed;
mod origins;

pub mod physical;

pub mod fusion;

#[cfg(test)]
mod service_test;

pub mod file_demand;

pub mod presence;

