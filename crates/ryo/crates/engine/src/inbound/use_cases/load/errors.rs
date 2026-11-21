//! Why `load` refuses.

use ryo_language::domain::fixed_names::errors::FixedNamesError;

/// Why a rule set could not be loaded: a pack that is not this rule set's, or a language whose fixed names cannot be words of
/// the run. A rule that cannot be loaded is not an error: `load` answers it among the refused.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::load::errors::LoadError;
///
/// assert_eq!(LoadError::PackCorrupt.to_string(), "pack bytes do not decode");
/// ```
///
/// A vocabulary that states a fixed name twice is refused, naming it:
///
/// ```
/// use ryo_engine::inbound::use_cases::load::errors::LoadError;
/// use ryo_language::domain::fixed_names::errors::FixedNamesError;
/// use ryo_language::domain::fixed_names::types::FixedWord;
///
/// let twice = FixedNamesError::Twice { first: FixedWord(0), again: FixedWord(1) };
/// assert_eq!(
///     LoadError::Vocabulary(twice).to_string(),
///     "the language's fixed names are refused: the fixed name at place 1 is stated before, at place 0",
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    /// The pack was written by another engine version.
    #[error("pack written by another engine version")]
    PackVersion,
    /// The pack was written for other rules.
    #[error("pack written for other rules")]
    PackDigest,
    /// The pack's bytes do not decode.
    #[error("pack bytes do not decode")]
    PackCorrupt,
    /// The language's fixed names cannot be words of the run: one is the empty text, or is stated at two places.
    #[error("the language's fixed names are refused: {0}")]
    Vocabulary(FixedNamesError),
}
