//! The `query_language` domain's `lineage` concept: what a column of a step's rows is, found from the relation column it was read from.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
