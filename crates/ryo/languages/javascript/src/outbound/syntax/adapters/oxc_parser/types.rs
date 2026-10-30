//! The adapter's deferred typed tree before lexical leaves are merged into it.
use crate::domain::syntax::types::Kind;
use oxc_span::Span;

/// One projected `ESTree` node or lexical leaf.
#[derive(Debug)]
pub(super) struct Element {
    /// Its language kind; transparent Oxc wrappers use no kind.
    pub kind: Option<Kind>,
    /// Its parser byte range.
    pub span: Span,
    /// Children in parser order, sorted stably when emitted.
    pub children: Vec<Element>,
    /// First child not passed by the ordered lexical stream.
    pub next_child: ChildCursor,
    /// The parser-decoded string value, when this is a string leaf.
    pub decoded: Option<DecodedText>,
}

/// The typed visitor's open nodes and completed root.
#[derive(Debug, Default)]
pub(super) struct VisitorStack {
    /// Ancestors currently being visited.
    pub stack: Vec<Element>,
    /// Completed roots.
    pub roots: Vec<Element>,
    /// Decoded string values indexed by their source range.
    pub strings: Vec<Decoded>,
}

/// A decoded string literal at its parser range.
#[derive(Debug)]
pub(super) struct Decoded {
    /// Source range including delimiters.
    pub span: Span,
    /// The value supplied by the parser.
    pub value: DecodedText,
}

/// A parser-decoded identifier or string value, independent of source spelling.
#[derive(Debug, Clone)]
pub(super) struct DecodedText(pub String);

/// Progress through one node's children during ordered lexical insertion.
#[derive(Debug, Default)]
pub(super) struct ChildCursor(pub usize);
