//! The plan's one growth check: each live node's bound composed from its declaration and its inputs' bounds, and a node whose
//! rows grow past its inputs' refused.
pub(crate) mod errors;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
