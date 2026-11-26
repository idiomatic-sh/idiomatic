//! The types of the `ruff_python_parser` adapter: plain data, public fields and derives, with no constructor and no inherent impl.
use ruff_python_ast::token::{Token as RuffToken, Tokens};
use ryo::languages::{self, DiagnosticCode, DiagnosticPhase, Offset, TreeImpact};

/// How many nodes are open: zero outside the root.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Depth(pub usize);

/// A place among the bytes of the file, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ByteIndex(pub usize);

/// A place among the tokens of the library, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TokenIndex(pub usize);

/// The text the library reads for a file: the file's own bytes when they are UTF-8, or a copy with each byte that is not replaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Readable<'a>(pub std::borrow::Cow<'a, str>);

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
    /// Diagnostic stage and tree impact.
    pub metadata: languages::DiagnosticMetadata,
}

/// The text of one parser diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DiagnosticMessage(pub String);

impl From<&str> for DiagnosticMessage {
    fn from(text: &str) -> Self {
        Self(text.to_owned())
    }
}

impl From<String> for DiagnosticMessage {
    fn from(text: String) -> Self {
        Self(text)
    }
}

/// One parser-library syntax diagnostic, before byte-range normalization.
pub(super) struct SemanticSyntaxDiagnostic {
    /// The diagnostic text emitted by the parser.
    pub message: DiagnosticMessage,
    /// The parser-native source range.
    pub range: ruff_text_size::TextRange,
    /// Stable provider-owned classification code.
    pub code: DiagnosticCode,
    /// The parser or validation phase that owns this diagnostic.
    pub phase: DiagnosticPhase,
    /// Whether the diagnostic preserves usable syntax facts.
    pub impact: TreeImpact,
}

/// Whether a valid module-prefix future statement enables postponed annotations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct FutureAnnotations(pub bool);

/// Whether the configured language version uses native deferred annotations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct DeferredAnnotations(pub bool);

/// Whether an annotation sits under generic type parameters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct GenericScope(pub bool);

/// Whether the lexer's token ranges cover the source completely.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct LexicallyComplete(pub bool);

/// Whether a certified synthetic end-of-file error marks an unclosed delimiter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct DelimiterEof(pub bool);

/// Whether every future directive seen so far is known.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct FutureValidity(pub bool);

/// Whether a token stream ends inside an open delimiter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct UnclosedDelimiter(pub bool);

/// A name bound in a scope; it hashes and compares as its text, so lookups by `&str` work.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(super) struct BoundName(pub String);

impl std::borrow::Borrow<str> for BoundName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// A place among the fields the open nodes of a walk hand their children, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FieldIndex(pub usize);

/// A node the walk has opened and not closed yet.
#[derive(Clone, Copy, Debug)]
pub(super) struct OpenWalked<'t> {
    /// The node.
    pub node: ruff_python_ast::AnyNodeRef<'t>,
    /// The id the walk gave it.
    pub at: languages::Position,
    /// Where the fields of its children start among the walk's fields.
    pub from: FieldIndex,
    /// The field of its next child, when its children open in the order the fields were read.
    pub next: FieldIndex,
}

/// The walker of the node table: it walks the library's tree in the order of the file, writes each node with the field it fills
/// and the tokens between the nodes, and keeps the typed node of each id.
pub(super) struct Walker<'b, 'w, 't> {
    /// The table's builder.
    pub builder: &'b mut languages::NodeTableBuilder<'w>,
    /// The library's tokens, in the order of the file.
    pub tokens: &'t [RuffToken],
    /// The first token not written yet.
    pub next: TokenIndex,
    /// Where the last token written ends: the gap up to the next token is white space.
    pub written: Offset,
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
    /// Whether the module postpones its annotations: the property the walk records on the module node.
    pub future_annotations: FutureAnnotations,
    /// Whether the module's annotations are postponed (the future import, or a stub): the timing property of the module node.
    pub annotations_deferred: bool,
    /// Whether declarations without values bind names in this source.
    pub declarations: crate::domain::model::types::AnnotatedDeclarations,
    /// The typed node of each id written so far.
    pub nodes: Vec<ruff_python_ast::AnyNodeRef<'t>>,
    /// The parent of each id written so far; the root is its own.
    pub parents: Vec<languages::Position>,
    /// Where the subtree of each id written so far ends; a node still open ends after itself.
    pub subtree_ends: Vec<languages::Position>,
    /// The open nodes, the innermost last.
    pub open: Vec<OpenWalked<'t>>,
    /// Where the text being walked starts among the offsets the library gives: zero for a file, the start of the expression
    /// for a string read in place.
    pub origin: Offset,
    /// The node that holds the expression being walked, for the expression of a string; none for a file.
    pub host: Option<languages::Position>,
    /// How many annotations are open: a string literal under one stands where the language writes a type.
    pub annotating: Depth,
    /// The string literals met.
    pub literals: Vec<crate::domain::node::types::StringSite>,
    /// The imported names met: the places of the aliases.
    pub imports: Vec<languages::Position>,
    /// The fields the open nodes hand their children, one run for each open node.
    pub fields: Vec<crate::domain::node::types::ChildField<'t>>,
    /// The names interned so far: of each node, and of each name token of the file's own tree.
    pub names: crate::domain::model::names::types::WalkNames,
    /// The file's text, when its bytes are text and this is the file's own tree: a name token's code is of its text here.
    /// Empty for an embedded segment and for a file that is no text: their name tokens get no code.
    pub text: &'t str,
}

/// A string the language reads as a type expression, parsed: the library's parse of it, the text its offsets are of, and where
/// its expression is in that text.
pub(super) struct ParsedString<'p> {
    /// The place of the string literal.
    pub host: languages::Position,
    /// The parse.
    pub parsed: StringParse,
    /// The text the parse's offsets are of: the text of the string's own tree when it is read in place, its decoded value
    /// otherwise.
    pub text: languages::NameText<'p>,
    /// Where the expression starts in that text.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// How the segment's bytes map to the file's.
    pub mapping: languages::EmbeddedMapping,
}

/// The library's parse of a string as a type expression.
pub(super) enum StringParse {
    /// Read in place: the offsets are the string's own.
    InPlace(ruff_python_parser::typing::ParsedAnnotation),
    /// Read from the decoded value: the string has escapes or is written in parts.
    Decoded(ruff_python_parser::Parsed<ruff_python_ast::ModExpression>),
}

/// The text the offsets of a string's nodes are of, and how its bytes map to the file's.
#[derive(Clone, Copy, Debug)]
pub(super) struct TextAround<'p> {
    /// The text.
    pub text: languages::NameText<'p>,
    /// How its bytes map to the file's.
    pub mapping: languages::EmbeddedMapping,
}
