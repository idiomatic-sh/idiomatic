//! Source coordinate indexing independent of syntax and relation cells.
/// Cached byte starts for one source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SourceLineIndex(pub Vec<SourceLineOffset>);
/// A zero-based byte offset in source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceLineOffset(pub u32);
/// A one-based source line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceLineNumber(pub i64);
/// A zero-based UTF-8 byte column within a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SourceByteColumn(pub i64);
/// Source bytes supplied without syntax dependencies.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceLineBytes<'a>(pub &'a [u8]);
