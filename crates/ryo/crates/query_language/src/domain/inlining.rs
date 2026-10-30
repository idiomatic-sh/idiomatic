//! The `query_language` domain's `inlining` concept: the views of a relation read in place, and the rows handed over cut to the columns asked.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
