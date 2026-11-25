//! The types of the `file_view` concept: the Rust language as the engine's walk and its fillers name it, the view of one walked
//! file the fillers read, and the buffers a worker keeps from file to file.

use crate::domain::model::types::ReadIndexes;
use crate::outbound::syntax::types::Tape;
use ryo::languages::{ByteSlice, Elements, FileId, NodeTable, Symbols};

/// The Rust language as the engine walks it, over the ports `P` its walk calls. Its fillers call no port, so a `fills!` item names
/// the language with none (`RustWalk`), and the same fillers serve the walk an edge composes over its ports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RustWalk<P = ()> {
    /// The ports the walk calls.
    pub(crate) ports: P,
}

/// One walked file as the fillers read it: the numbered node table the walk wrote, the file's syntax tree in the order the table
/// holds it, and what the readers work out of the file once.
#[derive(Debug)]
pub(crate) struct RustFile<'f> {
    /// The node table the walk wrote and finished.
    pub(crate) table: &'f NodeTable,
    /// The file's elements in pre-order, lent exclusively so the view can cross threads.
    pub(crate) elements: &'f mut Elements,
    /// The bytes of the file.
    pub(crate) source: ByteSlice<'f>,
    /// The number the run gave the file.
    pub(crate) file: FileId,
    /// The names the elements and the fillers intern.
    pub(crate) symbols: &'f mut Symbols,
    /// The indexes the fillers build on demand.
    pub(crate) model: &'f mut ReadIndexes,
}

/// The buffers a worker's walk keeps from file to file: refilled for each file, never reallocated.
#[derive(Debug, Default)]
pub(crate) struct RustScratch {
    /// The elements of the file in hand.
    pub(crate) elements: Elements,
    /// The names of the file in hand.
    pub(crate) symbols: Symbols,
    /// What the fillers work out of the file.
    pub(crate) model: ReadIndexes,
    /// The parser's steps for the file, kept until the roles of its elements are known.
    pub(crate) tape: Tape,
}
