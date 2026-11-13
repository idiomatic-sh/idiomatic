//! One source-backed standard mini-language observation.
use crate::domain::standard_format_spec::types::StandardSpecOutcome;
use ryo::languages::Position;

/// The place of a specification among the fields of its string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpecOrdinal(pub usize);

/// Which kind of string the specification was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpecSource(pub &'static str);

/// The first byte of a piece of the observations' text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpanStart(pub usize);

/// One past the last byte of a piece of the observations' text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SpanEnd(pub usize);

/// Where a piece of an observation's text is in the observations' one text buffer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ObservedSpan {
    /// The first byte of the piece.
    pub start: SpanStart,
    /// One past its last byte.
    pub end: SpanEnd,
}

/// A piece of text to append to the observations' buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ObservedPiece<'a>(pub &'a str);

/// Every observation's text, which the rows' spans point into.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ObservedText(pub String);

/// The buffer one f-string specification is decoded in before it joins the observations' text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DecodedText(pub String);

/// One interpolation of an f-string, with the f-string that owns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OwnedField {
    /// The f-string.
    pub owner: Position,
    /// The interpolation.
    pub field: Position,
}

/// The decoded text of a specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpecValue(pub ObservedSpan);

/// The exact brace-field selector path of a specification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SelectorPath(pub ObservedSpan);

/// The explicit conversion of a field, as written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ConversionText(pub ObservedSpan);

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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Observations {
    pub rows: Vec<Observation>,
    pub completeness: ObservationCompleteness,
    /// Every row's decoded text, selector and conversion, which the rows' spans point into.
    pub text: ObservedText,
    /// The buffer one f-string specification is decoded in before it joins `text`.
    pub decoded: DecodedText,
    /// The interpolations of the file's f-strings, grouped per owner while the rows are made.
    pub owners: Vec<OwnedField>,
}
