//! Text-glob semantics, including compilation and exact interpreter fallback.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
