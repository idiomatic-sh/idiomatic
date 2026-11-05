//! Independent retained raw-token parse results and source occurrences.
use crate::domain::format_syntax::types::{
    FormatCacheInput, FormatFieldOrdinal, FormatResultIndex,
};
use crate::domain::format_syntax::types::{FormatParseResult, FormatParseState};
use ryo::languages::Position;

/// One string expression and its consecutive source-token segments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawFormatOccurrence {
    /// Original visible string expression.
    pub node: Position,
    /// First segment's index in the retained vector.
    pub first: RawSegmentIndex,
    /// Index after its last segment.
    pub end: RawSegmentIndex,
    /// All segments were extracted and represented completely.
    pub complete: RawCompleteness,
}

/// One original token; repeated bodies retain distinct source identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawFormatSegment {
    /// Existing string-token identity, including prefixes and quotes in its span.
    pub token: Position,
    /// Zero-based token source order within the enclosing expression.
    pub ordinal: RawTokenOrdinal,
    /// First global field index contributed by this successfully parsed segment.
    pub first_field: FormatFieldOrdinal,
    /// Shared grammar result, or unavailable token text/envelope.
    pub cached: Option<FormatResultIndex>,
}

/// A cache independent of decoded percent and brace grammar demand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawFormatIndex {
    /// Budgeted previous-source grammar results; no source-node identities.
    pub memo: crate::domain::grammar_cache::types::Memo<FormatParseResult>,
    /// Exact unescaped raw bodies, parsed once per distinct input.
    pub by_input: indexmap::IndexMap<FormatCacheInput, FormatResultIndex>,
    /// One complete grammar result per distinct body.
    pub values: Vec<FormatParseResult>,
    /// Each visible source expression, retaining source order.
    pub occurrences: Vec<RawFormatOccurrence>,
    /// Token segments, retaining identity even when their grammar result is shared.
    pub segments: Vec<RawFormatSegment>,
    /// Current source completeness and whether demand has initialized the cache.
    pub state: FormatParseState,
}

/// Position in the retained sequence of original string-token segments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawSegmentIndex(pub usize);
/// Source-token ordinal within one original string expression.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawTokenOrdinal(pub usize);
/// Whether every original segment was observed completely.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RawCompleteness {
    Complete,
    Partial,
}
/// A preserved logical source token, including its envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawFormatToken<'a>(pub &'a str);
/// The exact borrowed body inside a validated original token envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawFormatBody<'a>(pub &'a str);
impl From<RawCompleteness> for bool {
    fn from(value: RawCompleteness) -> Self {
        value == RawCompleteness::Complete
    }
}
