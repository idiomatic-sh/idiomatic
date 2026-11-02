//! Shared contracts, implemented at the benchmark composition boundary.
pub mod engine;
pub mod evidence;
pub mod execution;
pub mod linter;
pub mod suite;
pub mod worker;
pub mod workspace;

pub mod collection;

pub(crate) mod context;

/// External registration and inventory-result files.
pub mod inventory;
