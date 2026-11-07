//! The types of the `sarif` port: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::judgement::types::Inventory;
use crate::domain::violations::types::{
    Fingerprinted, Level, MessageText, Place, RepoPath, RuleId, Suppression,
};

/// A SARIF log's text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SarifText(pub Box<str>);

/// A coordinate of a result's region, one-based.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Coordinate(pub u32);

/// Whether a coordinate was given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bound {
    /// This one.
    Given(Coordinate),
    /// None.
    Absent,
}

/// Where a result's region ends, as its tool wrote it: two results that differ only there are two results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct End {
    /// Its last line.
    pub line: Bound,
    /// Its last column.
    pub column: Bound,
}

/// One located result of a log.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reported {
    /// Its file, normalized: relative as the tool wrote it, or absolute.
    pub path: RepoPath,
    /// Where in it.
    pub place: Place,
    /// Where its region ends.
    pub end: End,
    /// Its rule.
    pub rule: RuleId,
    /// Its level.
    pub level: Level,
    /// Whether an accepted in-source suppression covers it, and its justification.
    pub suppression: Suppression,
    /// Its message.
    pub message: MessageText,
    /// Its fingerprint when its place gives it one: the whole file's, for a result with no region; unknown
    /// otherwise, for the line's to be read.
    pub fingerprint: Fingerprinted,
    /// The `primaryLocationLineHash` its tool gave it, without an occurrence suffix: never its identity, which a tool
    /// hashing a path in would break when a file moves, but the one used when its file cannot be read.
    pub supplied: Fingerprinted,
}

/// What a log says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    /// Its located results, each once.
    pub results: Vec<Reported>,
    /// The rules its runs say they evaluated.
    pub inventory: Inventory,
}

/// A path as a URI carries it, percent-encoded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Uri(pub Box<str>);
