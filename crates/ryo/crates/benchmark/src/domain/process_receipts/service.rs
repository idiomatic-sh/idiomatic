//! Reconstruct expected bytes from registration and actual request roots, never case_passed alone.
use super::errors::ProcessReceiptsError as Failure;
use crate::domain::canonical_text::{service::text_digest, types::Document};
use crate::domain::invocation_identity::{
    service::normalize_text,
    types::{Boundaries, InvocationText},
};
use crate::domain::process_failure::service::{group_digest, output_digest};
use crate::outbound::inventory::types::{
    ArgumentText, CompleteInvocation, EnvironmentName, EnvironmentValue, InvocationDocument, Pin,
    Signal,
    case_document::{CaseDocument, Comparison},
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::{
    observation_document::{ExitStatus, ObservationDocument},
    reference_discrepancy::{ArtifactPointer, ExpectationKind},
    text::{PathText, Sha256Text, YesNo},
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

fn pin(value: Option<&Sha256Text>) -> Pin {
    value.map_or(Pin::Absent, |digest| Pin::Sha256(digest.clone()))
}

/// The reference side is the explicit process variant, terminated exactly as registered.
fn require_reference_variant(
    case: &CaseDocument,
    reference: &ObservationDocument,
    process: &ProcessFailure,
    executable: &ArtifactDigest,
) -> Result<(), Failure> {
    let (Some(certificate), Some(captured), Some(proof)) = (
        case.reference_discrepancy.as_ref(),
        reference.process_observation.as_ref(),
        reference.discrepancy_validation.as_ref(),
    ) else {
        return Err(Failure::ReferenceVariantDiffers);
    };
    let identity = &certificate.identity;
    let pinned = Sha256Text(executable.0.clone());
    let template = Sha256Text(output_digest(process).0);
    let group = Sha256Text(group_digest(process, executable)?.0);
    if case.comparison != Comparison::Full
        || proof.reference_expectation_kind != ExpectationKind::ProcessFailure
        || captured.kind != ExpectationKind::ProcessFailure
        || captured.metadata_error.is_some()
        || captured.observer_no_deadline != YesNo::Yes
        || captured.exit != ExitStatus(i64::from(process.exit.0))
        || captured.signal.is_some()
        || captured.executable_sha256.as_ref() != Some(&pinned)
        || captured.settings_sha256.as_ref() != Some(&identity.settings)
        || captured.template_sha256.as_ref() != Some(&template)
        || proof.process_observation_group_sha256.as_ref() != Some(&group)
        || reference.diagnostics.is_some()
        || reference.skipped.is_some()
        || reference.coverage.is_some()
        || reference.execution_metadata.is_some()
        || reference.raw_parity_matches_intended != Some(YesNo::No)
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
    let (Some(certificate), Some(captured)) = (
        case.reference_discrepancy.as_ref(),
        reference.process_observation.as_ref(),
    ) else {
        return Err(Failure::ReferenceVariantDiffers);
    };
    let identity = &certificate.identity;
    if invocation.file_metadata_sha256 != pin(case.file_metadata_sha256.as_ref()) {
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
    let identity = case
        .reference_discrepancy
        .as_ref()
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
    let process = case
        .reference_discrepancy
        .as_ref()
        .and_then(|certificate| certificate.process.clone())
        .ok_or(Failure::ProcessRegistrationMissing)?;
    let (Some(captured), Some(proof)) = (
        reference.process_observation.as_ref(),
        reference.discrepancy_validation.as_ref(),
    ) else {
        return Err(Failure::ReferenceVariantDiffers);
    };
    let executable = ArtifactDigest(proof.actual_executable_sha256.0.clone());
    require_reference_variant(case, reference, &process, &executable)?;
    let identity = &case
        .reference_discrepancy
        .as_ref()
        .ok_or(Failure::ReferenceVariantDiffers)?
        .identity;
    if identity.file_metadata != case.file_metadata_sha256
        || proof.file_metadata_sha256 != case.file_metadata_sha256
    {
        return Err(Failure::FileMetadataIdentityDiffers);
    }
    let root = reference
        .actual_request_root
        .as_ref()
        .ok_or(Failure::RequestRootMissing)?;
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
    let pointers = captured
        .artifacts
        .as_ref()
        .ok_or(Failure::MissingRawProcessPath)?;
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
