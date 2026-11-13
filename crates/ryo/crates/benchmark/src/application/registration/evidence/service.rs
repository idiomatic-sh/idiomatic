//! The discrepancy evidence a rule's cases declare, staged from its settings profile's directory, or from the bench
//! when the evidence the cases name is the bench's.
use crate::outbound::suite::interface::SuitePort;
use crate::outbound::suite::types::{
    ArtifactPath, ClaimedEvidence, ClaimedProcess, EvidenceArtifact, EvidenceRequirement,
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Stage the discrepancy evidence the rule's cases declare into the stage, from the settings directory when it holds
/// all of it, else from the bench; answer the directory it was staged from.
pub(in super::super) fn stage_evidence(
    suite: &dyn SuitePort,
    bench: &Path,
    settings: &Path,
    rule: &ryo::linter::RuleId,
    stage: &Path,
) -> Result<PathBuf, Error> {
    let claims = suite
        .case_claims(ArtifactPath(bench), std::slice::from_ref(rule))
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let mut declared: BTreeMap<PathBuf, ArtifactDigest> = BTreeMap::new();
    for claim in &claims {
        let mut artifacts = claim
            .discrepancy
            .evidence
            .iter()
            .map(|item| {
                let (ClaimedEvidence::LanguageSpec { path, sha256, .. }
                | ClaimedEvidence::IndependentLanguageRun { path, sha256, .. }
                | ClaimedEvidence::PublicToolContract { path, sha256, .. }
                | ClaimedEvidence::PriorWorkerRun { path, sha256, .. }) = item;
                (path.clone(), ArtifactDigest(sha256.0.clone()))
            })
            .collect::<Vec<_>>();
        if let ClaimedProcess::Failure(failure) = &claim.discrepancy.process {
            for observation in &failure.observations {
                for item in [&observation.invocation, &observation.stdout, &observation.stderr] {
                    artifacts.push((item.path.clone(), item.sha256.clone()));
                }
            }
        }
        for (path, sha256) in artifacts {
            if declared.get(&path).is_some_and(|prior| prior != &sha256) {
                return Err(Error::Invalid(super::super::constants::CONFLICTING_EVIDENCE.into()));
            }
            let _previous = declared.insert(path, sha256);
        }
    }
    let artifacts = declared
        .iter()
        .map(|(path, sha256)| EvidenceArtifact {
            path: path.clone(),
            sha256: sha256.clone(),
            requirement: EvidenceRequirement::Language,
        })
        .collect::<Vec<_>>();
    let root = [settings, bench]
        .into_iter()
        .find(|root| suite.read_discrepancy_artifacts(ArtifactPath(root), &artifacts).is_ok())
        .unwrap_or(settings);
    let _validated = suite
        .read_discrepancy_artifacts(ArtifactPath(root), &artifacts)
        .map_err(|error| Error::Invalid(format!("{}: {error}", super::super::constants::EVIDENCE_REFUSED)))?;
    for path in declared.keys() {
        let target = stage.join(path);
        if suite.digest(ArtifactPath(&target)).is_ok() {
            return Err(Error::Invalid(super::super::constants::EVIDENCE_COLLIDES.into()));
        }
        copy(suite, &root.join(path), &target)?;
    }
    Ok(root.to_owned())
}


/// Copy one file, making its directory first.
pub(in super::super) fn copy(suite: &dyn SuitePort, from: &Path, to: &Path) -> Result<(), Error> {
    if let Some(parent) = to.parent() {
        let _directory = suite.ensure_directory(ArtifactPath(parent))?;
    }
    suite.copy_file(ArtifactPath(from), ArtifactPath(to))
}
