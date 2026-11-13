//! Cargo builds and seals the executables; the search path finds the reference tool.
pub(crate) mod adapter;
#[cfg(test)]
mod adapter_test;

mod constants;

mod schemas;

pub(crate) mod settings;
