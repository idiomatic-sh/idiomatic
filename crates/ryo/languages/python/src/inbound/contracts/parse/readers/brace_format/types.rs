//! The types of the brace-format readers.
use ryo::languages;

/// The stable outcome and error labels of one source string.
#[derive(Clone, Copy)]
pub(crate) struct OutcomeLabels {
    /// How the string parsed.
    pub outcome: languages::NameText<'static>,
    /// The malformed-error category, or `none`.
    pub(crate) error: languages::NameText<'static>,
}

/// The automatic, indexed, keyword and nested-field counts of one parsed string.
#[derive(Clone, Copy)]
pub(crate) struct FieldCounts {
    /// Automatically numbered fields.
    pub(crate) automatic: languages::Integer,
    /// Explicitly indexed fields.
    pub(crate) indexed: languages::Integer,
    /// Keyword fields.
    pub(crate) keyword: languages::Integer,
    /// Fields nested in a format specification.
    pub(crate) nested: languages::Integer,
}
