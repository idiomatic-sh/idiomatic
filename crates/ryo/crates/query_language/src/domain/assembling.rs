//! The `query_language` domain's `assembling` concept: a validated program lowered to a plan, by calling the concepts that lower each kind of step.

mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
