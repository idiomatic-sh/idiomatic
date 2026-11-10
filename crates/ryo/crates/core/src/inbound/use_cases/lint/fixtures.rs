//! The test support of the `lint` use case: what the whole per-file path must pass, once, for
//! every product's tests to call.

pub mod contract;
pub mod corpus;
pub mod engine;
#[cfg(test)]
pub(crate) mod given;
#[cfg(test)]
pub(crate) mod lifecycle;
#[cfg(test)]
pub(crate) mod lookups;
pub mod measures;
pub mod operators;
pub mod runs;
#[cfg(test)]
pub(crate) mod scenario;
#[cfg(test)]
pub(crate) mod skips;
#[cfg(test)]
pub(crate) mod stores;
#[cfg(test)]
pub(crate) mod worlds;
