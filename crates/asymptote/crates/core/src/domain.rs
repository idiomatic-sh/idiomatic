//! The core's rules, one module per concept: pure functions over its values, with no IO and no ports.

pub mod captures;
pub mod git;
pub mod hooks;
pub mod inputs;
pub mod judgement;
pub mod manifest;
pub mod operations;
pub mod perf;
pub mod ratchet;
pub mod renames;
pub mod reports;
pub mod seeding;
pub mod selection;
pub mod violations;
