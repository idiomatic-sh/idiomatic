//! The types of the percent-format readers.
use crate::domain::format_syntax::types::{FormatCharacter, FormatSummary};
use ryo::languages;

/// The parse summary of one string and its stable outcome label.
#[derive(Clone, Copy)]
pub(crate) struct Summarized {
    /// What the conversions consume and select.
    pub summary: FormatSummary,
    /// How the string parsed.
    pub outcome: languages::NameText<'static>,
}

/// The error category, character and scalar index of one malformed string, or empty defaults.
#[derive(Clone, Copy)]
pub(crate) struct ErrorParts {
    /// The error category, or `none`.
    pub category: languages::NameText<'static>,
    /// The offending character, when the error names one.
    pub character: Option<FormatCharacter>,
    /// The scalar index of the error.
    pub index: languages::Integer,
}
