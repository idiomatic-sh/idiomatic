//! The `spelled` part of the `needs` concept: which of the texts the needs ask for a file spells as a token's exact text,
//! found by one pass over the file's tokens a file.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
