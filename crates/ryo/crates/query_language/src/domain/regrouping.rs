//! The `query_language` domain's `regrouping` concept: the per-file inputs of a join after the files joined with each other before the
//! files hand them over, so a file hands over only the rows that meet.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
