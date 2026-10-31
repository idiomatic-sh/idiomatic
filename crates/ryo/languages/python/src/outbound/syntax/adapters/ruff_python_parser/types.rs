//! The types of the `ruff_python_parser` adapter: plain data, public fields and derives, with no constructor and no inherent impl.
use ruff_python_ast::token::{Token as RuffToken, Tokens};
use ryo::languages::{self, Offset};

/// How many nodes are open: zero outside the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Depth(pub usize);

/// Whether the source-order visitor is traversing a type annotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AnnotationContext {
    /// Ordinary runtime syntax.
    Runtime,
    /// A syntactic annotation; semantic exclusions are checked after binding.
    Annotation,
    /// A possible typing API argument or type-alias value, not yet semantically classified.
    Candidate,
}

/// A quoted expression candidate and the native element retaining its source.
#[derive(Clone, Debug)]
pub(super) struct AnnotationCandidate {
    /// The original string node in the element table.
    pub host: languages::Position,
    /// Parser-decoded value and source mapping information.
    pub literal: ruff_python_ast::ExprStringLiteral,
}

/// A place among the bytes of the file, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ByteIndex(pub usize);

/// A place among the tokens of the library, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TokenIndex(pub usize);

/// The text the library reads for a file: the file's own bytes when they are UTF-8, or a copy with each byte that is not replaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Readable<'a>(pub std::borrow::Cow<'a, str>);

/// The text the library decoded for a string's tokens: the value of a string literal, or the literal part of an f-string or a
/// t-string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Decoded<'a>(pub &'a str);

/// Which kind of string an interpolation is in, the innermost one open: what the standard names it by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Strings {
    /// An f-string, or none: a `FormattedValue`.
    Formatted,
    /// A t-string: an `Interpolation`.
    Template,
}

/// What `CPython`'s parser says where it refuses what the library reads without an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Said(pub String);

/// A place `CPython`'s parser refuses and what it says there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParserRefusal {
    /// Where.
    pub at: ruff_text_size::TextRange,
    /// What.
    pub said: Said,
}

/// The writer of the table: it walks the library's tree in the order of the file and writes its tokens between the nodes.
pub(super) struct Writer<'b, 'w, 't> {
    /// Annotation candidates are collected only in explicit annotation contexts.
    pub annotations: Vec<AnnotationCandidate>,
    /// Current syntactic context, refined by semantic binding before projection.
    pub annotation: AnnotationContext,
    /// The first logical source byte emitted by this traversal.
    pub start: Offset,
    /// Whether canonical identifiers must be retained independently of physical spans.
    pub embedding: AnnotationContext,
    /// The tree, and where its last token ends.
    pub tree: languages::Lossless<'b, 'w>,
    /// The library's tokens, in the order of the file.
    pub tokens: &'t [RuffToken],
    /// The first token not written yet.
    pub next: TokenIndex,
    /// How many nodes are open.
    pub depth: Depth,
    /// Where each open node ends, the innermost last.
    pub ends: Vec<Offset>,
    /// The strings open, the innermost last: what an interpolation is named by.
    pub strings: Vec<Strings>,
    /// The library's tokens, to find a keyword in.
    pub parsed_tokens: &'t Tokens,
    /// Where `CPython` refuses what the library reads: what the adapter reports after the library's own errors.
    pub refused: Vec<ParserRefusal>,
    /// How long the file is.
    pub length: Offset,
    /// Source spelling retained only to distinguish normalized identifiers.
    pub source: languages::NameText<'t>,
}
