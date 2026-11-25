//! The types of the `file_view` concept: the Markdown language as the engine's walk and its fillers name it, the view of one walked
//! file the fillers read, and the buffers a worker keeps from file to file.

use crate::domain::model::types::Sections;
use ryo::languages::{FileId, NodeTable};

/// The Markdown language as the engine walks it, over the ports `P` its walk calls. Its fillers call no port, so a `fills!` item names
/// the language with none (`MarkdownWalk`), and the same fillers serve the walk an edge composes over its ports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MarkdownWalk<P = ()> {
    /// The ports the walk calls.
    pub(crate) ports: P,
}

/// A walked file's syntax: its node table in place, and its text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Syntax<'f> {
    /// The node table.
    pub(crate) table: &'f NodeTable,
    /// The bytes of the file.
    pub(crate) text: &'f [u8],
    /// The number the run gave the file.
    pub(crate) file: FileId,
    /// What the library decoded of the file's tokens.
    pub(crate) decodings: &'f Decodings,
}

/// One walked file as the fillers read it: its syntax in place, and its sections, worked out by the first fill that needs them.
#[derive(Debug)]
pub(crate) struct MarkdownFile<'f> {
    /// The node table and the text.
    pub(crate) syntax: Syntax<'f>,
    /// The sections, lent from the worker's buffers.
    pub(crate) sections: &'f mut Sections,
    /// The text a filler builds for one row, lent from the worker's buffers.
    pub(crate) text: &'f mut String,
    /// Whether the sections have been worked out for this file.
    pub(crate) modelled: Modelled,
}

/// Whether a file's sections have been worked out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Modelled {
    /// They have.
    Done,
    /// They have not.
    Waiting,
}

/// The buffers a worker keeps from file to file: refilled for each file, never reallocated.
#[derive(Debug, Default)]
pub(crate) struct MarkdownScratch {
    /// What the library decoded of the tokens of the file in hand.
    pub(crate) decodings: Decodings,
    /// The sections.
    pub(crate) sections: Sections,
    /// The text a filler builds for one row.
    pub(crate) text: String,
}

/// What the library decoded of the tokens of the file in hand, by where each token is. It is kept beside the node table, which holds
/// every token as it is written, so the text the engine reads of a token is what the file says and what the library decoded is
/// what a filler asks for.
#[derive(Debug, Default)]
pub(crate) struct Decodings {
    /// Where each decoded token's text is in `text`, by the token's first and last byte.
    pub(crate) places: std::collections::HashMap<(u32, u32), (usize, usize)>,
    /// The decoded texts, one after another.
    pub(crate) text: String,
}
