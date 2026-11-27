//! The `reaching` concept: the files each rule entry of each scope reaches, every entry of a scope matched in one pass.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
