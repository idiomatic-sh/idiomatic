//! The types of the `file_view` concept: the CSS language as the engine's walk and its fillers name it, and the view of one walked
//! file the fillers read.

use ryo::languages::{FileId, NodeTable};

/// The CSS language as the engine walks it, over the ports `P` its walk calls. Its fillers call no port, so a `fills!` item names
/// the language with none (`CssWalk`), and the same fillers serve the walk an edge composes over its ports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CssWalk<P = ()> {
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
}

/// One walked file as the fillers read it: its syntax in place.
#[derive(Debug)]
pub(crate) struct CssFile<'f> {
    /// The node table and the text.
    pub(crate) syntax: Syntax<'f>,
}

/// The buffers a worker keeps from file to file: CSS keeps none.
#[derive(Debug, Default)]
pub(crate) struct CssScratch;
