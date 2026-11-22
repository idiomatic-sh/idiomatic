//! The `query_language` domain's `assembling` concept: a validated program lowered to a plan, by calling the concepts that lower each kind of step.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
