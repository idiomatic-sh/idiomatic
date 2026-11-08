//! The sole immutable executable form. Original plans are preparation inputs only.
pub(crate) mod kernel;
pub(crate) mod service;
pub(crate) mod types;

mod aliases;
mod merge;
mod observed;
mod origins;

pub(crate) mod physical;

pub(crate) mod fusion;

#[cfg(test)]
mod service_test;

pub(crate) mod file_demand;

