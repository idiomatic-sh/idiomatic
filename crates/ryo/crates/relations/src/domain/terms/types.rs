//! The types of the `terms` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. Each closed set is an enum, so a language that answers with a word the
//! set does not have does not compile; the word is text only where the answer leaves the engine.

/// Where each closed set's words begin among the numbers a name can have: a word is a name that
/// no file has to write down, so it is found by its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordBlock {
    /// The words of `DeclarationKind`.
    Declaration,
    /// The words of `ParameterKind`.
    Parameter,
    /// The words of `ValueKind`.
    Value,
    /// The words of `MappingKind`.
    Mapping,
    /// The words of `DependencyKind`.
    Dependency,
    /// The reasons a language declares for a value it cannot resolve statically, by their place in its list.
    Unknown,
    /// The words of `DependencyOrigin`.
    Origin,
}

/// The words a column of a catalog relation can hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vocabulary {
    /// Any name a file wrote, or any number.
    Open,
    /// One of a closed set of words, which the block says.
    Closed(WordBlock),
}

/// Why a language cannot say a value of a column: the number of a reason in the closed list the language declares, within the range
/// the language has of the block of reasons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownReason(pub u8);

/// A reason a language declares for a value it cannot resolve statically: its number, which is unique among the languages linked
/// together (each language has a range of its own, in `docs/language-contract.md`), and its name, `wildcard-import`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownName {
    /// The number the symbol of an unknown value of this reason has in the block of reasons.
    pub reason: UnknownReason,
    /// What the reason is called.
    pub text: ReasonName,
}

/// What a reason for an unknown value is called.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReasonName(pub &'static str);

/// A closed set of words.
pub trait Term: Copy {
    /// The block of numbers the set's words are in.
    const BLOCK: WordBlock;

    /// The word's number within its set.
    fn number(self) -> ChildOrdinal;

    /// The word as text, for where it leaves the engine.
    fn text(self) -> NameText<'static>;
}

/// Whether a symbol is a word of a closed set, or a name a file wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wordness {
    /// A word.
    Term,
    /// A name a file wrote.
    Name,
}

/// A name, interned once: the same text is the same symbol everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(pub u32);

/// The place of an element among its parent's children.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChildOrdinal(pub u32);

/// The text of a name, borrowed: the bytes of the file it is in, or the derived text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NameText<'a>(pub &'a str);
