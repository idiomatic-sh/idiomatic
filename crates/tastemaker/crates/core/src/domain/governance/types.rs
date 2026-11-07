//! The types of the `governance` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::sync::Arc;

use crate::domain::applications::types::{LintAnswer, LintRuleId, LintUnknown};
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::types::PathSet;

/// Whether a rule governs a file, or any file the lint call reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Governance {
    /// It reads the file, or at least one of them.
    Governs,
    /// It reads none: it has nothing to find there.
    Idle,
}

/// A rule's answers and what it could not decide, as the lint call gave them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Answers {
    /// The rows it answered.
    pub answers: Vec<LintAnswer>,
    /// What it could not decide.
    pub unknowns: Vec<LintUnknown>,
    /// The files no linked language read.
    pub unread: Vec<RepoPath>,
}

/// A rule's `include` and `exclude` compiled, to tell the files it governs.
#[derive(Clone)]
pub struct Governing {
    /// The files it reads.
    pub include: Inclusion,
    /// The files of those it does not read.
    pub exclude: Arc<dyn PathSet>,
}

impl std::fmt::Debug for Governing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The files a rule reads.
#[derive(Clone)]
pub enum Inclusion {
    /// Every file: it names no `include`.
    Every,
    /// Those its `include` passes.
    Only(Arc<dyn PathSet>),
}

impl std::fmt::Debug for Inclusion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// One rule's answer about one file, by which it is decided once.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Decision {
    /// The rule of the call.
    pub rule: LintRuleId,
    /// The file the answer is about.
    pub path: RepoPath,
}
