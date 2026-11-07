//! The types of the `kind_check` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::descriptions::types::KindWord;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::query_tokens::types::QueryOrigin;

/// A word a rule writes for a kind: what it is compared with, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskedKind {
    /// How the kind is compared with the word: a name, or a pattern.
    pub test: Test,
    /// The word, or the pattern.
    pub word: KindWord,
    /// The original fragment source, when this comparison came from a linked module.
    pub origin: KindOrigin,
}

/// Whether some language has a kind that satisfies a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KindMatch {
    /// A kind of a language does.
    Names,
    /// No kind of any language does.
    Nothing,
}

/// The place of a rule among the rules of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RulePlace(pub usize);

/// A word a rule writes for a kind that no linked language has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownKind {
    /// The rule that writes it.
    pub rule: RulePlace,
    /// The word.
    pub word: KindWord,
    /// The original fragment source, when available.
    pub origin: KindOrigin,
}

/// Where a word a rule writes for a kind was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KindOrigin {
    /// The word came from the recorded source of a linked module.
    Recorded(QueryOrigin),
    /// No source of the word is recorded.
    Unrecorded,
}

/// What is said of a word that names no kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindMessage(pub Box<str>);

/// What a run is refused with for a word that names no kind: its message and where the word was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KindRefusal {
    /// What is said of the word.
    pub message: KindMessage,
    /// Where the word was written.
    pub origin: KindOrigin,
}

/// The names of the linked languages, as one phrase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageList(pub Box<str>);
