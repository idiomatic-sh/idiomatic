//! Builds the facts adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The facts adapter that keeps everything in memory: a worker builds its columns from each relation of the catalog.
#[must_use]
pub fn memory() -> impl crate::outbound::facts::interface::FactsPort {
    super::adapters::memory::adapter::FactsMemoryAdapter::new()
}

/// The facts adapter that keeps the words the files leave for the cross-file finish in a file of `directory`, not in memory.
#[must_use]
pub fn spill(directory: std::path::PathBuf) -> impl crate::outbound::facts::interface::FactsPort {
    super::adapters::spill::adapter::FactsSpillAdapter::new(directory)
}

/// The facts adapter that keeps the words the files leave for the cross-file finish in a file of the system's temporary directory,
/// for a run that keeps nothing: the file is removed from it as soon as it is open.
#[must_use]
pub fn spill_scratch() -> impl crate::outbound::facts::interface::FactsPort {
    super::adapters::spill::adapter::FactsSpillAdapter::in_scratch()
}
