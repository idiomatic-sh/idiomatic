//! What a certified measurement remembers from before its paired run.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use std::path::PathBuf;

/// The sources a run reads and their content identity before the contestants touch them.
#[derive(Clone, Debug)]
pub struct RunBaseline {
    /// Every source path of the request, under its root.
    pub sources: Vec<PathBuf>,
    /// The content identity of those sources before the run.
    pub before: ArtifactDigest,
}

/// The rules a pair is judged on, and the reference's rule for a source its parser rejects.
#[derive(Clone, Copy, Debug)]
pub(crate) struct JudgedRules<'a> {
    /// The rules both contestants ran.
    pub selected: &'a [ryo::linter::RuleId],
    /// The reference's invalid-syntax rule.
    pub syntax: &'a ryo::linter::RuleId,
}

/// The corpus sources read so far, each read once when a finding in it is first placed.
#[derive(Debug, Default)]
pub(crate) struct SourceTexts(
    /// Each source read, by its corpus-relative path.
    pub std::collections::BTreeMap<ryo::linter::RelativePath, ryo::linter::SourceText>,
);

/// The pair a corpus is measured with, and what their runs go through.
#[derive(Clone, Copy)]
pub(crate) struct Contestants<'a> {
    /// The pinned reference linter.
    pub reference: &'a dyn crate::outbound::linter::interface::LinterPort,
    /// The candidate linter.
    pub candidate: &'a dyn crate::outbound::linter::interface::LinterPort,
    /// Process launch and observation.
    pub execution: &'a dyn crate::outbound::execution::interface::ExecutionPort,
    /// Sealed project context.
    pub context: &'a dyn crate::outbound::context::interface::ContextPort,
    /// Artifact validation and fingerprinting.
    pub suite: &'a dyn crate::outbound::suite::interface::SuitePort,
}
impl std::fmt::Debug for Contestants<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The certificate both contestants hold, and how the run is accounted.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Certification<'a> {
    /// The certificate both contestants hold for this corpus.
    pub certificate: &'a crate::domain::suite::types::certificate::value::Certificate,
    /// Whether the run is accounted or diagnostic.
    pub mode: &'a ryo_benchmark_contracts::domain::measurements::types::MeasurementMode,
}
