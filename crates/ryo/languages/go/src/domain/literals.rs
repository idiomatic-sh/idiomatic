//! The go domain's `literals` concept: the text a literal of a file stands for, which the library does not decode.

pub(crate) mod constants;

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
