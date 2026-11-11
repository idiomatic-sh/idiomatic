//! Request-only interning. Runtime acquisition compares prepared IDs, never request metadata.
pub(super) mod identity;
pub mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod types;
