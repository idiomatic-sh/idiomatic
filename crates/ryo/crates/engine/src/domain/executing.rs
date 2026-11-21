//! The `executing` concept: the one loop over a program's words.

pub(crate) mod bodies;
pub(crate) mod budget;
pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
