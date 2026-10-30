//! The `core` domain's outbound ports: what it needs from outside, one module each.

pub mod corpus;
pub mod digest;
pub mod facts;
pub mod language;
pub mod lint_store;
pub mod matching;
pub mod memory;
pub mod query_language;
pub mod rows;
pub mod scheduler;
