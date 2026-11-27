//! Retained indentation scan storage, reused between files.
use ryo::languages::{Offset, Position, Span};
/// One complete lexical region, or an open region held only on the private scan stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndentationRegion {
    /// Original module or INDENT node identity.
    pub(crate) id: Position,
    /// Original enclosing region identity.
    pub(crate) parent: Position,
    /// Half-open lexical interval.
    pub(crate) span: Span,
    /// End of the raw indentation prefix at the start of the interval.
    pub(crate) indentation_end: Offset,
}
