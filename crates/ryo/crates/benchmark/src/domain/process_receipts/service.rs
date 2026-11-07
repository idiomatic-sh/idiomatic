//! Reconstruct expected bytes from registration and actual request roots, never case_passed alone.
use super::errors::ProcessReceiptsError as Failure;
use crate::domain::canonical_text::{service::text_digest, types::Document};
use crate::domain::invocation_identity::{
    service::normalize_text,
    types::{Boundaries, InvocationText},
};
use crate::domain::process_failure::service::{group_digest, output_digest};
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{ArgumentText, CompleteInvocation, EnvironmentName, EnvironmentValue, InvocationDocument};
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseDocument;
use ryo_benchmark_contracts::domain::workloads::types::CaseComparison;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::text::value::{ExitStatus, FileMetadataDigest, PathText, Sha256Text, YesNo};
use ryo_benchmark_contracts::domain::documents::types::case_document::value::CaseReference;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
    IntentMatch, ObservationDocument, ProcessArtifactsRecord, ProcessRecord, RecordedMetadata, RecordedReason,
    ReferenceProcess, ReportedFindings, ReportedSkips, RequestRoot, ValidationRecord,
};
use ryo_benchmark_contracts::domain::reports::types::CoverageAccounting;
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::{
    ArtifactPointer, DiscrepancyRecord, DiscrepancyValidation, ExpectationKind, ProcessExpectation,
};
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::process::types::ProcessFailure;
use sha2::Digest as _;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn raw<'a>(
    artifacts: &'a BTreeMap<PathBuf, CapturedBytes>,
    pointer: &ArtifactPointer,
) -> Result<&'a CapturedBytes, Failure> {
    let bytes = artifacts
        .get(Path::new(&pointer.path.0))
        .ok_or(Failure::MissingCapturedProcessBytes)?;
    if pointer.sha256.0 != format!("{:x}", sha2::Sha256::digest(&bytes.0)) {
        return Err(Failure::CapturedByteDigestDiffers);
    }
    Ok(bytes)
}

fn pin(value: &FileMetadataDigest) -> Pin {
    match value {
        FileMetadataDigest::Declared(digest) => Pin::Sha256(digest.clone()),
        FileMetadataDigest::Undeclared => Pin::Absent,
    }
}

/// The documented discrepancy a case registers, when it differs from the reference.
fn differing(case: &CaseDocument) -> Option<&DiscrepancyRecord> {
    match &case.reference_discrepancy {
        CaseReference::Differs(record) => Some(record),
        CaseReference::Agrees => None,
    }
}

/// The process a reference side was observed to run instead of reporting.
fn observed(reference: &ObservationDocument) -> Option<&ProcessRecord> {
    match &reference.process_observation {
        ReferenceProcess::Observed(record) => Some(record),
        ReferenceProcess::NotObserved => None,
    }
}

/// The validation a reference side attests.
fn attested(reference: &ObservationDocument) -> Option<&DiscrepancyValidation> {
    match &reference.discrepancy_validation {
        ValidationRecord::Attested(proof) => Some(proof),
        ValidationRecord::Unattested => None,
    }
}

