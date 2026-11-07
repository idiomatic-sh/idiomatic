//! The digest port, implemented with BLAKE3.

pub(crate) mod adapter;
mod constants;

#[cfg(test)]
mod adapter_test;
