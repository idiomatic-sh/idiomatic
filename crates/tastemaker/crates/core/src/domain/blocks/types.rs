//! The types of the `blocks` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::comments::types::ByteSpan;
use crate::domain::contracts::types::{DataRow, WrittenValue};

/// A section of a document and where its title is written.
#[derive(Clone, Copy, Debug)]
pub struct Titled<'r> {
    /// The section's row of `defines`.
    pub row: &'r DataRow,
    /// Where its title is.
    pub span: ByteSpan,
}

/// One block of a document where it is written: a section, a run of prose, or a literal with its value.
#[derive(Clone, Copy, Debug)]
pub struct Placed<'r> {
    /// Where it is.
    pub span: ByteSpan,
    /// What it is.
    pub block: Block<'r>,
}

/// What a block of a document is.
#[derive(Clone, Copy, Debug)]
pub enum Block<'r> {
    /// A section, by its row of `defines`.
    Section(&'r DataRow),
    /// A run of prose.
    Run,
    /// A literal of a block of its own, by its value.
    Literal(&'r WrittenValue),
}

/// How many sections hold a section, its own left out, and one: 1 for a top section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionDepth(pub usize);