/// The reference side is the explicit process variant, terminated exactly as registered.
fn require_reference_variant(
    case: &CaseDocument,
    reference: &ObservationDocument,
    process: &ProcessFailure,
    executable: &ArtifactDigest,
) -> Result<(), Failure> {
    let (Some(certificate), Some(captured), Some(proof)) =
        (differing(case), observed(reference), attested(reference))
    else {
        return Err(Failure::ReferenceVariantDiffers);
    };
    let identity = &certificate.identity;
    let pinned = Sha256Text(executable.0.clone());
    let template = Sha256Text(output_digest(process).0);
    let group = Sha256Text(group_digest(process, executable)?.0);
    if case.comparison != CaseComparison::Full
        || proof.reference_expectation_kind != ExpectationKind::ProcessFailure
        || captured.kind != ExpectationKind::ProcessFailure
        || captured.metadata_error != RecordedReason::Absent
        || captured.observer_no_deadline != YesNo::Yes
        || captured.exit != ExitStatus(i64::from(process.exit.0))
        || captured.signal != Signal::Absent
        || captured.executable_sha256 != Pin::Sha256(pinned)
        || captured.settings_sha256 != Pin::Sha256(identity.settings.clone())
        || captured.template_sha256 != Pin::Sha256(template)
        || proof.process_observation_group_sha256 != Pin::Sha256(group)
        || reference.diagnostics != ReportedFindings::Unreported
        || reference.skipped != ReportedSkips::Unreported
        || reference.coverage != CoverageAccounting::Unaccounted
        || reference.execution_metadata != RecordedMetadata::Unrecorded
        || reference.raw_parity_matches_intended != IntentMatch::Matched(YesNo::No)
    {
        return Err(Failure::ReferenceVariantDiffers);
    }
    Ok(())
}

/// The raw invocation record states every identity the registration and the receipt bind.
fn require_invocation(
    case: &CaseDocument,
    reference: &ObservationDocument,
    invocation: &CompleteInvocation,
    root: &PathText,
    executable: &ArtifactDigest,
) -> Result<(), Failure> {
    let (Some(certificate), Some(captured)) = (differing(case), observed(reference)) else {
        return Err(Failure::ReferenceVariantDiffers);
    };
    let identity = &certificate.identity;
    if invocation.file_metadata_sha256 != pin(&case.file_metadata_sha256) {
        return Err(Failure::InvocationFileMetadataDiffers);
    }
    if invocation.tool != identity.tool {
        return Err(Failure::InvocationToolDiffers);
    }
    if invocation.version != identity.version {
        return Err(Failure::InvocationVersionDiffers);
    }
    if invocation.actual_request_root != *root {
        return Err(Failure::InvocationRootDiffers);
    }
    if invocation.files != case.files {
        return Err(Failure::InvocationFilesDiffer);
    }
    if invocation.source_sha256 != Pin::Sha256(case.source_sha256.clone()) {
        return Err(Failure::InvocationSourceDiffers);
    }
    if invocation.executable_sha256.0 != executable.0 {
        return Err(Failure::InvocationExecutableDiffers);
    }
    if invocation.settings_sha256 != identity.settings {
        return Err(Failure::InvocationSettingsDiffer);
    }
    if invocation.profile_sha256 != identity.profile {
        return Err(Failure::InvocationProfileDiffers);
    }
    if invocation.exit != captured.exit {
        return Err(Failure::InvocationExitDiffers);
    }
    if invocation.signal != Signal::Absent {
        return Err(Failure::InvocationSignalDiffers);
    }
    if invocation.observer_no_deadline != YesNo::Yes {
        return Err(Failure::InvocationObserverDiffers);
    }
    Ok(())
}

