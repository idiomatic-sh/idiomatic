//! The `core` domain's `keeping` concept: what a run keeps of a repository for the next, and the digests that say whether it can
//! be used.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
