//! The core's outbound ports: what it needs from outside, one module each.

pub mod analysis;
pub mod check_store;
pub mod digest;
pub mod manifest;
pub mod matching;
pub mod packages;
pub mod repository;
pub mod scheduler;
pub mod templates;
pub mod validate_store;
