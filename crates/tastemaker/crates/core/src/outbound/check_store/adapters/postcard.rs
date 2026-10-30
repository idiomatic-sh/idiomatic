//! The `check_store` port, implemented with postcard.

pub(crate) mod adapter;
mod constants;
mod schemas;

#[cfg(test)]
mod adapter_test;
