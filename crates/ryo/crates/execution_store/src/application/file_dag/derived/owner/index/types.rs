//! Byte and row bounds of index admission.

/// The bytes a file's retained derived payloads may occupy in all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct ByteLimit(pub usize);
