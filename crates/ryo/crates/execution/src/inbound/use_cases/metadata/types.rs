//! Data returned by pure metadata decisions; none carries execution authority.

/// The precise partial source set and the declared number of files it may name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AffectedFiles<'a> {
    pub affected: &'a [Option<ryo_relations::domain::relations::types::FileId>],
    pub file_count: CoverageFileCount,
}

/// Number of declared files whose partial sources may be named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverageFileCount(pub usize);

/// Distinct validated file ordinals affected by one partial source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileOrdinals(pub Vec<usize>);
