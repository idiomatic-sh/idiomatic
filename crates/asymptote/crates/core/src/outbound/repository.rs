//! The `repository` port: the history questions the ratchet asks of git: where the working tree stands, where a
//! branch started, what the lock held there, what was renamed since, and which files a tool reads.

pub(crate) mod adapters;
pub mod errors;
pub mod factory;
pub mod interface;
pub mod types;

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
