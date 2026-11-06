//! Byte and row bounds of index admission.

/// The bytes a file's retained derived payloads may occupy in all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::derived::owner) struct ByteLimit(pub usize);

/// A lower bound on the bytes one index occupies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::derived::owner) struct IndexBytes(pub usize);

/// The rows an index would cover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::derived::owner) struct IndexRowCount(pub usize);

/// The key columns an index would hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::derived::owner) struct KeyColumns(pub usize);
