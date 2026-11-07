//! The `core` domain's `memory_diagnostics` concept: the opt-in lifecycle memory diagnostics, written at the phases of a run.

pub(crate) mod constants;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
