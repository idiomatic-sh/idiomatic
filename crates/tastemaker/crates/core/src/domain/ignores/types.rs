//! The types of the `ignores` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! An in-source ignore, `ignore: <package/rule> - <reason>` in a line comment, accepts the findings of one rule where
//! it is written.

use crate::domain::comments::types::{ByteSpan, LineIndex};
use crate::domain::findings::types::{Finding, MessageText, Reason, RuleId, Suppression};

/// What an ignore comment says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Written {
    /// A rule and a reason.
    Ignore {
        /// The rule's id.
        rule: RuleId,
        /// Why its findings here are accepted.
        reason: Reason,
    },
    /// `ignore:` with no rule before the separator.
    NoRule,
    /// A rule and no reason, or an empty one.
    NoReason {
        /// The rule's id.
        rule: RuleId,
    },
}

/// What an ignore covers: its rule's findings in the whole file, those whose first line of code is within a range, or
/// nothing when no code follows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Covered {
    /// The whole file.
    File,
    /// These lines.
    Lines {
        /// The first.
        first: LineIndex,
        /// The last.
        last: LineIndex,
    },
    /// No code follows it.
    Nothing,
}

/// One usable ignore: where it is, its rule and reason, what it covers, and whether it covered a finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ignore {
    /// The comment's span.
    pub at: ByteSpan,
    /// The rule it names.
    pub rule: RuleId,
    /// Why.
    pub reason: Reason,
    /// What it covers.
    pub reach: Covered,
    /// Whether it covered a finding.
    pub used: Usage,
}

/// Whether an ignore covered a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Usage {
    /// Not yet.
    Unused,
    /// It did.
    Used,
}

/// One malformed or misplaced ignore, as a finding says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadIgnore {
    /// The comment's span.
    pub at: ByteSpan,
    /// What is wrong.
    pub message: MessageText,
}

/// Whether a reach covers a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Covering {
    /// It does.
    Covers,
    /// It does not.
    Misses,
}

/// What a file's comments write: the usable ignores and what is wrong with the others.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadIgnores {
    /// The usable ignores.
    pub ignores: Vec<Ignore>,
    /// The malformed ones.
    pub problems: Vec<BadIgnore>,
}

/// A file's ignores applied to its findings: those no ignore covers, those one does, and the ignores that suppress
/// nothing or are malformed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sifted {
    /// The findings kept.
    pub kept: Vec<Finding>,
    /// The findings suppressed, with their reasons.
    pub suppressed: Vec<Suppression>,
    /// The malformed and unused ignores.
    pub problems: Vec<BadIgnore>,
}

/// What the ignores of a run's files do to its findings: the findings kept, those suppressed, and the lines of each
/// file read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Suppressed {
    /// The findings kept, the problems of the ignores among them.
    pub kept: Vec<Finding>,
    /// The findings suppressed, with their reasons.
    pub suppressed: Vec<Suppression>,
}
