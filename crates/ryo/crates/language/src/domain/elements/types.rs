// Numbers, Elements, TableBuilder and Completion are the engine's element table and the builders that write it, handed to every language: what only the engine may write is a private field holding a crate-private engine part (WrittenColumn in Numbers, ElementsEngine in TableState, BuilderEngine, CompletionEngine), reached by the crate through `AsRef`/`AsMut`, so a language builds a tree only through the SDK's functions and cannot corrupt the table. A public field would hand a language the table's invariants; Rust's privacy is the compiler's enforcement.
//! The types of the `elements` concept: the element table of a file and what builds and reads it.

use super::constants::{
    INCOMPLETE_IMPACT, LEXICALLY_COMPLETE_IMPACT, PARSE_PHASE, PRESERVED_IMPACT, SCRATCH,
    UNSPECIFIED_CODE, VALIDATION_PHASE,
};
use crate::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::Expanded;
use ryo_relations::domain::relations::types::{
    Bytes, Count, Extent, FieldId, FileId, Flags, KindBits, Offset, Position, PropertyId,
};
use ryo_relations::domain::terms::types::ChildOrdinal;
use ryo_relations::domain::terms::types::Symbol;

use crate::domain::symbols::types::Symbols;

/// The parent of an element about to be added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parent {
    /// The element is a root: it is its own parent.
    Root,
    /// The element is a child of this one.
    Of(Position),
}

/// Where the next element goes: its parent and its place among the parent's children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementPlacement {
    /// The parent.
    pub parent: Parent,
    /// The element's place among its parent's children.
    pub ordinal: ChildOrdinal,
}

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
pub struct DiagnosticCode(&'static str);

impl DiagnosticCode {
    /// Used by existing providers that report messages without a narrower stable code.
    pub const UNSPECIFIED: Self = Self::provider_owned(UNSPECIFIED_CODE);

    /// Construct a stable diagnostic code owned by a language provider.
    #[must_use]
    pub const fn provider_owned(code: &'static str) -> Self {
        Self(code)
    }

    /// The stable code text for the provider's relation reader.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Stage that produced a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticPhase {
    /// The parser or recovery machinery reported the diagnostic.
    Parse,
    /// A validation pass reported it after producing a tree.
    Validation,
}

impl DiagnosticPhase {
    /// The stable query vocabulary.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Parse => PARSE_PHASE,
            Self::Validation => VALIDATION_PHASE,
        }
    }
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

impl DiagnosticMetadata {
    /// Build metadata for a provider-owned diagnostic.
    #[must_use]
    pub const fn new(phase: DiagnosticPhase, impact: TreeImpact, code: DiagnosticCode) -> Self {
        Self {
            phase,
            impact,
            code,
        }
    }
}

impl TreeImpact {
    /// The stable query vocabulary.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Preserved => PRESERVED_IMPACT,
            Self::LexicallyComplete => LEXICALLY_COMPLETE_IMPACT,
            Self::Incomplete => INCOMPLETE_IMPACT,
        }
    }
}

/// A column of numbers, one for each row, indexed by the row's [`Position`]: the engine writes it, a language only reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Numbers<T>(WrittenColumn<T>);

impl<T> Default for Numbers<T> {
    /// An empty column.
    fn default() -> Self {
        Self(WrittenColumn::default())
    }
}

/// The numbers of a column as the engine writes them: one for each row, and how many of the last rows the language added for
/// reading after the file's own tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WrittenColumn<T> {
    /// The numbers, one for each row.
    pub values: Vec<T>,
    /// How many of the last rows the language added for reading, which the column's slice leaves out.
    pub added: Count,
}

impl<T> Default for WrittenColumn<T> {
    /// An empty column.
    fn default() -> Self {
        Self {
            values: Vec::new(),
            added: Count(0),
        }
    }
}

impl<T> AsRef<WrittenColumn<T>> for Numbers<T> {
    /// The column, for the crate to read whole.
    fn as_ref(&self) -> &WrittenColumn<T> {
        &self.0
    }
}

impl<T> AsMut<WrittenColumn<T>> for Numbers<T> {
    /// The column, for the crate to write.
    fn as_mut(&mut self) -> &mut WrittenColumn<T> {
        &mut self.0
    }
}

