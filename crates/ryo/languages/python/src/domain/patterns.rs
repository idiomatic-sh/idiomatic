//! The python domain's `patterns` concept: whether a directory is one a workspace's member or exclude glob names.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
