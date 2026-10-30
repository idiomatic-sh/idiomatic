//! The `validate_store` port, implemented with postcard.

pub(crate) mod adapter;
mod constants;
mod schemas;
pub(crate) mod types;

#[cfg(test)]
mod adapter_test;
