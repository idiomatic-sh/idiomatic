//! What a validator answers over a replayed receipt, and the case whose unselected context stays bound.
use crate::outbound::suite::interface::SuitePort as _;
use crate::outbound::suite::types::ArtifactMetadata;
use crate::outbound::suite::types::ArtifactPath;
use crate::outbound::suite::types::ArtifactSet;
use crate::outbound::suite::types::CaseArtifacts;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::requests::types::StagedSources;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;

/// What a validator answers over a replayed receipt: it admits the receipt or refuses it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The receipt passed.
    Accepted,
    /// The receipt was refused, however it was refused.
    Refused,
}

/// The verdict a result states.
pub(crate) fn verdict<T, E>(result: Result<T, E>) -> Verdict {
    match result {
        Ok(_) => Verdict::Accepted,
        Err(_) => Verdict::Refused,
    }
}

/// A case whose staged context file is no diagnostic input, over the local suite boundary.
pub(crate) struct ContextProbe {
    root: tempfile::TempDir,
    prepared: PreparedCase,
    files: Vec<std::path::PathBuf>,
    digest: ArtifactDigest,
}
impl ContextProbe {
    /// The case `case.py` staged beside the context file `local.py`, both listed.
    pub(crate) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("case.py"), "import local\n").unwrap();
        std::fs::write(root.path().join("local.py"), "value = 1\n").unwrap();
        let mut prepared = crate::domain::discrepancy::fixtures::case();
        prepared.request.root = root.path().into();
        prepared.request.staged_sources = StagedSources::Listed(
            vec!["case.py", "local.py"]
                .into_iter()
                .map(|path| ryo::linter::RelativePath(path.into()))
                .collect(),
        );
        let files = crate::domain::suite::service::materialized_sources(&prepared.request);
        let digest = ArtifactDigest(
            crate::outbound::suite::factory::local()
                .case_source_digest(ArtifactPath(root.path()), ArtifactSet(&files))
                .unwrap()
                .0,
        );
        Self {
            root,
            prepared,
            files,
            digest,
        }
    }
    /// How many files the case selects as diagnostic inputs.
    pub(crate) fn diagnostic_files(&self) -> usize {
        self.prepared.request.files.len()
    }
    /// What the suite boundary answers when it verifies the evidence of the listed sources.
    pub(crate) fn check(&self) -> Verdict {
        verdict(
            crate::outbound::suite::factory::local().verify_discrepancy_artifacts(CaseArtifacts {
                profile: self.root.path(),
                root: self.root.path(),
                files: &self.files,
                source_sha256: &self.digest,
                file_metadata_files: &[],
                file_metadata: ArtifactMetadata::Undeclared,
                evidence: &[],
            }),
        )
    }
    /// Rewrites the context file `local.py`.
    pub(crate) fn write_context(&self, text: &str) {
        std::fs::write(self.root.path().join("local.py"), text).unwrap();
    }
    /// The sources the case materializes once none is listed.
    pub(crate) fn unlisted_sources(&mut self) -> Vec<std::path::PathBuf> {
        self.prepared.request.staged_sources = StagedSources::Unlisted;
        crate::domain::suite::service::materialized_sources(&self.prepared.request)
    }
}
