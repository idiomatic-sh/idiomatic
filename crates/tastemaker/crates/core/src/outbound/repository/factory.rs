//! Builds the repository adapters, one function each. A factory builds nothing else, chooses nothing, and is called only
//! by an edge.

/// The disk adapter: the file system, walked with `ignore`, never entering the directories `skipped` names nor the
/// store's directory `store`.
#[must_use]
pub fn disk(
    skipped: Vec<crate::domain::files::types::SkippedDirectory>,
    store: crate::domain::files::types::DirectoryName,
) -> impl crate::outbound::repository::interface::RepositoryPort {
    super::adapters::disk::adapter::RepositoryDiskAdapter { skipped, store }
}

/// The memory adapter: scratch directories held in memory, walked as the disk adapter walks a disk, never entering the
/// directories `skipped` names nor the store's directory `store`. Each scenario's project is written there.
#[must_use]
pub fn memory(
    skipped: Vec<crate::domain::files::types::SkippedDirectory>,
    store: crate::domain::files::types::DirectoryName,
) -> impl crate::outbound::repository::interface::RepositoryPort {
    super::adapters::memory::adapter::RepositoryMemoryAdapter {
        skipped,
        store,
        trees: std::sync::Arc::default(),
    }
}
