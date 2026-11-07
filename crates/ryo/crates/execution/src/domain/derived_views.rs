//! Semantic derived-view algorithms; application owns acquisition and every retained lifetime.
pub(crate) mod types;

pub(crate) mod service;

#[cfg(test)]
pub(crate) mod fixtures;

#[cfg(test)]
mod service_test;
