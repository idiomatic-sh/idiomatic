//! The `core` domain's `rules` concept: the rules of a run as the domain reads them, and what is decided of them before any file is.

mod constants;
pub(crate) mod errors;
pub(crate) mod service;

#[cfg(test)]
mod service_test;