impl<T> std::ops::Deref for Numbers<T> {
    type Target = [T];

    /// The column to read, without the elements the language added for reading after the file's own tree: the readers that ask
    /// for those reach them by their place.
    fn deref(&self) -> &[T] {
        let column = &self.0;
        column
            .values
            .get(..column.values.len().saturating_sub(column.added.0))
            .unwrap_or_default()
    }
}

/// An open node while the table is built: where it is and how many children it has so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenNode {
    /// The node.
    pub position: Position,
    /// The place the next child takes among its children.
    pub next: ChildOrdinal,
}

/// What the names of the kinds of a file add up to: each kind's number and the hash of its name, summed, so the order they were named
/// in does not matter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KindSum(pub u64);

/// Whether the provider's parser accepted the source under its selected syntax policy.
///
/// This is separate from whether a parser produced a recoverable tree or reported validation
/// diagnostics. Providers that cannot make this distinction should leave the value undetermined.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SyntaxAcceptance {
    /// The parser accepted the source.
    Accepted,
    /// The parser rejected the source, even if it retained a recovery tree.
    Rejected,
    /// The provider did not determine whether the source was accepted.
    #[default]
    Undetermined,
}

/// The element table of one file: its syntax tree in pre-order, column by column, 28 bytes for
/// each element. An element's descendants are the `size` elements after it; the root is its own
/// parent. The columns are cleared between files and never reallocated.
#[derive(Debug, Default)]
pub struct Elements {
    /// What kind of node or token the element is, in the language's own numbering.
    pub kind: Numbers<KindBits>,
    /// Bit flags; [`TOKEN`](ryo_relations::domain::relations::constants::TOKEN) marks a token.
    pub flags: Numbers<Flags>,
    /// Where it starts, in bytes.
    pub start: Numbers<Offset>,
    /// Where it ends, in bytes.
    pub end: Numbers<Offset>,
    /// How many elements are below it.
    pub size: Numbers<Extent>,
    /// Its parent, by position.
    pub parent: Numbers<Position>,
    /// Its position among its parent's children.
    pub ordinal: Numbers<ChildOrdinal>,
    /// Its role under its parent, in the language's own vocabulary; [`NO_FIELD`](ryo_relations::domain::relations::constants::NO_FIELD) where it has none. Empty until the language is asked for the roles (`FieldRequest::Wanted`).
    pub field: Numbers<FieldId>,
    /// The text of a token, interned; [`NO_SYMBOL`](ryo_relations::domain::relations::constants::NO_SYMBOL) for a node and for a token nobody reads.
    pub text: Numbers<Symbol>,
    /// What the engine keeps of the table beside its columns, which a language holds but never reaches into.
    pub state: TableState,
}

/// What the engine keeps of a file's element table beside its columns, opaque to a language, which holds it with the table.
#[derive(Debug, Default)]
pub struct TableState(ElementsEngine);

