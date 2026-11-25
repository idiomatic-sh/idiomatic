//! The types of the `file_view` concept: the language as the engine's walk names it, and the view of one walked file the fillers read.

use ryo::languages::{ByteSlice, FileId, NodeTable};

/// The html language as the engine walks it, over the ports `P` its walk calls. Its fillers call no port, so a `fills!` item names
/// the language with none (`HtmlWalk`), and the same fillers serve the walk an edge composes over its ports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HtmlWalk<P = ()> {
    /// The ports the walk calls.
    pub(crate) ports: P,
}

/// One walked file as the fillers read it: its numbered node table and its text.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HtmlFile<'f> {
    /// The file's node table, finished.
    pub(crate) table: &'f NodeTable,
    /// The file's bytes.
    pub(crate) source: ByteSlice<'f>,
    /// The file's number.
    pub(crate) file: FileId,
}
