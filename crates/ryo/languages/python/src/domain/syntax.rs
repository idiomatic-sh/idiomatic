//! The python domain's `syntax` concept: how a reader moves about a file's tree.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
