//! The python domain's `scopes` concept: the scope each element is evaluated in, and where a name is bound once `global` and
//! `nonlocal` have been read.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
