//! The types of the `kind_check` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::descriptions::types::KindWord;
use ryo_values::domain::plan_parts::types::Test;

/// A word a rule writes for a kind: what it is compared with, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AskedKind {
    /// How the kind is compared with the word: a name, or a pattern.
    pub test: Test,
    /// The word, or the pattern.
    pub word: KindWord,
}

/// Whether some language has a kind that satisfies a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KindMatch {
    /// A kind of a language does.
    Names,
    /// No kind of any language does.
    Nothing,
}

/// The place of a rule among the rules of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RulePlace(pub usize);

/// A word a rule writes for a kind that no linked language has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnknownKind {
    /// The rule that writes it.
    pub rule: RulePlace,
    /// The word.
    pub word: KindWord,
}

/// What a run is refused with for a word that names no kind: the word, and the languages that were looked in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindRefusal(pub Box<str>);

/// The names of the linked languages, as one phrase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LanguageList(pub Box<str>);
