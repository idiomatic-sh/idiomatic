//! The `check_store` port, implemented with postcard.

pub(crate) mod adapter;
mod constants;
mod schemas;
pub(crate) mod settings;

#[cfg(test)]
mod adapter_test;
