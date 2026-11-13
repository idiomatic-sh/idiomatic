//! The standard tree while Oxc's typed visitor projects its nodes.

use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use oxc_semantic::Scoping;
use oxc_span::Span;
use ryo::languages::PropertyId;

/// The text attached to a leaf.
#[derive(Debug)]
pub(super) enum Text {
    /// The spelling in the source.
    Source,
    /// The value the parser decoded.
    Decoded(DecodedText),
}

/// One node or token in the standard tree.
#[derive(Debug)]
pub(super) struct Piece {
    /// Its standard kind.
    pub kind: Kind,
    /// Its byte range.
    pub span: Span,
    /// Its source-ordered children.
    pub children: Vec<Piece>,
    /// Its decoded value, when the parser provides one.
    pub text: Text,
    /// First child that can contain the next source-ordered token.
    pub cursor: ChildCursor,
    /// The scalar properties the parser's semantic pass gives the node, written on it after it opens.
    pub properties: Vec<NodeProperty>,
}

/// One scalar property of a node, as the text the domain reads back with `languages::property_at`.
#[derive(Debug)]
pub(super) struct NodeProperty {
    /// Which property it is.
    pub id: PropertyId,
    /// Its value.
    pub value: DecodedText,
}

/// A visitor's stack and completed root nodes.
#[derive(Debug, Default)]
pub(super) struct VisitorStack {
    /// Nodes currently being visited.
    pub open: Vec<Piece>,
    /// Completed root nodes.
    pub roots: Vec<Piece>,
    /// Parser-decoded values indexed by their source ranges.
    pub decoded: Vec<Decoded>,
    /// Native node whose typed callback finishes its projection after the upstream walk.
    pub deferred: Option<VisitDepth>,
    /// The symbols and references of the parser's semantic pass, which the references and bindings of the tree are resolved by.
    pub scoping: Scoping,
}

/// A value decoded by the source parser.
#[derive(Clone, Debug)]
pub(super) struct DecodedText(pub String);
/// The decoded value at an exact parser source range.
#[derive(Debug)]
pub(super) struct Decoded {
    /// The source range the value was decoded from.
    pub span: Span,
    /// The value the parser decoded.
    pub value: DecodedText,
    /// The token the range is, in the standard's names.
    pub token_kind: Token,
}

/// Position within one projected node’s children.
#[derive(Debug)]
pub(super) struct ChildCursor(pub usize);

/// Stack depth before entering a node with a typed normalization callback.
#[derive(Debug)]
pub(super) struct VisitDepth(pub usize);
