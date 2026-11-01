//! Independent retained raw-token parse results and source occurrences.
use crate::domain::format_syntax::types::{FormatParseResult, FormatParseState};
use ryo::languages::Position;

/// One string expression and its consecutive source-token segments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawFormatOccurrence {
    /// Original visible string expression.
    pub node: Position,
    /// First segment's index in the retained vector.
    pub first: usize,
    /// Index after its last segment.
    pub end: usize,
    /// All segments were extracted and represented completely.
    pub complete: bool,
}

/// One original token; repeated bodies retain distinct source identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawFormatSegment {
    /// Existing string-token identity, including prefixes and quotes in its span.
    pub token: Position,
    /// Zero-based token source order within the enclosing expression.
    pub ordinal: usize,
    /// First global field index contributed by this successfully parsed segment.
    pub first_field: usize,
    /// Shared grammar result, or unavailable token text/envelope.
    pub cached: Option<usize>,
}

/// A cache independent of decoded percent and brace grammar demand.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RawFormatIndex {
    /// Exact unescaped raw bodies, parsed once per distinct input.
    pub by_input: std::collections::HashMap<String, usize>,
    /// One complete grammar result per distinct body.
    pub values: Vec<FormatParseResult>,
    /// Each visible source expression, retaining source order.
    pub occurrences: Vec<RawFormatOccurrence>,
    /// Token segments, retaining identity even when their grammar result is shared.
    pub segments: Vec<RawFormatSegment>,
    /// Current source completeness and whether demand has initialized the cache.
    pub state: FormatParseState,
}
