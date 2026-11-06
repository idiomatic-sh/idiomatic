//! Plain values for the decoded brace-format parser.
use crate::domain::format_syntax::types::{
    FormatCharacter, FormatFieldOrdinal, RetainedFormatIndex,
};

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
        index: BraceArgumentIndex,
        /// Whether the index exceeds the relation's signed integer range.
        overflow: SelectorRange,
    },
    /// An explicit positional index. `overflow` means it fits `usize` but not `i64`.
    Index {
        /// The parsed positional argument index.
        index: BraceArgumentIndex,
        /// Whether the index exceeds the relation's signed integer range.
        overflow: SelectorRange,
        /// Its exact decimal spelling.
        text: BraceSelectorText,
    },
    /// A keyword selector, including digit strings larger than `usize` per upstream behavior.
    Keyword(BraceSelectorText),
}

/// Where a field appears relative to its owning placeholder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BraceFieldContext {
    /// A field in the decoded string itself.
    Literal,
    /// A field nested in its parent's format specification.
    FormatSpec {
        /// The outer field whose format specification contains this field.
        parent_ordinal: FormatFieldOrdinal,
    },
}

/// One field occurrence in source traversal order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BraceFormatField {
    /// Zero-based order: outer fields precede fields in their format specifications.
    pub ordinal: FormatFieldOrdinal,
    /// The owning top-level field, or no parent for a literal field.
    pub context: BraceFieldContext,
    /// Its root selector.
    pub selector: BraceSelector,
    /// Decoded text after an explicit colon; absence differs from an empty spec.
    pub format_spec: Option<BraceSpecText>,
    /// Complete selector path before conversion/specification.
    pub selector_text: BraceSelectorText,
    /// Explicit conversion; absence preserves the original value protocol.
    pub conversion: Option<FormatCharacter>,
}

/// Successfully parsed field sequence and upstream-compatible selector counts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ParsedBraceFormat {
    /// Fields, preserving duplicate occurrences and traversal order.
    pub fields: Vec<BraceFormatField>,
    /// Number of automatic positional selectors.
    pub automatic_count: BraceFieldCount,
    /// Number of explicit positional selectors.
    pub indexed_count: BraceFieldCount,
    /// Number of keyword selectors.
    pub keyword_count: BraceFieldCount,
    /// Number of fields nested in format specifications.
    pub nested_field_count: BraceFieldCount,
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

/// Retained parse state for one source tree; populated only when either brace relation is requested.
pub(crate) type BraceFormatIndex =
    RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>;

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

/// Whether an argument index fits the relation's signed integer range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectorRange {
    Representable,
    Overflow,
}
impl From<bool> for SelectorRange {
    fn from(overflow: bool) -> Self {
        if overflow {
            Self::Overflow
        } else {
            Self::Representable
        }
    }
}
impl From<SelectorRange> for bool {
    fn from(range: SelectorRange) -> Self {
        range == SelectorRange::Overflow
    }
}

/// Positional argument selected by one brace field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BraceArgumentIndex(pub usize);
/// The exact selector path or decimal selector spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BraceSelectorText(pub String);
/// Decoded format-spec text after an explicit colon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BraceSpecText(pub String);
/// Number of fields in one brace-selector category.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BraceFieldCount(pub usize);
