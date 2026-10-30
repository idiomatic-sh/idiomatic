//! The `core` domain's `storing` concept: what a run keeps in its store for the next, what it can use again of what the last kept,
//! and when it must read every file again.

pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
