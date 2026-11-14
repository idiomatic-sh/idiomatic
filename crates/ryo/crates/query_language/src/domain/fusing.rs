//! The `query_language` domain's `fusing` concept: the filters and lookups of a plan that are read where they are found, instead of being made into tables first.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
