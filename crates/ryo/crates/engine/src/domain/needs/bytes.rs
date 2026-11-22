//! The `bytes` part of the `needs` concept: which of the literals the needs ask the file's texts to hold the file's bytes
//! hold, found by one pass over the file's bytes a file (its byte set) and a search for each longer literal.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
