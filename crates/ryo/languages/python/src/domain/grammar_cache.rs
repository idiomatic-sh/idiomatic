//! Bounded reuse of application-owned grammar results; no syntax identities or parser ASTs.
pub(crate) mod constants;
pub(crate) mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
