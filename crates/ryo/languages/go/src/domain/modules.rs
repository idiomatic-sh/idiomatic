//! The go domain's `modules` concept: the modules the manifests of a repository declare, which require which, which a workspace holds,
//! and the module each file belongs to.

pub(crate) mod constants;
pub(crate) mod service;
pub(crate) mod types;

#[cfg(test)]
mod service_test;
