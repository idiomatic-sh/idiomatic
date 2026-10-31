//! Resolve lexical import and builtin provenance without lint policy or runtime execution.
pub(crate) mod constants;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
