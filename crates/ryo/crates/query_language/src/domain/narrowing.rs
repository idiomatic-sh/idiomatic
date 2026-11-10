//! The `query_language` domain's `narrowing` concept: the rows of a per-file input of a join after the files narrowed in each file to
//! those whose key a value of the other input can be, before the files hand them over.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
