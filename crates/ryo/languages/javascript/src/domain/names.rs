//! The `names` concept: what a JavaScript declaration binds, which names are read where they occur, and what a class inherits.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
