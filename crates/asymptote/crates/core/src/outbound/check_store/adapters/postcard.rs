//! The `check_store` port, implemented with postcard.

pub(crate) mod adapter;
mod constants;
pub(crate) mod errors;
mod schemas;
pub(crate) mod settings;

#[cfg(test)]
mod adapter_test;
