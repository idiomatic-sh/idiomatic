//! The `core` domain's `governance` concept: which files each placed rule governs, so that its answers about other files are
//! not its findings and a rule that governs none of the files read is idle.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
