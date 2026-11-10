//! The `core` domain's `repository` concept: the files of a run as it finds them, and what an earlier run kept of them.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
