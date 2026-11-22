//! The `query_language` domain's `splicing` concept: the definitions of a linked program lowered into its plan, each
//! expansion of a fragment that binds no relation input lowered once over a catalog and spliced into every later plan with
//! the same expansion.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
