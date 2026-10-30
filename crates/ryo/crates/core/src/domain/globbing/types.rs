//! The types of the `globbing` concept: plain data, public fields and derives, with no constructor and no inherent impl. A glob and the
//! text it is matched against, read a byte and a character at a time.

/// A pattern read as three pieces one after the other: what precedes an alternative, the alternative, what follows it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Alternated<'a> {
    /// What precedes the alternative.
    pub before: GlobPiece<'a>,
    /// The alternative.
    pub alternative: GlobPiece<'a>,
    /// What follows it.
    pub after: GlobPiece<'a>,
}

/// One piece of a pattern.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GlobPiece<'a>(pub &'a str);

/// A place in a pattern or a text, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GlobIndex(pub usize);

/// Whether a class is negated: `[!a-z]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Negation {
    /// The class takes the characters it lists.
    Plain,
    /// The class takes the characters it does not list.
    Negated,
}

/// Where a class closes, and whether it is negated.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClassClose {
    /// Where its `]` is.
    pub close: GlobIndex,
    /// Whether it is negated.
    pub negation: Negation,
}

/// Where a class opens and closes, and whether it is negated.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClassBounds {
    /// Where its `[` is.
    pub open: GlobIndex,
    /// Where its `]` is.
    pub close: GlobIndex,
    /// Whether it is negated.
    pub negation: Negation,
}

/// The text a glob is matched against.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Subject<'a>(pub &'a str);

/// The text of a glob.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlobText<'a>(pub &'a str);

/// A character of the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Letter(pub char);

/// A byte of a pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Byte(pub u8);

/// Whether the ends of a pattern rule a text out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ruling {
    /// The text cannot match.
    RulesOut,
    /// The text may match.
    Leaves,
}

/// Whether a glob matches a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GlobMatch {
    /// All of the text matches.
    Matches,
    /// The text does not match.
    Differs,
}

/// A set of patterns made ready for repeated text comparisons.
#[derive(Debug)]
pub(crate) enum CompiledGlobs {
    /// The library has exactly the text matcher semantics for these patterns.
    Ready(regex_automata::meta::Regex),
    /// These patterns retain the interpreter's character, escape or directory semantics.
    Interpreted,
}
