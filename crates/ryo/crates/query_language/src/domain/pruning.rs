//! The `query_language` domain's `pruning` concept: the steps nothing reads, dropped, and the inputs walked.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
