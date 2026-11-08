//! The `query_language` domain's `lineage` concept: what a column of a step's rows is, found from the relation column it was read from,
//! and the per-file steps its values come from.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
