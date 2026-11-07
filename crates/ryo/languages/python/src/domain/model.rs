//! The python domain's `model` concept: the `global` and `nonlocal` names of every scope of a file, worked out once for its readers.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
