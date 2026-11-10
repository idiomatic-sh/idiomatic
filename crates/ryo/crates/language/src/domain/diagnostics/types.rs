//! The types of the `diagnostics` concept: what a parser reported, its stage, its stable code and its effect on the tree.

use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Bytes, Offset};

/// What parsing and validation reported about a file, with stable metadata and source ranges.
/// Messages are kept as text, not as names, so reporting them while the table is built leaves the
/// names of the file's bytes first, as they must be.
#[derive(Debug, Default)]
pub(crate) struct Diagnostics {
    /// The text of every message, one after another.
    pub text: Bytes,
    /// Where each message begins in `text`.
    pub from: Vec<Offset>,
    /// Where each message ends in `text`.
    pub to: Vec<Offset>,
    /// Where each starts.
    pub start: Vec<Offset>,
    /// Where each ends.
    pub end: Vec<Offset>,
    /// Stable provider-owned codes, one per diagnostic.
    pub code: Vec<DiagnosticCode>,
    /// The reporting stage, one per diagnostic.
    pub phase: Vec<DiagnosticPhase>,
    /// Whether each diagnostic leaves the parsed tree incomplete.
    pub impact: Vec<TreeImpact>,
}

/// Stable diagnostic identity owned by the language provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticCode(pub &'static str);

/// Stage that produced a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticPhase {
    /// The parser or recovery machinery reported the diagnostic.
    Parse,
    /// A validation pass reported it after producing a tree.
    Validation,
}

/// Whether a diagnostic leaves the syntax tree or the token stream incomplete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeImpact {
    /// This diagnostic does not indicate missing or recovered syntax.
    Preserved,
    /// The syntax tree may be incomplete, but the provider certified tokenization through EOF.
    /// This includes preserved lexer state: byte coverage alone does not certify
    /// comments or delimiters after a lost string, interpolation or continuation.
    /// Readers may still declare individual lexical relations partial.
    LexicallyComplete,
    /// The tree may be recovered or incomplete.
    Incomplete,
}

/// Typed metadata attached to a parser or validation diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagnosticMetadata {
    /// The stage that reported the diagnostic.
    pub phase: DiagnosticPhase,
    /// Whether this diagnostic indicates incomplete syntax or only certifies a complete token stream.
    pub impact: TreeImpact,
    /// The stable code owned by the language provider.
    pub code: DiagnosticCode,
}

/// What the parser reported at one place of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Diagnostic<'a> {
    /// What parsing or validation reported.
    pub message: ByteSlice<'a>,
    /// Where it was found.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The stable code supplied by the language provider.
    pub code: DiagnosticCode,
    /// The stage that produced this finding.
    pub phase: DiagnosticPhase,
    /// Whether this finding affects the completeness of semantic facts.
    pub impact: TreeImpact,
}

/// Whether what a file's readers read from it may be missing something the parser could not read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxCompleteness {
    /// Nothing the parser reported leaves the facts of the file incomplete.
    Complete,
    /// The parser reported something that may leave them incomplete.
    Incomplete,
}