/// What the engine keeps of a file's table beside its columns: names, kind index, decoded texts, expansions, open nodes, notes.
#[derive(Debug, Default)]
pub(crate) struct ElementsEngine {
    /// Parser acceptance for this source; legacy providers remain conservative by default.
    pub syntax_acceptance: SyntaxAcceptance,
    /// Source line starts, populated only when source-location is requested; storage is reused.
    pub line_starts: crate::domain::location::types::SourceLineIndex,
    /// Whether native preorder also follows increasing source starts.
    pub source_order: SourceOrder,
    /// What each kind is called, by the number of the kind, as the file's names; no symbol for a kind none of its elements has.
    pub names: Vec<Symbol>,
    /// What the names of the kinds add up to, the same for every file of a language and not for another: they are numbered otherwise.
    pub vocabulary: KindSum,
    /// Where the elements of each kind are, once the table is built: `kind_start[k]..kind_start[k + 1]` of `kind_order`.
    pub kind_start: Vec<Count>,
    /// The positions of the elements, grouped by kind and in order within a kind.
    pub kind_order: Vec<Position>,
    /// What each role is called, by its number, as the file's names; no symbol for a role the language did not name.
    pub field_names: Vec<Symbol>,
    /// The text a token stands for, as its language's library decoded it (a string's value with its escapes resolved), for the tokens it
    /// decoded one of, in the order of their positions.
    pub decoded: Vec<DecodedProperty>,
    /// Actual decoded-entry visits by the interning algorithm, observed only in tests.
    #[cfg(test)]
    pub decoded_intern_visits: usize,
    /// The bytes of the decoded texts.
    pub decoded_text: Bytes,
    /// The elements a macro expansion produced, in the order of their positions; none for a file no expansion touched.
    pub expanded: Vec<Expanded>,
    /// Separately parsed expressions, kept outside the native lossless tree.
    pub embedded: Vec<ryo_relations::domain::relations::types::EmbeddedSource>,
    /// The open nodes while the table is built.
    pub open: Vec<OpenNode>,
    /// The model the worker keeps for the language between files; the engine's, which the language never sees but through `file_model`.
    pub scratch: LanguageScratch,
    /// Engine-owned initialization identity for the current native population.
    pub model_preparation: ModelPreparation,
    /// What the language wrote of the file besides its tree, bytes only it reads, when the request asked for them.
    pub notes: Bytes,
    /// What the parser reported.
    pub diagnostics: Diagnostics,
    /// Where the elements the language added for reading begin, when it added any: trees that hang off an element of the file (the
    /// arguments of a macro read as expressions) after every element the parser gave. A walk gives them only to the readers that ask,
    /// and no tree operation counts them.
    pub expanded_from: Option<Position>,
}

impl AsRef<ElementsEngine> for TableState {
    /// What the engine keeps of the table, for the crate to read.
    fn as_ref(&self) -> &ElementsEngine {
        &self.0
    }
}

impl AsMut<ElementsEngine> for TableState {
    /// What the engine keeps of the table, for the crate to write.
    fn as_mut(&mut self) -> &mut ElementsEngine {
        &mut self.0
    }
}

/// What a worker keeps for the language between files: the model it refills for each file.
#[derive(Default)]
pub(crate) struct LanguageScratch(pub Option<Box<dyn std::any::Any + Send>>);

impl std::fmt::Debug for LanguageScratch {
    /// Names the model without printing it: it is the language's, of a type only the language knows.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(SCRATCH)
    }
}

/// Whether the worker's model has been initialized for the current native table.
/// Completion appends do not change this identity; a native clear invalidates it first.
#[derive(Debug, Default)]
pub(crate) struct ModelPreparation {
    /// Canonical parser settings, with room reused across generations.
    pub configuration: String,
    /// Only native elements participate; appended expressions do not require a second resolver.
    pub native_count: Count,
    /// Set to `Ready` only by the common preparation boundary after provider initialization returns.
    pub ready: ModelReadiness,
}

/// Whether the worker's model was initialized for the table it now holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ModelReadiness {
    /// The model is not initialized, or the table changed under it.
    #[default]
    Stale,
    /// The model was initialized by the common preparation boundary.
    Ready,
}

/// What a language's syntax adapter builds a file's table through: the table, and the file's bytes
/// the names are read from. The adapter opens a node, adds a token, closes a node, reports what the parser found (`enter_node`, `leave_node`, `token`, `diagnose`); its fields are the engine's, so it reaches nothing else.
#[derive(Debug)]
pub struct TableBuilder<'a>(BuilderEngine<'a>);

/// What a [`TableBuilder`] writes through, the engine's: the table, the names and the file's bytes.
#[derive(Debug)]
pub(crate) struct BuilderEngine<'a> {
    /// The worker's table.
    pub elements: &'a mut Elements,
    /// The names of the tokens, interned as the table is built.
    pub symbols: &'a mut Symbols,
    /// The bytes of the file.
    pub source: ByteSlice<'a>,
    /// Separate decoded text while appending an embedded expression. Its symbols
    /// are derived names, never offsets into the original file.
    pub logical_source: Option<ByteSlice<'a>>,
    /// Whether it builds nothing: reading a file through it does what the library does and writes no element, no name and no
    /// diagnostic. The engine reads a file this way to know what the library alone costs.
    pub writing: TableWriting,
}

