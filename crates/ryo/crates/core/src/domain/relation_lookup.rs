//! The `core` domain's `relation_lookup` concept: the relation ryo owns that a name written in a query stands for.

pub(crate) mod errors;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
