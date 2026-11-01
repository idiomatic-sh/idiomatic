//! Plain values for the decoded brace-format parser.
use ryo::languages::Position;
use std::collections::HashMap;

/// Stable categories reported by the brace field parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BraceFormatError {
    /// A bracket opened without a matching closing bracket.
    UnmatchedBracket,
    /// A closing brace appeared where a field was expected.
    MissingStartBracket,
    /// A format field nested more than one level.
    PlaceholderRecursionExceeded,
    /// A field conversion contains zero or multiple characters.
    UnknownConversion,
    /// A field has an empty attribute or index.
    EmptyAttribute,
    /// A field index has no closing bracket.
    MissingRightBracket,
    /// Text followed a bracket index without a dot or another bracket.
    InvalidCharacterAfterRightBracket,
}

/// The root selector written in one brace field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BraceSelector {
    /// Automatic positional numbering.
    Automatic {
        /// The next implicit positional argument.
        index: usize,
        /// Whether the index exceeds the relation's signed integer range.
        overflow: bool,
    },
    /// An explicit positional index. `overflow` means it fits `usize` but not `i64`.
    Index {
        /// The parsed positional argument index.
        index: usize,
        /// Whether the index exceeds the relation's signed integer range.
        overflow: bool,
        /// Its exact decimal spelling.
        text: String,
    },
    /// A keyword selector, including digit strings larger than `usize` per upstream behavior.
    Keyword(String),
}

/// Where a field appears relative to its owning placeholder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BraceFieldContext {
    /// A field in the decoded string itself.
    Literal,
    /// A field nested in its parent's format specification.
    FormatSpec {
        /// The outer field whose format specification contains this field.
        parent_ordinal: usize,
    },
}

/// One field occurrence in source traversal order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BraceFormatField {
    /// Zero-based order: outer fields precede fields in their format specifications.
    pub ordinal: usize,
    /// The owning top-level field, or no parent for a literal field.
    pub context: BraceFieldContext,
    /// Its root selector.
    pub selector: BraceSelector,
}

/// Successfully parsed field sequence and upstream-compatible selector counts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ParsedBraceFormat {
    /// Fields, preserving duplicate occurrences and traversal order.
    pub fields: Vec<BraceFormatField>,
    /// Number of automatic positional selectors.
    pub automatic_count: usize,
    /// Number of explicit positional selectors.
    pub indexed_count: usize,
    /// Number of keyword selectors.
    pub keyword_count: usize,
    /// Number of fields nested in format specifications.
    pub nested_field_count: usize,
}

/// The parser outcome for one distinct decoded input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum BraceFormatResult {
    /// Parsed, including a valid format with no fields.
    Parsed(ParsedBraceFormat),
    /// Invalid format syntax or field name.
    Malformed(BraceFormatError),
    /// The input could not be decoded or retained completely.
    Unavailable,
}

/// One result retained per decoded input string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CachedBraceFormat {
    /// Parser result.
    pub result: BraceFormatResult,
}

/// One original literal occurrence and its cached parse result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BraceFormatOccurrence {
    /// Original string-literal node.
    pub node: Position,
    /// Index into cached values, absent for dynamic/unknown input.
    pub cached: Option<usize>,
    /// Dynamic string content is unknown rather than unavailable.
    pub unknown_input: bool,
}

/// Retained parse state for one source tree; populated only when either brace relation is requested.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BraceFormatIndex {
    /// Decoded input to cached parse result.
    pub by_input: HashMap<String, usize>,
    /// Distinct decoded input results.
    pub values: Vec<CachedBraceFormat>,
    /// Source occurrences.
    pub occurrences: Vec<BraceFormatOccurrence>,
    /// Completeness of the current source's brace-format inputs.
    pub state: BraceFormatState,
}

/// Cache/read completeness.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BraceFormatState {
    /// No reader has prepared the index since the source was parsed.
    #[default]
    Fresh,
    /// Every visible literal was represented.
    Complete,
    /// A literal or parser result could not be represented.
    Partial,
}
