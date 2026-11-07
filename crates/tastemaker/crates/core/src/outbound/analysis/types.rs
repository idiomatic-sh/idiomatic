//! The types of the `analysis` port: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::applications::types::{PlacedRow, PlannedRule};
use crate::domain::contracts::types::RenderedContent;
use crate::domain::files::types::MachinePath;
use crate::domain::paths::types::RepoPath;

/// How the ryo adapter runs every lint call: the invocation's one pool it reads files on, and the most memory it may use.
/// The edge sets them once, beside where the adapter keeps its work, which its factory takes.
// ignore: idiomatic.arch/port/plain-port-types - the settings hold the invocation's thread pool, which has no value to compare: two pools are the same only by identity, and a contract row compares requests and answers, never settings
#[derive(Clone, Debug)]
pub struct AnalysisSettings {
    /// The invocation's one pool, the scheduler's own, which ryo's engine reads files on.
    pub pool: crate::outbound::scheduler::types::WorkerPool, // ignore: idiomatic.arch/import-directions - the invocation runs on one pool, built once by the edge and shared by the scheduler and ryo's engine: the analysis settings carry the scheduler's own pool rather than a second pool or a copy of its type
    /// The most memory a call may use.
    pub memory: Memory,
}

/// The most memory a lint call may use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Memory {
    /// What the machine has.
    Machine,
    /// This many bytes.
    Bytes(MemoryBytes),
}

/// An amount of memory, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryBytes(pub u64);

/// Which files a lint call reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Corpus {
    /// Every file below the root the repository does not ignore: a case's scratch directory.
    Walked,
    /// These files, relative to the root: the files the workspace governs, so ryo walks nothing and sees no other file.
    Listed(Vec<RepoPath>),
    /// Only these files, given as text: a scenario's project, written nowhere. The root names nothing then.
    Given(Vec<GivenFile>),
}

/// One lint call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisRequest {
    /// The directory the call reads.
    pub root: MachinePath,
    /// The rules, as the plan holds them.
    pub rules: Vec<PlannedRule>,
    /// The rows of `tastemaker.placed`.
    pub placed: Vec<PlacedRow>,
    /// Which files it reads.
    pub corpus: Corpus,
    /// Files given as text, read beside the corpus: the rendered contents the contracts compare.
    pub beside: Vec<GivenFile>,
    /// The relations tm hosts beside `tastemaker.placed`, for the queries of a verdict or a count call.
    pub hosted: Hosted,
}

/// The relations a lint call hosts beside `tastemaker.placed`: none for a call over the plan's rules, else those of
/// the one call that judges what the plan's call answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hosted {
    /// None: the call runs the plan's rules.
    Nothing,
    /// The scenarios' findings and expectations, for the verdict queries: the relations `tastemaker.expected`,
    /// `tastemaker.found`, `tastemaker.only`, `tastemaker.own` and `tastemaker.evaluated`.
    Verdict(crate::domain::scenario_judging::types::VerdictRows),
    /// The counts and pairs a check left waiting, for the count queries: the relations `tastemaker.template-entry` and
    /// `tastemaker.condition-answer`.
    Counts(crate::domain::judging::types::CountRows),
}

/// A file given as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GivenFile {
    /// Its path, relative to the root.
    pub path: RepoPath,
    /// Its text.
    pub text: RenderedContent,
}

/// What a lint call answers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AnalysisReport {
    /// The rows the rules selected, each file relative to the lint call's root.
    pub findings: Vec<crate::domain::applications::types::LintAnswer>,
    /// What they could not decide.
    pub unknowns: Vec<crate::domain::applications::types::LintUnknown>,
    /// The files no linked language read.
    pub unread: Vec<RepoPath>,
}

/// Whether a query is one ryo can run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryVerdict {
    /// It is.
    Valid,
    /// It is not, and why.
    Invalid(InvalidReason),
}

/// Why a query is not one ryo can run, as the engine said it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidReason(pub Box<str>);

impl std::fmt::Display for InvalidReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
