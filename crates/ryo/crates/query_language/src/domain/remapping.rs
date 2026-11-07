//! The `query_language` domain's `remapping` concept: the rewriting of a fragment step's identities, inputs and parameters at a call site.

mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
