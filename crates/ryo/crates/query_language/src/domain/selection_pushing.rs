//! The `query_language` domain's `selection_pushing` concept: the filters of a union or a join made on its inputs, so it combines only
//! the rows they let through.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
