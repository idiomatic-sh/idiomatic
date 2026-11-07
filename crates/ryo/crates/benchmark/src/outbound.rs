//! Shared contracts, implemented at the benchmark composition boundary.
pub mod discovery;
pub mod engine;
pub mod evidence;
pub mod execution;
pub mod linter;
pub mod receipts;
pub mod suite;
pub mod worker;
pub mod workspace;

pub mod collection;

pub mod context;

/// External registration and inventory-result files.
pub mod inventory;
