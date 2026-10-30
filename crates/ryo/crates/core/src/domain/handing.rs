//! The `core` domain's `handing` concept: what a file handed the steps that read across files, as the store keeps it in its tables
//! and gives it back.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
