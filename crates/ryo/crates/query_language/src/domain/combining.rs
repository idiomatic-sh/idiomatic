//! The `query_language` domain's `combining` concept: the set calls (`union`, `intersection` and `difference`) of two inputs with the same columns, lowered to steps.

pub(crate) mod service;

#[cfg(test)]
mod service_test;
