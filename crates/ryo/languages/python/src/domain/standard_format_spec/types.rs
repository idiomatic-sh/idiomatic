//! A standard mini-language observation; custom `__format__` may interpret the same text differently.

/// Recognition of one complete decoded format specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StandardSpecOutcome {
    /// Every static clause belongs to the target version's standard mini-language.
    Recognized,
    /// Nested replacement fields defer final interpretation until their values are substituted.
    Dynamic,
    /// The first candidate presentation type is absent from the target's standard grammar.
    InvalidType(char),
    /// Another structural error prevents an invalid-type conclusion.
    OtherError,
}
