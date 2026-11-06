//! One source-backed standard mini-language observation.
use crate::domain::standard_format_spec::types::StandardSpecOutcome;
use ryo::languages::Position;

/// The place of a specification among the fields of its string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpecOrdinal(pub usize);

/// Which kind of string the specification was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpecSource(pub &'static str);

/// The decoded text of a specification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SpecValue(pub String);

/// The exact brace-field selector path of a specification.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SelectorPath(pub String);

/// The explicit conversion of a field, as written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ConversionText(pub String);

impl std::ops::Deref for SpecValue {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for SelectorPath {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for ConversionText {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

/// Whether every input the observations are read from was decoded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ObservationCompleteness {
    /// Every input was decoded.
    #[default]
    Whole,
    /// Some input could not be decoded.
    Partial,
}

/// A decoded specification; no runtime value or custom-format guarantee is inferred.
pub(crate) struct Observation {
    pub id: Position,
    pub ordinal: SpecOrdinal,
    pub parent: Option<SpecOrdinal>,
    pub origin: Position,
    pub source_kind: SpecSource,
    /// Unknown for a dynamic f-string specification, never fabricated empty text.
    pub value: Option<SpecValue>,
    pub outcome: StandardSpecOutcome,
    /// Exact brace-field selector path; empty for f-string expressions.
    pub selector: SelectorPath,
    /// Explicit conversion, distinct from the original value protocol.
    pub conversion: ConversionText,
}

/// Missing decoded input makes this demanded relation partial.
#[derive(Default)]
pub(crate) struct Observations {
    pub rows: Vec<Observation>,
    pub completeness: ObservationCompleteness,
}
