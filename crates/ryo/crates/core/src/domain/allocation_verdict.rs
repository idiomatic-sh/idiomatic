//! The `core` domain's `allocation_verdict` concept: how many blocks the files of a lint allocated beyond what the parser library does on its own, and whether that is no more than the findings explain.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
