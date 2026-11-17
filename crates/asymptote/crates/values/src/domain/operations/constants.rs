//! Where the lock lives and when it is compacted.

/// The directory of a product's cache, beneath its root: the stores its use cases keep, which ignore themselves in
/// git.
pub const CACHE: &str = ".asymptote/cache";
/// How many operations the lock may hold before `up` or `down` replaces them with a snapshot: compaction bounds the
/// lock's files and the history a rename projection reads, since a snapshot becomes every record's origin.
pub const COMPACT_AT: usize = 64;
/// The directory segment that holds a lock: no tool reads it, so it is no input whose change runs a tool again.
pub const LOCK_SEGMENT: &str = ".asymptote";
