//! The `core` domain's `corpus_copies` concept: the files of a corpus's inputs, once or many times over, under the paths a lint reads them by.

pub(crate) mod constants;
pub(crate) mod service;
pub mod types;

#[cfg(test)]
mod service_test;
