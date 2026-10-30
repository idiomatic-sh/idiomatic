//! The rust domain's `tokens` concept: the tokens of its trees, named as rust-analyzer's parser names its kinds.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
