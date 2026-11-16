//! The `query_language` domain's `widths` concept: how many columns the rows of a relation or a step have, from what the step
//! makes of the rows it reads.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
