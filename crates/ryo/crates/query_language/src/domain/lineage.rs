//! The `query_language` domain's `lineage` concept: what a column of a step's rows is, found from the relation column it was read from,
//! the per-file steps its values come from, and the plan's side of `ryo_values`' lineage, which alone says when a column states the
//! file of its row.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
