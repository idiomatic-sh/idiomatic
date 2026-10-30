//! The types of the `carrying` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::applications::types::{Carried, GlobText, HandedRule, ParamSet};
use crate::domain::packages::types::Release;

/// Whether two applications of a release can reach one file, so one entry's `exclude` could take what another includes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlap {
    /// No file is reached by two of them: the release can be handed whole.
    Apart,
    /// Some file can be: its files are matched here, entry by entry.
    Shared,
}

/// A rule handed whole, by its release and its params.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HandedKey {
    /// The rule's release, handed as one rule.
    pub release: Release,
    /// The params its entries pass.
    pub params: ParamSet,
}

/// A rule's `include` and `exclude` globs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleGlobs {
    /// The files it reads.
    pub include: Vec<GlobText>,
    /// The files of `include` it does not read.
    pub exclude: Vec<GlobText>,
}

/// The applications of the releases handed whole, as rules, and those whose files are matched here.
#[derive(Clone, Debug, Default)]
pub struct CarriedSplit<'w> {
    /// The rules handed whole.
    pub handed: Vec<HandedRule>,
    /// The applications matched here.
    pub matched: Vec<Carried<'w>>,
}
