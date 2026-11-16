//! The `query_language` domain's `iterating` concept: the call that runs a first-in-first-out dataflow fixpoint over a graph,
//! lowered to a step.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
