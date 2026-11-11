//! The `contract` service: pure functions over the domain's types, with no IO and no ports.
//! Logical mutable-set requirements are separate from acquisition and storage.
pub mod service;
#[cfg(test)]
mod service_test;
