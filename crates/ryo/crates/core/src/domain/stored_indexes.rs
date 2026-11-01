//! Immutable stored-key indexes shared only within one audited physical file read.
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
