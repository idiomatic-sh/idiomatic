//! One source-backed standard mini-language observation.
use crate::domain::standard_format_spec::types::StandardSpecOutcome;
use ryo::languages::Position;
/// A decoded specification; no runtime value or custom-format guarantee is inferred.
pub(crate) struct Observation {
    pub id: Position,
    pub ordinal: usize,
    pub parent: Option<usize>,
    pub origin: Position,
    pub source_kind: &'static str,
    /// Unknown for a dynamic f-string specification, never fabricated empty text.
    pub value: Option<String>,
    pub outcome: StandardSpecOutcome,
    /// Exact brace-field selector path; empty for f-string expressions.
    pub selector: String,
    /// Explicit conversion, distinct from the original value protocol.
    pub conversion: String,
}
/// Missing decoded input makes this demanded relation partial.
#[derive(Default)]
pub(crate) struct Observations {
    pub rows: Vec<Observation>,
    pub partial: bool,
}