impl<'a> From<BuilderEngine<'a>> for TableBuilder<'a> {
    /// The builder that writes through `engine`.
    fn from(engine: BuilderEngine<'a>) -> Self {
        Self(engine)
    }
}

impl<'a> AsMut<BuilderEngine<'a>> for TableBuilder<'a> {
    /// What the builder writes through, for the crate's operations on it.
    fn as_mut(&mut self) -> &mut BuilderEngine<'a> {
        &mut self.0
    }
}

/// What a language completes the table of a file through, once its tree is built: it names its kinds and writes its notes, and reaches
/// nothing else of the table (`name_kind`, `write_notes`); its fields are the engine's.
#[derive(Debug)]
pub struct Completion<'a>(CompletionEngine<'a>);

/// What a [`Completion`] completes, the engine's: the table, the names, the file's bytes and its number.
#[derive(Debug)]
pub(crate) struct CompletionEngine<'a> {
    /// The file's table, whose kinds are named and whose notes are written.
    pub elements: &'a mut Elements,
    /// The names the kinds are named with.
    pub symbols: &'a mut Symbols,
    /// The bytes of the file.
    pub source: ByteSlice<'a>,
    /// The number the run gave the file.
    pub file: FileId,
}

impl<'a> From<CompletionEngine<'a>> for Completion<'a> {
    /// The completion of what `engine` holds.
    fn from(engine: CompletionEngine<'a>) -> Self {
        Self(engine)
    }
}

impl<'a> AsMut<CompletionEngine<'a>> for Completion<'a> {
    /// What the completion writes, for the crate's operations on it.
    fn as_mut(&mut self) -> &mut CompletionEngine<'a> {
        &mut self.0
    }
}

/// The children of one element, in order, found by skipping each child's subtree.
#[derive(Clone, Debug)]
pub struct Children<'a> {
    /// The table.
    pub elements: &'a Elements,
    /// The next child.
    pub next: Position,
    /// The last position inside the parent.
    pub last: Position,
}

impl Iterator for Children<'_> {
    type Item = Position;

    fn next(&mut self) -> Option<Position> {
        if self.next > self.last {
            return None;
        }
        let child = self.next;
        let size = self
            .elements
            .size
            .as_ref()
            .values
            .get(usize::try_from(child.0).unwrap_or(usize::MAX))?;
        self.next = Position(child.0 + size.0 + 1);
        Some(child)
    }
}

/// A file's element table read in place, with the file's bytes and number: what every reader
/// reads its rows from. The table is only read; the names a reader makes are in the symbols it is given beside it.
#[derive(Clone, Copy, Debug)]
pub struct Tree<'a> {
    /// The file's syntax tree, in pre-order.
    pub elements: &'a Elements,
    /// The bytes of the file.
    pub source: ByteSlice<'a>,
    /// The number the run gave the file.
    pub file: FileId,
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

/// Whether a builder writes what it is given into the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableWriting {
    /// It writes the elements, the names and the diagnostics.
    Writes,
    /// It writes nothing: reading a file through it does what the library does and no more.
    Discards,
}

/// The text a token stands for, decoded by the language's library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedProperty {
    /// The element.
    pub element: Position,
    /// The scalar property, or zero for the decoded element value.
    pub property: PropertyId,
    /// Where its decoded text starts in the bytes of the decoded texts.
    pub from: Offset,
    /// Where it ends.
    pub to: Offset,
}

/// Whether a tree's native ordering preserves source starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceOrder {
    /// Source starts never decrease.
    #[default]
    Forward,
    /// Tree construction reordered nodes or inserted unlocated nodes.
    Reordered,
}

/// A formatter writing directly into retained parser-property storage.
pub(crate) struct PropertyText<'a>(pub &'a mut Vec<u8>);
impl std::fmt::Write for PropertyText<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

/// Semantic input required when completing a provider's native syntax population.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompletionRequirement {
    /// Completion does not require a prepared file model; preserve legacy provider order.
    #[default]
    Independent,
    /// The common engine must prepare the provider model once before completion.
    PreparedModel,
}
