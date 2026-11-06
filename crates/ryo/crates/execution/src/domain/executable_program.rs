//! The sole immutable executable form. Original plans are preparation inputs only.
pub(crate) mod kernel;
pub(crate) mod service;
pub(crate) mod types;

mod observed;

pub(crate) mod physical;

pub(crate) mod column;

pub(crate) mod fusion;

pub(crate) mod selection;

pub(crate) mod native;

#[cfg(test)]
mod service_test;

pub(crate) mod file_demand;

#[cfg(test)]
pub(crate) mod observations;
