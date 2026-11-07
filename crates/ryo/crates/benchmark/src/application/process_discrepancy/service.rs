//! Require a completed numeric failure and every declared byte; no adapter error is itself evidence.
use super::errors::ProcessDiscrepancyError as Failure;
use crate::domain::discrepancy::types::{OptionalDigest, ReferenceKind, ReferenceProof};
use crate::domain::process_failure::service::expected;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{CompleteInvocation, InvocationDocument};
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use ryo_benchmark_contracts::domain::documents::types::text::value::YesNo;
use crate::outbound::suite::{
    interface::SuitePort,
    types::{ArtifactMetadata, CaseArtifacts},
};
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use std::path::{Path, PathBuf};

/// The proof and the capture bind the same executable, settings, profile and file metadata.
fn require_identity<'a>(
    invocation: &'a InvocationDocument,
    proof: &ReferenceProof,
) -> Result<&'a CompleteInvocation, Failure> {
    let InvocationDocument::Complete(actual) = invocation else {
        return Err(Failure::IdentityNotValidatedGroup);
    };
    let same_metadata = match (&actual.file_metadata_sha256, &proof.file_metadata) {
        (Pin::Absent, OptionalDigest::Absent) => true,
        (Pin::Sha256(recorded), OptionalDigest::Pinned(pinned)) => recorded.0 == pinned.0,
        _ => false,
    };
    if proof.kind != ReferenceKind::ProcessFailure
        || actual.executable_sha256.0 != proof.executable.0
        || actual.settings_sha256.0 != proof.settings.0
        || actual.profile_sha256.0 != proof.profile.0
        || !same_metadata
    {
        return Err(Failure::IdentityNotValidatedGroup);
    }
    Ok(actual)
}

/// Require a completed numeric failure and every declared byte; no adapter error is itself evidence.
pub(crate) fn matches(
    case: &PreparedCase,
    invocation: &InvocationDocument,
    output: &ProcessOutput,
    proof: &ReferenceProof,
    suite: &dyn SuitePort,
    rules: &Path,
) -> Result<(), Failure> {
    let process = expected(case).ok_or(Failure::NoProcessExpectation)?;
    let actual = require_identity(invocation, proof)?;
    if output.signal != ryo_benchmark_contracts::domain::measurements::types::TerminationSignal::Exited
        || output.status != process.exit
        || actual.signal != Signal::Absent
        || actual.observer_no_deadline != YesNo::Yes
    {
        return Err(Failure::TerminationDiffers);
    }
    let files = case
        .request
        .files
        .iter()
        .map(|file| PathBuf::from(file.0.as_ref()))
        .collect::<Vec<_>>();
    let stdout = crate::domain::process_output::service::expand(
        &process.stdout,
        &case.request.root,
        &files,
    )
    .map_err(|error| error.to_string())?;
    let stderr = crate::domain::process_output::service::expand(
        &process.stderr,
        &case.request.root,
        &files,
    )
    .map_err(|error| error.to_string())?;
    if stdout != output.stdout || stderr != output.stderr {
        return Err(Failure::BytesDiffer);
    }
    let profile_root = rules.parent().ok_or(Failure::MissingProfileBoundary)?;
    let metadata_files = match &case.request.file_metadata {
        ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Declared(values) => values
            .keys()
            .map(|file| PathBuf::from(file.0.as_ref()))
            .collect::<Vec<_>>(),
        ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared => Vec::new(),
    };
    suite
        .verify_discrepancy_artifacts(CaseArtifacts {
            profile: profile_root,
            root: &case.request.root,
            files: &files,
            file_metadata_files: &metadata_files,
            file_metadata: match &proof.file_metadata {
                OptionalDigest::Absent => ArtifactMetadata::Undeclared,
                OptionalDigest::Pinned(digest) => ArtifactMetadata::Declared(digest),
            },
            source_sha256: &proof.source,
            evidence: &[],
        })
        .map_err(|error| Failure::from(error.to_string()))?;
    Ok(())
}
