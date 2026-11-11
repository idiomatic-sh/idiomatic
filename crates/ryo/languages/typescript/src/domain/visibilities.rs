//! The typescript domain's `visibilities` concept: the visibility a declaration is written with, `export` or an accessibility.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
