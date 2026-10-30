//! The store port, implemented as no store: a run that keeps nothing for the next.

pub(crate) mod adapter;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
