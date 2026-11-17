//! The `query_language` domain's `rerouting` concept: an operation that reads one step made to read another in its place, each
//! column it names there renamed.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
