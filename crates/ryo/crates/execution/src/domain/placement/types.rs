//! The types of the `placement` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. Placement says which rules govern which files, and it is decided once for
//! the run: one set of globs for every rule, one bit for every rule.

use ryo_values::domain::files::types::Glob;
use ryo_values::domain::runs::types::RuleNumber;

use ryo_values::domain::files::types::GlobsMatcher;

/// Which files a rule governs: the globs it includes, and the globs it leaves out. A rule that
/// includes nothing governs every file the globs it leaves out do not match.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleScope {
    /// The globs of the files the rule governs.
    pub include: Vec<Glob>,
    /// The globs of the files it does not.
    pub exclude: Vec<Glob>,
}

/// Which side of a rule's scope a glob is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobSide {
    /// The glob names files the rule governs.
    Include,
    /// The glob names files the rule does not govern.
    Exclude,
}

/// Whose glob a glob of the set is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobOwner {
    /// The rule.
    pub rule: RuleNumber,
    /// Its side.
    pub side: GlobSide,
}

/// A bit for each rule, set when the rule governs the file: sixty-four rules to a word.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mask(pub Vec<u64>);

/// Where a rule's bit is in a mask: the word it is in, and the bit within the word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Bit {
    /// The word.
    pub word: WordIndex,
    /// The word with only the rule's bit set.
    pub mask: BitMask,
}

/// A word of a mask, by its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WordIndex(pub usize);

/// A word with only some bits set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BitMask(pub u64);

/// Whether a rule governs a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Governing {
    /// It does.
    Governs,
    /// It does not.
    DoesNotGovern,
}

/// The scopes of every rule of a run, decided once: the globs to make ready, whose each is whose,
/// and the rules that govern every file their excluded globs do not match.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScopePlacement {
    /// Every glob of every rule, each once, in one list, so a path is matched against all of them at once.
    pub globs: Vec<Glob>,
    /// Whose glob each glob of the list is, by its place in the list: a glob several rules write is each one's.
    pub owners: Vec<Vec<GlobOwner>>,
    /// The rules that include nothing.
    pub always: Mask,
}

/// The scopes of every rule, with their globs made ready by the `matching` port.
#[derive(Clone, Debug)]
pub struct PlacedScopes {
    /// What was decided of the scopes.
    pub placement: ScopePlacement,
    /// The globs, ready.
    pub matcher: GlobsMatcher,
}

/// The text of a glob, by which the globs of the rules are told apart.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GlobKey(pub Box<str>);
