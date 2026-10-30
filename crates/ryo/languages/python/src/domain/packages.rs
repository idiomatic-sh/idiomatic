//! The python domain's `packages` concept: the packages the manifests of a repository declare, and how they hold and need one another.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
