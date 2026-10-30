//! The types of the `admission` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_language::domain::relations::types::FileId;

/// How many bytes the file has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ByteLength(pub u64);

/// How many places the file's table has, the elements the language added for reading included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementTotal(pub u64);

/// What a file's output must lie within.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Limits {
    /// The number the run gave the file: the file every cell of a file or a node names.
    pub file: FileId,
    /// The elements of its table: the places a node can be at.
    pub elements: ElementTotal,
    /// How many things the parser reported about it: the ids of reports are numbered below this, in a space of their own above the places of
    /// the table.
    pub diagnostics: ElementTotal,
    /// Its length in bytes: where a span can end.
    pub length: ByteLength,
}

/// The numbers one column of one relation holds, as they are stored.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StoredNumbers<'a>(pub &'a [u64]);

/// Whether the span of a cell lies within the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpanContainment {
    /// It starts no later than it ends and ends no later than the file.
    Within,
    /// It does not.
    Outside,
}
