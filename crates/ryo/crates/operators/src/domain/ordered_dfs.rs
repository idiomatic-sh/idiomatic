//! Portable ordered depth-first semantics: the edges visited from each distinct root, with their ordinals.
pub(crate) mod service;
pub mod types;

#[cfg(test)]
mod service_test;