/// Raw argv, environment and directory establish the pinned settings again, without trusting precomputed digests.
fn require_settings(case: &CaseDocument, invocation: &CompleteInvocation) -> Result<(), Failure> {
    let identity = differing(case)
        .map(|certificate| &certificate.identity)
        .ok_or(Failure::ReferenceVariantDiffers)?;
    let pinned = Sha256Text(
        text_digest(&Document(
            invocation.normalized_settings.canonical.0.clone(),
        ))
        .0,
    );
    if pinned != identity.settings {
        return Err(Failure::MissingPinnedNormalizedInvocation);
    }
    if invocation.state.0.is_empty() {
        return Err(Failure::RawStateMissing);
    }
    let boundaries = Boundaries {
        state: InvocationText(invocation.state.0.clone()),
        root: InvocationText(invocation.actual_request_root.0.clone()),
    };
    let spell = |value: &InvocationText| normalize_text(value, &boundaries).0;
    let arguments = invocation
        .arguments
        .iter()
        .map(|argument| ArgumentText(spell(&InvocationText(argument.0.clone()))))
        .collect::<Vec<_>>();
    let environment = invocation
        .environment
        .iter()
        .map(|(name, value)| {
            (
                EnvironmentName(spell(&InvocationText(name.0.clone()))),
                EnvironmentValue(spell(&InvocationText(value.0.clone()))),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let settings = &invocation.normalized_settings;
    if arguments != settings.arguments
        || environment != settings.environment
        || spell(&InvocationText(invocation.directory.0.clone())) != settings.directory.0
    {
        return Err(Failure::RawSettingsNotEstablished);
    }
    Ok(())
}

/// Validate only the explicit reference process variant; candidate diagnostics are checked by the caller.
pub(crate) fn validate(
    case: &CaseDocument,
    reference: &ObservationDocument,
    artifacts: &BTreeMap<PathBuf, CapturedBytes>,
    invocations: &BTreeMap<PathBuf, InvocationDocument>,
) -> Result<(), Failure> {
    let Some(ProcessExpectation::Failure(process)) = differing(case).map(|certificate| &certificate.process)
    else {
        return Err(Failure::ProcessRegistrationMissing);
    };
    let process = process.clone();
    let (Some(captured), Some(proof)) = (observed(reference), attested(reference)) else {
        return Err(Failure::ReferenceVariantDiffers);
    };
    let executable = ArtifactDigest(proof.actual_executable_sha256.0.clone());
    require_reference_variant(case, reference, &process, &executable)?;
    let identity = &differing(case)
        .ok_or(Failure::ReferenceVariantDiffers)?
        .identity;
    if identity.file_metadata != case.file_metadata_sha256
        || proof.file_metadata_sha256 != case.file_metadata_sha256
    {
        return Err(Failure::FileMetadataIdentityDiffers);
    }
    let RequestRoot::Written(root) = &reference.actual_request_root else {
        return Err(Failure::RequestRootMissing);
    };
    if captured.actual_request_root != *root {
        return Err(Failure::ObservationRootDiffers);
    }
    let files = case
        .files
        .iter()
        .map(|file| PathBuf::from(&file.0))
        .collect::<Vec<_>>();
    let stdout = crate::domain::process_output::service::expand(
        &process.stdout,
        Path::new(&root.0),
        &files,
    )?;
    let stderr = crate::domain::process_output::service::expand(
        &process.stderr,
        Path::new(&root.0),
        &files,
    )?;
    let expected_key = crate::domain::process_output::service::observation_key(
        &crate::domain::process_output::types::ReceiptKeyParts {
            rule: crate::domain::canonical_text::types::Label::from(case.rule.0.as_str()),
            name: crate::domain::canonical_text::types::Label::from(case.name.0.as_str()),
            side: crate::domain::canonical_text::types::Label::from(
                crate::domain::discrepancy::types::Contestant::Reference
                    .to_string()
                    .as_str(),
            ),
        },
    );
    let ProcessArtifactsRecord::Attached(pointers) = &captured.artifacts else {
        return Err(Failure::MissingRawProcessPath);
    };
    for pointer in [&pointers.stdout, &pointers.stderr, &pointers.invocation] {
        if Path::new(&pointer.path.0)
            .components()
            .nth(1)
            .is_none_or(|part| part.as_os_str() != expected_key.0.as_str())
        {
            return Err(Failure::ArtifactBelongsToAnotherCase);
        }
    }
    if *raw(artifacts, &pointers.stdout)? != stdout || *raw(artifacts, &pointers.stderr)? != stderr
    {
        return Err(Failure::CapturedBytesDifferFromTemplates);
    }
    let _record = raw(artifacts, &pointers.invocation)?;
    let InvocationDocument::Complete(invocation) = invocations
        .get(Path::new(&pointers.invocation.path.0))
        .ok_or(Failure::MissingRawInvocation)?
    else {
        return Err(Failure::MetadataCaptureFailed);
    };
    require_invocation(case, reference, invocation, root, &executable)?;
    require_settings(case, invocation)
}
