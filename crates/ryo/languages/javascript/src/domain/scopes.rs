//! The `scopes` concept: the lexical environments of a JavaScript file and the owner of each occurrence.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
