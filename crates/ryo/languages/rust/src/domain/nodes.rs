//! The rust domain's `nodes` concept: the nodes of its trees, named as rust-analyzer's parser names its kinds.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
