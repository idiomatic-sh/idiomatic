//! What each built derived payload occupies, as the ledger's backings are told it; the engine alone assigns lifetime classes
//! and budgets.
pub(crate) mod service;

#[cfg(test)]
mod service_test;

pub(crate) mod types;
