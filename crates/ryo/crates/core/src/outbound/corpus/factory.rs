//! Builds the corpus adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The ignore corpus adapter: the files on disk below `root`, walked (those `.gitignore` does not exclude) or as `listing` names
/// them, and the files of `beside` read as given in place of the disk's. A walk does not go into a directory named in `skipped`.
#[must_use]
pub fn ignore(
    root: ryo_language::inbound::use_cases::project::types::RootPath,
    listing: ryo_values::domain::files::types::CorpusListing,
    beside: Vec<ryo_values::domain::files::types::CorpusFile>,
    skipped: super::types::SkippedDirectories,
) -> impl crate::outbound::corpus::interface::CorpusPort {
    super::adapters::ignore::adapter::CorpusIgnoreAdapter::new(root, listing, beside, skipped)
}

/// The memory corpus adapter: `files`, read instead of the disk.
#[must_use]
pub fn memory(
    files: Vec<ryo_values::domain::files::types::CorpusFile>,
    inventory: ryo_language::domain::project_context::types::GivenProjectInventory,
) -> impl crate::outbound::corpus::interface::CorpusPort {
    super::adapters::memory::adapter::CorpusMemoryAdapter::new(files, inventory)
}
