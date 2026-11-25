//! The types of the `file_view` concept: the language as the engine's walk names it, and the view of one walked file the fillers read.

use crate::domain::model::types::ReadIndexes;
use ryo::languages::{self, ByteSlice, FileId, NodeTable};

/// The typescript language as the engine walks it, over the ports `P` its walk calls. Its fillers call no port, so a `fills!` item
/// names the language with none (`TypeScriptWalk`), and the same fillers serve the walk an edge composes over its ports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TypeScriptWalk<P = ()> {
    /// The ports the walk calls.
    pub(crate) ports: P,
}

/// The control-flow facts of a file as the parser's semantic pass drafted them, each node as its id in the numbered table.
#[derive(Debug, Default)]
pub(crate) struct FlowFacts {
    /// Every block, numbered from zero in this order.
    pub(crate) blocks: Vec<languages::FlowBlockFact>,
    /// Every edge, by block numbers.
    pub(crate) edges: Vec<languages::FlowEdgeFact>,
    /// Every step that is a node.
    pub(crate) steps: Vec<languages::FlowStepFact>,
    /// Every node's block.
    pub(crate) members: Vec<languages::FlowMemberFact>,
}

/// What the library made of one carrier's pattern, as the walk wrote it: the pattern's elements are in the table under the carrier.
#[derive(Debug)]
pub(crate) struct CarrierFacts {
    /// The carrier's element.
    pub(crate) host: languages::Position,
    /// The flags the pattern was read with.
    pub(crate) flags: String,
    /// The library's message when it refused the pattern or the flags.
    pub(crate) refusal: Option<String>,
    /// Whether the message is about the flags.
    pub(crate) flags_refused: bool,
    /// Where the library refused, when it said.
    pub(crate) place: Option<(languages::Offset, languages::Offset)>,
}

/// One walked file as the fillers read it: its numbered node table and its text. The parse, the typed node of each id and the
/// model each filler reads are added to this view as the relations are filled.
#[derive(Debug)]
pub(crate) struct TypeScriptFile<'f> {
    /// The file's node table, finished.
    pub(crate) table: &'f NodeTable,
    /// The file's bytes.
    pub(crate) source: ByteSlice<'f>,
    /// The file's number.
    pub(crate) file: FileId,
    /// The indexes the readers of the file build on the first read that asks.
    pub(crate) model: ReadIndexes,
    /// The control-flow facts, when the walk was asked for them.
    pub(crate) flow: Option<FlowFacts>,
    /// What the library made of each carrier's pattern, when the walk was asked for patterns.
    pub(crate) patterns: Vec<CarrierFacts>,
}
