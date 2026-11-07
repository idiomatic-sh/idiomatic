//! The types of the `reporting` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use crate::domain::applications::types::{Plan, SpannedFinding};
use crate::domain::findings::types::{Finding, Fingerprint, Reason, Suppression};
use crate::domain::governance::types::Answers;
use crate::domain::paths::types::RepoPath;

/// What the report is made of: the plan, the lint call's answers, the findings made before it and
/// the directories the report is limited to.
#[derive(Debug)]
pub struct Reported<'a> {
    /// The plan the lint call ran.
    pub plan: &'a Plan,
    /// The lint call's answers, their paths the repository's.
    pub answers: Answers,
    /// The findings made before the lint call.
    pub findings: Vec<Finding>,
    /// The directories the report is limited to; none for the whole workspace.
    pub within: &'a [RepoPath],
    /// What the last check settled, to be used again for a file that is as it was.
    pub reuse: &'a Reuse,
}

/// A finding written as the one text a file's fingerprint chain hashes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IncomingText(pub Box<str>);

/// What the last check settled, for the files that are as they were: what each had before its ignores were applied, as one
/// fingerprint, and the findings those came to. A file whose findings before its ignores are the same is not read again.
#[derive(Debug, Default)]
pub struct Reuse {
    /// What each file had before its ignores were applied.
    pub incoming: std::collections::BTreeMap<RepoPath, Fingerprint>,
    /// The findings the last check kept.
    pub findings: Vec<Finding>,
    /// The findings the last check's ignores accepted.
    pub suppressed: Vec<Suppression>,
}

/// What settling the findings leaves: each finding to be fingerprinted with its standing, the findings of the files not read as they
/// were, and what each file had before its ignores were applied.
#[derive(Debug, Default)]
pub struct Settled {
    /// The findings, sorted.
    pub candidates: Vec<Candidate>,
    /// The findings of the files not read again.
    pub reused: Fingerprinted,
    /// What each file settled had before its ignores were applied.
    pub sourced: std::collections::BTreeMap<RepoPath, Fingerprint>,
}

/// Where findings are collected: those of a whole file, and those of a span of one.
#[derive(Debug)]
pub struct Sinks<'a> {
    /// The findings of a whole file.
    pub whole: &'a mut Vec<Finding>,
    /// The findings of a span of a file, placed once its text is read.
    pub spanned: &'a mut Vec<SpannedFinding>,
}

/// A finding to fingerprint, and the reason an ignore accepts it, when one does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The finding.
    pub finding: Finding,
    /// The source line captured before releasing the file lease.
    pub line: crate::domain::findings::types::FirstLine,
    /// Whether an ignore accepts it.
    pub acceptance: Standing,
}

/// Whether an ignore accepts a finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    /// No ignore accepts it: it stands.
    Stands,
    /// An ignore accepts it, for this reason.
    Accepted(Reason),
}

/// The findings fingerprinted: those that stand, and those an ignore accepts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fingerprinted {
    /// The findings that stand.
    pub standing: Vec<Finding>,
    /// The findings an ignore accepts.
    pub accepted: Vec<Suppression>,
}

/// Why a contract cannot be compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blindness {
    /// No linked language reads its file or its rendered content.
    Unread,
    /// A row the comparison reads came back without its node.
    Dropped,
    /// Its rendered content answered none of the relations the comparison reads.
    Unanswered,
}
