//! The types of the `file_view` concept: the go language as the engine's walk and its fillers name it, the view of one walked file the
//! fillers read, and the buffers a worker keeps from file to file.

use ryo::languages::{ByteSlice, FileId, NodeTable};

/// The go language as the engine walks it, over the ports `P` its walk calls. Its fillers call no port, so a `fills!` item names the
/// language with none (`GoWalk`), and the same fillers serve the walk an edge composes over its ports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GoWalk<P = ()> {
    /// The ports the walk calls.
    pub(crate) ports: P,
}

/// A walked file's syntax: its node table in place, and its bytes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Syntax<'f> {
    /// The node table, finished.
    pub(crate) table: &'f NodeTable,
    /// The bytes of the file.
    pub(crate) source: ByteSlice<'f>,
    /// The number the run gave the file.
    pub(crate) file: FileId,
}

/// One walked file as the fillers read it: its syntax in place.
#[derive(Debug)]
pub(crate) struct GoFile<'f> {
    /// The node table and the text.
    pub(crate) syntax: Syntax<'f>,
    /// Whether the file is a module's `go.mod`, which has no imports.
    pub(crate) module: bool,
}

/// The names of a walked file: a text becomes a word of the file, the cell a row holds. Shared, so a lazy answer that holds the file's
/// names does not stop another from naming a text.
pub(crate) struct Words<'w>(pub(crate) std::cell::RefCell<&'w mut ryo::languages::FileWords>);
