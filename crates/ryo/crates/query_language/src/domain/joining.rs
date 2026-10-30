//! The `query_language` domain's `joining` concept: the join a `select` writes, and the columns of either side its `where`
//! names.

pub(crate) mod constants;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
