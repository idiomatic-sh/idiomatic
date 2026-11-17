//! The `query_language` domain's `fanning` concept: what each join's keys are proven to pair, stamped on the plan before it
//! crosses to the operators.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
