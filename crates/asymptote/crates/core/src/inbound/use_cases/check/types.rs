//! The types of the `check` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use std::sync::Arc;

use crate::application::accepted_state::service::AcceptedState;
use crate::domain::judgement::types::{Judgement, Run, Verdict};
use crate::domain::manifest::types::{Groups, ProductRoot};
use crate::domain::operations::types::Revision;
use crate::domain::perf::types::PerfOutcome;
use crate::domain::violations::types::ToolId;
use crate::outbound::bmf::interface::BmfPort;
use crate::outbound::check_store::interface::CheckStorePort;
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::LineFingerprints;
use crate::outbound::lock::interface::LockPort;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::repository::interface::RepositoryPort;
use crate::outbound::sarif::interface::SarifPort;
use crate::outbound::sources::interface::SourcesPort;
use crate::outbound::tools::interface::ToolsPort;

/// What the run is judged against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Against {
    /// The lock in the working tree.
    Lock,
    /// What `<rev>`'s branch accepted: the union of its lock at the merge base with the working tree and at its tip,
    /// so only what this branch introduced is new.
    Since(Revision),
}

/// Whether each tool's fix command runs first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fixing {
    /// It does, before the tools run.
    Fix,
    /// It does not.
    Report,
}

/// What `check` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckRequest {
    /// Where to look for the product: this directory, then its ancestors.
    pub start: ProductRoot,
    /// What it is judged against.
    pub against: Against,
    /// Whether each tool's fix runs first.
    pub fixing: Fixing,
    /// The groups whose tools run; the lock's records of any other tool are not judged.
    pub groups: Groups,
}

/// Whether a tool ran, or its last run was reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reuse {
    /// Its inputs did not move since its last completed run, which is reused.
    Reused,
    /// It ran.
    Ran,
}

/// One tool's run in this check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolRun {
    /// Complete failure streams, when this run retained them.
    pub failure_output: crate::domain::reports::types::FailureOutput,
    /// The run.
    pub run: Run,
    /// Whether it ran now.
    pub reuse: Reuse,
}

/// A file a check read: its lines' fingerprints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadFile {
    /// Each line's fingerprint.
    pub lines: LineFingerprints,
}

/// What `check` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckReport {
    /// The product.
    pub root: ProductRoot,
    /// Each tool's run, in the order they are configured.
    pub runs: Vec<ToolRun>,
    /// What the runs mean against the baseline.
    pub outcome: Judgement,
    /// What they come to.
    pub verdict: Verdict,
    /// What they were judged against.
    pub against: Against,
    /// What the tools' measurements mean against the lock's perf records.
    pub perf: PerfOutcome,
    /// The fix commands that did not succeed, in the order they ran; the check ran on over what they left.
    pub failed_fixes: Vec<FailedFix>,
}

/// A fix command that did not succeed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailedFix {
    /// The tool whose fix it was.
    pub tool: ToolId,
    /// How it failed.
    pub failure: crate::domain::judgement::types::Failure,
}

/// The ports `check` reaches the outside through.
#[derive(Debug)]
pub struct CheckPorts {
    /// The product's `asymptote.toml`.
    pub manifest: Arc<dyn ManifestPort>,
    /// Accepted records and their history, bound to the product's ports.
    pub accepted: AcceptedState,
    /// The tools' processes.
    pub tools: Arc<dyn ToolsPort>,
    /// The tools' SARIF, and the log printed.
    pub sarif: Arc<dyn SarifPort>,
    /// The tools' Bencher Metric Format.
    pub bmf: Arc<dyn BmfPort>,
    /// The lock, which an improved perf record is written to.
    pub lock: Arc<dyn LockPort>,
    /// The files tools report on.
    pub sources: Arc<dyn SourcesPort>,
    /// Fingerprints and digests.
    pub digest: Arc<dyn DigestPort>,
    /// The kept report and fold.
    pub store: Arc<dyn CheckStorePort>,
    /// Git.
    pub repository: Arc<dyn RepositoryPort>,
}
