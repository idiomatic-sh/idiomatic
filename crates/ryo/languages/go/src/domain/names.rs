//! The go domain's `names` concept: the names a row holds, made from the tokens of the file; what cannot be resolved statically is the
//! unknown value of its reason.

pub(crate) mod constants;

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
