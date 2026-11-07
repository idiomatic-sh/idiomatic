//! The `query_language` domain's `selecting` concept: a `select` lowered to the join, the filter and the projection the query wrote.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
