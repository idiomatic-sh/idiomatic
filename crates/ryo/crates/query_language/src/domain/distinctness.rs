//! The `query_language` domain's `distinctness` concept: whether the rows of a step hold each value of a set of its columns once, so a
//! join keyed on them meets one row of that step at most.

pub(crate) mod service;
pub(crate) mod types;

