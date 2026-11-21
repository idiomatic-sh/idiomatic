//! The failures of the `fixed_names` concept: a language's fixed names that cannot be base words of a run.

use super::types::FixedWord;

/// Why a language's fixed names were refused: a fixed name's word is its place after the empty text, which holds only when every
/// name is stated once and none is the empty text.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fixed_names::errors::FixedNamesError;
/// use ryo_language::domain::fixed_names::types::FixedWord;
///
/// let twice = FixedNamesError::Twice { first: FixedWord(0), again: FixedWord(2) };
/// assert_eq!(twice.to_string(), "the fixed name at place 2 is stated before, at place 0");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FixedNamesError {
    /// A fixed name is the empty text, which is word 0 of every run and no fixed name's.
    #[error("the fixed name at place {} is the empty text", .place.0)]
    Empty {
        /// The place of the empty name.
        place: FixedWord,
    },
    /// A fixed name is stated at two places: a text has one word, so the second place would have no word of its own.
    #[error("the fixed name at place {} is stated before, at place {}", .again.0, .first.0)]
    Twice {
        /// The place the name is first stated at.
        first: FixedWord,
        /// The place it is stated at again.
        again: FixedWord,
    },
}
