//! The core's outbound ports: what it needs from outside, one module each.

pub mod bmf;
pub mod check_store;
pub mod clock;
pub mod digest;
pub mod hooks;
pub mod lock;
pub mod manifest;
pub mod matching;
pub mod repository;
pub mod sarif;
pub mod sources;
pub mod tools;
