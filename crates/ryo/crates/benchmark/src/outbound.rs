//! Shared contracts, implemented at the benchmark composition boundary.
pub mod discovery;
pub mod engine;
pub mod evidence;
pub mod execution;
pub mod host;
pub mod linter;
pub mod process_meter;
pub mod profile_tools;
pub mod receipts;
pub mod scheduler;
pub mod suite;
pub mod worker;
pub mod workspace;

pub mod collection;

pub mod context;

pub mod executables;

/// External registration and inventory-result files.
pub mod inventory;
