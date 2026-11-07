//! The go domain's `signatures` concept: the parameters, the receiver and the named results of a function, by where they are in its
//! declaration.

pub(crate) mod constants;

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
