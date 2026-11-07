//! Retained indentation scan storage, reused between files.
use ryo::languages::{Offset, Position, Span};
/// One complete lexical region, or an open region held only on the private scan stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndentationRegion {
    /// Original module or INDENT node identity.
    pub id: Position,
    /// Original enclosing region identity.
    pub parent: Position,
    /// Half-open lexical interval.
    pub span: Span,
    /// End of the raw indentation prefix at the start of the interval.
    pub indentation_end: Offset,
}
/// Whether the current file's lexical index has been requested and completed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum IndentationState {
    /// No relation has requested the current file's regions.
    #[default]
    Fresh,
    /// The current file has a balanced complete lexical token stream.
    Complete,
    /// Parser diagnostics or an unbalanced token stream prevent a complete answer.
    Incomplete,
}
/// Scratch capacity belongs to the retained file model; scanning is lazy.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct IndentationIndex {
    /// Rows in source order after completion.
    pub rows: Vec<IndentationRegion>,
    /// Open regions during the one token scan.
    pub stack: Vec<IndentationRegion>,
    /// Validity for the current file.
    pub state: IndentationState,
}
