//! The `query_language` domain's `banding` concept: a join's `where` split into its band, the order comparisons between its
//! sides, and the conditions the filter after the join keeps.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
