//! Bounded reuse of application-owned grammar results; no syntax identities or parser ASTs.
pub(crate) mod service;
pub(crate) mod types;
#[cfg(test)]
mod service_test;
