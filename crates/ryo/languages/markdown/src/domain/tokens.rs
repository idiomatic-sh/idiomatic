//! The markdown domain's `tokens` concept: the tokens of its trees, named as mdast names its terminal rules.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
