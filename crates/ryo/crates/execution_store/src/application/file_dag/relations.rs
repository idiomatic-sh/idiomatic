//! One file generation owns representation descriptors and lazy admission proofs.
mod accounting;
pub mod admission;
mod external;
pub mod generation;
pub mod handle;
mod identity;
pub(crate) mod inspection;
pub mod interface;
pub mod number;
pub mod packed;
pub(crate) mod producer;
pub mod publication;
pub mod service;
#[cfg(test)]
mod service_test;
pub(crate) mod source;
pub mod types;
