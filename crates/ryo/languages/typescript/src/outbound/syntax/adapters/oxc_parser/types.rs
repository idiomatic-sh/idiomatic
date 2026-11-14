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
    /// The scalar properties the parser gives the node, written on it with the node.
    pub properties: Vec<NodeProperty>,
    /// Whether it carries a regular-expression pattern, and which.
    pub carries: Carries,
}

/// A visitor's stack and completed root nodes.
#[derive(Debug, Default)]
pub(super) struct VisitorStack<'a> {
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
    /// The elements that may carry a regular-expression pattern, in the order the walk met them.
    pub carriers: Vec<Carrier<'a>>,
    /// The carriers a call noted whose string the walk has not reached yet, innermost last: a call's strings are reached before any
    /// call after it is entered, so the string the walk meets is the last one's or none's.
    pub pending: Vec<CarrierIndex>,
}

/// A value decoded by the source parser.
#[derive(Clone, Debug)]
pub(super) struct DecodedText(pub String);
/// A scalar property the parser gives a node: its id and the text of its value.
#[derive(Debug)]
pub(super) struct NodeProperty {
    /// The property, among the language's ids.
    pub id: PropertyId,
    /// Its value, as the parser decoded it.
    pub value: DecodedText,
}
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

/// Whether a node carries a regular-expression pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Carries {
    /// It carries none.
    Nothing,
    /// It carries the pattern of the carrier at this place of the walk's list.
    Pattern(CarrierIndex),
}

/// A place in the walk's list of carriers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CarrierIndex(pub usize);

/// An element that may carry a regular-expression pattern: a pattern literal, or the string a call of `RegExp` takes first.
#[derive(Debug)]
pub(super) struct Carrier<'a> {
    /// The carrier's extent.
    pub span: Span,
    /// How the source writes the pattern.
    pub form: Form<'a>,
    /// How the source writes the flags.
    pub flags: PatternFlags,
    /// Where the carrier was written in the element table.
    pub host: Hosting,
}

/// How the source writes a pattern.
#[derive(Clone, Copy, Debug)]
pub(super) enum Form<'a> {
    /// Between the slashes of a pattern literal, at this extent.
    Literal(Span),
    /// As a string literal, the first argument of a call of `RegExp` by this name: read whole only when the name is the global
    /// constructor, which oxc's resolution says once it has resolved the file's names, and only checked otherwise.
    Constructor(&'a oxc_ast::ast::IdentifierReference<'a>),
    /// Not at all: the first argument of a call of `RegExp` is no string, and the carrier is the string its second argument is, whose
    /// flags alone are read.
    Flags,
}

/// How far a carrier is read: a pattern the global constructor or a literal reads is written whole; any other is only checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reach {
    /// Written whole: the flags, then the pattern's elements or the place and message of what the library refused.
    Read,
    /// Written only when the library refuses it: the flags, and the place and message of what it refused; never any element.
    Checked,
}

/// How the source writes the flags of a pattern.
#[derive(Debug)]
pub(super) enum PatternFlags {
    /// After the closing slash of a literal, at this extent.
    Bare(Span),
    /// As a string literal at this extent, which stands for this text.
    Quoted(Span, DecodedText),
    /// Not at all: the call takes no second argument.
    Absent,
    /// As an argument the source does not settle.
    Unknown,
}

/// Where a carrier stands in the element table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Hosting {
    /// Not written: the projection dropped it.
    Unwritten,
    /// Written at this position.
    At(ryo::languages::Position),
}

/// What writes a parsed pattern's elements into the element table, under the carrier it was adopted by.
pub(super) struct PatternWriter<'b, 't> {
    /// The table being written.
    pub builder: &'b mut ryo::languages::TableBuilder<'t>,
}
