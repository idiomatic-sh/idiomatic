//! The types of the brace-format readers.
use ryo::languages;

/// The stable outcome and error labels of one source string.
#[derive(Clone, Copy)]
pub(crate) struct OutcomeLabels {
    /// How the string parsed.
    pub outcome: languages::NameText<'static>,
    /// The malformed-error category, or `none`.
    pub error: languages::NameText<'static>,
}

/// The automatic, indexed, keyword and nested-field counts of one parsed string.
#[derive(Clone, Copy)]
pub(crate) struct FieldCounts {
    /// Automatically numbered fields.
    pub automatic: languages::Integer,
    /// Explicitly indexed fields.
    pub indexed: languages::Integer,
    /// Keyword fields.
    pub keyword: languages::Integer,
    /// Fields nested in a format specification.
    pub nested: languages::Integer,
}
