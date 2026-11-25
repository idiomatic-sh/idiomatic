//! The types of the `file_view` concept: the go language as the engine's walk and its fillers name it, the view of one walked file the
//! fillers read, and the buffers a worker keeps from file to file.

use crate::domain::syntax::types::Roling;
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
#[cfg_attr(test, expect(dead_code, reason = "the fillers read the view's table, text and number as their relations are written"))]
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
#[cfg_attr(test, expect(dead_code, reason = "the fillers read the view as their relations are written"))]
pub(crate) struct GoFile<'f> {
    /// The node table and the text.
    pub(crate) syntax: Syntax<'f>,
}

/// The buffers a worker keeps from file to file: refilled for each file, never reallocated.
#[derive(Default)]
pub(crate) struct GoScratch {
    /// What gives each element of the walk its role.
    pub(crate) roling: Roling,
}
