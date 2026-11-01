//! Plain application-owned format parser values.

/// The grammar used to parse a caller-selected format input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormatDialect {
    /// Python printf-style string formatting with a `str` left operand.
    PercentString,
}

/// Why a format input could not be parsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormatErrorKind {
    /// A mapping key opened with `(` but did not close.
    UnmatchedKeyParentheses,
    /// A format ended before completing a conversion.
    IncompleteFormat,
    /// A conversion character is not supported by string percent formatting.
    UnsupportedFormatCharacter,
    /// A fixed width or precision exceeded the parser's accepted integer range.
    WidthOrPrecisionTooLarge,
}

/// The parser error and its position in Unicode scalar units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FormatParseError {
    /// The stable error category.
    pub kind: FormatErrorKind,
    /// The character the parser reported, if the category has one.
    pub character: Option<char>,
    /// The error position in Unicode scalar units.
    pub index: usize,
}

/// Which argument a conversion consumes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FormatArgument {
    /// A positional argument.
    Positional,
    /// A mapping item named by its spelling within the parser input.
    Mapping(String),
}

/// The mode for one width or precision modifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormatQuantityMode {
    /// No modifier.
    None,
    /// A fixed amount appears in the format text.
    Fixed,
    /// An amount is consumed from the positional value tuple.
    Star,
    /// A decimal point appears without a precision amount.
    Dot,
}

/// One parsed conversion field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FormatFieldValue {
    /// Its zero-based order among conversions.
    pub ordinal: usize,
    /// Its argument selection.
    pub argument: FormatArgument,
    /// Its conversion character.
    pub conversion: char,
    /// Width consumption.
    pub width: FormatQuantityMode,
    /// Precision consumption.
    pub precision: FormatQuantityMode,
}

/// Compact consumption facts used beside the individual format fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FormatSummary {
    /// Positional values consumed by conversions and dynamic modifiers.
    pub positional_consumption: usize,
    /// Whether at least one conversion selects a mapping key.
    pub mapping_present: bool,
    /// Whether at least one conversion consumes dynamic width.
    pub dynamic_width: bool,
    /// Whether at least one conversion consumes dynamic precision.
    pub dynamic_precision: bool,
}

/// A successfully parsed format input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParsedFormat {
    /// Conversion fields in their source order.
    pub fields: Vec<FormatFieldValue>,
    /// Aggregate argument-consumption properties.
    pub summary: FormatSummary,
}

/// The owned result of parsing one format input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FormatParseResult {
    /// Parsing completed successfully.
    Parsed(ParsedFormat),
    /// The parser recognized malformed format text.
    Malformed(FormatParseError),
    /// The selected dialect is not implemented by this parser.
    Unavailable,
}

/// A result retained once for one distinct decoded input value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CachedFormat {
    /// The parser's result for this value.
    pub result: FormatParseResult,
}

/// One original string literal and the cached parser value, when it has static decoded text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FormatOccurrence {
    /// The source-tree node for this literal.
    pub node: ryo::languages::Position,
    /// The result in `FormatParseIndex::values`; absent means unknown or unavailable input.
    pub cached: Option<usize>,
    /// Whether the input was dynamic rather than unavailable parser data.
    pub unknown_input: bool,
}

/// Current source occurrences and a separate byte-budgeted content memo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FormatParseIndex {
    /// Budgeted previous-source grammar results; no source-node identities.
    pub memo: crate::domain::grammar_cache::types::Memo<FormatParseResult>,
    /// Input text to one cached result, avoiding duplicate grammar work.
    pub by_input: indexmap::IndexMap<String, usize>,
    /// Parsed or malformed results, one per distinct decoded input.
    pub values: Vec<CachedFormat>,
    /// Source occurrences and their cached values.
    pub occurrences: Vec<FormatOccurrence>,
    /// Cache validity for the current source.
    pub state: FormatParseState,
}

/// Whether the source-local format index is fresh, complete or partial.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FormatParseState {
    /// No format reader has built the index since the last parse.
    #[default]
    Fresh,
    /// Every visible string occurrence and parser result was represented.
    Complete,
    /// A parser input or source occurrence could not be represented.
    Partial,
}
