//! The types of the `selection` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use crate::domain::violations::types::{LineNumber, RepoPath, RuleId, ToolId};

/// A glob over paths relative to the product's root.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathPattern(pub Box<str>);

/// A location `check` printed: `path:line`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct At {
    /// The file.
    pub path: RepoPath,
    /// The line.
    pub line: LineNumber,
}

/// The selectors of `up`: each kind left empty selects everything, several of one kind are alternatives, and the
/// kinds combine.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selectors {
    /// The tools.
    pub tools: Vec<ToolId>,
    /// The rules.
    pub rules: Vec<RuleId>,
    /// The path patterns.
    pub paths: Vec<PathPattern>,
    /// The locations.
    pub at: Vec<At>,
}

/// Whether a path matches the path patterns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathMatch {
    /// It does, or there are none.
    Matched,
    /// It does not.
    Unmatched,
}

/// Whether the selectors choose a violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// They do.
    Chosen,
    /// They leave it.
    Left,
}
