//! The phases and owners the lifecycle memory diagnostics name, and their tracing target.
use super::types::{MemoryOwner, MemoryPhase};

/// The text "store_staged_descriptors".
pub(super) const STORE_STAGED_DESCRIPTORS: MemoryOwner = MemoryOwner("store_staged_descriptors");

/// The text "store_handed".
pub(crate) const STORE_HANDED: MemoryOwner = MemoryOwner("store_handed");

/// The text "worker_notes".
pub(super) const WORKER_NOTES: MemoryOwner = MemoryOwner("worker_notes");

/// The text "cross_symbols".
pub(super) const CROSS_SYMBOLS: MemoryOwner = MemoryOwner("cross_symbols");

/// The text "worker_arenas".
pub(super) const WORKER_ARENAS: MemoryOwner = MemoryOwner("worker_arenas");

/// The text "fresh_records".
pub(super) const FRESH_RECORDS: MemoryOwner = MemoryOwner("fresh_records");

/// The text "after_files".
pub(crate) const AFTER_FILES: MemoryPhase = MemoryPhase("after_files");

/// The text "after_store_write".
pub(crate) const AFTER_STORE_WRITE: MemoryPhase = MemoryPhase("after_store_write");

/// The text "before_store_write".
pub(crate) const BEFORE_STORE_WRITE: MemoryPhase = MemoryPhase("before_store_write");

/// The text "after_store_build".
pub(crate) const AFTER_STORE_BUILD: MemoryPhase = MemoryPhase("after_store_build");

/// The text "before_store_build".
pub(crate) const BEFORE_STORE_BUILD: MemoryPhase = MemoryPhase("before_store_build");

/// The text "before_cross_cleanup".
pub(crate) const BEFORE_CROSS_CLEANUP: MemoryPhase = MemoryPhase("before_cross_cleanup");

/// The tracing target of the lifecycle memory diagnostics.
pub(crate) const LIFECYCLE_MEMORY_DIAGNOSTIC: &str =
    ryo_values::domain::table_words::constants::LIFECYCLE_TARGET;
