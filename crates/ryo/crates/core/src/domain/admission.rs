//! The `core` domain's `admission` concept: what typing does not prove of the output a language wrote for one file, so that a language's
//! bug is refused where it is found and never reaches the store.

pub(crate) mod errors;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
