//! The `language` domain's `shapes` concept: a file's expression-shape index, built once per file bottom-up over the
//! [`keys`](super::keys) interner. A language names only the words of one node's key; the walk, the index and its state are
//! this concept's.

pub mod service;
pub mod types;

#[cfg(test)]
mod service_test;
