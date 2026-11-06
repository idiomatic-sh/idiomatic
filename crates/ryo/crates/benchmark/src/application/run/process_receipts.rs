//! Reconstruct expected bytes from registration and actual request roots, never case_passed alone.
use crate::application::run::constants;
use crate::outbound::inventory::types::case_document::{CaseDocument, Comparison};
use crate::outbound::suite::types::{
    observation_document::{ExitStatus, ObservationDocument},
    reference_discrepancy::{ArtifactPointer, ExpectationKind},
    text::{Sha256Text, YesNo},
};
use serde_json::Value;
use sha2::Digest as _;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn raw<'a>(
    artifacts: &'a BTreeMap<PathBuf, Vec<u8>>,
    pointer: &ArtifactPointer,
) -> Result<&'a [u8], String> {
    let bytes = artifacts
        .get(Path::new(&pointer.path.0))
        .ok_or(constants::MISSING_CAPTURED_PROCESS_BYTES)?;
    if pointer.sha256.0 != format!("{:x}", sha2::Sha256::digest(bytes)) {
        return Err(constants::CAPTURED_PROCESS_BYTE_DIGEST_DIFFERS.into());
    }
    Ok(bytes)
}

fn text(value: Option<&Sha256Text>) -> Value {
    value.map_or(Value::Null, |text| Value::String(text.0.clone()))
}

/// Validate only the explicit reference process variant; candidate diagnostics are checked by the caller.
pub(crate) fn validate(
    case: &CaseDocument,
    reference: &ObservationDocument,
    artifacts: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), String> {
    let wire = case
        .reference_discrepancy
        .as_ref()
        .and_then(|certificate| certificate.process.clone())
        .ok_or_else(|| "invalid process registration: missing".to_owned())?;
    let certificate = case
        .reference_discrepancy
        .as_ref()
        .ok_or(constants::REFERENCE_PROCESS_VARIANT_OR_EXACT_TERMINATI)?;
    let process = super::process_discrepancy::convert(wire).map_err(|error| error.to_string())?;
    let (Some(captured), Some(proof)) = (
        reference.process_observation.as_ref(),
        reference.discrepancy_validation.as_ref(),
    ) else {
        return Err(constants::REFERENCE_PROCESS_VARIANT_OR_EXACT_TERMINATI.into());
    };
    let identity = &certificate.identity;
    let executable = proof.actual_executable_sha256.0.as_str();
    if case.comparison != Comparison::Full
        || proof.reference_expectation_kind != ExpectationKind::ProcessFailure
        || captured.kind != ExpectationKind::ProcessFailure
        || captured.metadata_error.is_some()
        || captured.observer_no_deadline != YesNo::Yes
        || captured.exit != ExitStatus(i64::from(process.exit))
        || captured.signal.is_some()
        || captured
            .executable_sha256
            .as_ref()
            .map(|digest| digest.0.as_str())
            != Some(executable)
        || captured.settings_sha256.as_ref() != Some(&identity.settings)
        || captured
            .template_sha256
            .as_ref()
            .map(|digest| digest.0.as_str())
            != Some(
                super::process_discrepancy::output_digest(&process)
                    .0
                    .as_str(),
            )
        || proof
            .process_observation_group_sha256
            .as_ref()
            .map(|digest| digest.0.as_str())
            != Some(
                super::process_discrepancy::group_digest(&process, executable)
                    .map_err(|error| error.to_string())?
                    .0
                    .as_str(),
            )
        || reference.diagnostics.is_some()
        || reference.skipped.is_some()
        || reference.coverage.is_some()
        || reference.execution_metadata.is_some()
        || reference.raw_parity_matches_intended != Some(YesNo::No)
    {
        return Err(constants::REFERENCE_PROCESS_VARIANT_OR_EXACT_TERMINATI.into());
    }
    if identity.file_metadata != case.file_metadata_sha256
        || proof.file_metadata_sha256 != case.file_metadata_sha256
    {
        return Err(constants::PROCESS_CERTIFICATE_FILE_METADATA_IDENTITY_D.into());
    }
    let root = reference
        .actual_request_root
        .as_ref()
        .ok_or(constants::PROCESS_RECEIPT_REQUEST_ROOT_IS_MISSING)?
        .0
        .as_str();
    if captured.actual_request_root.0 != root {
        return Err(constants::PROCESS_OBSERVATION_ROOT_DIFFERS_FROM_RECEIP.into());
    }
    let files = case
        .files
        .iter()
        .map(|file| PathBuf::from(&file.0))
        .collect::<Vec<_>>();
    let stdout =
        crate::domain::suite::process::service::expand(&process.stdout, Path::new(root), &files)?;
    let stderr =
        crate::domain::suite::process::service::expand(&process.stderr, Path::new(root), &files)?;
    let expected_key = crate::domain::suite::process::service::observation_key(
        case.rule.0.as_str(),
        case.name.0.as_str(),
        constants::REFERENCE,
    )?;
    let pointers = captured
        .artifacts
        .as_ref()
        .ok_or(constants::MISSING_RAW_PROCESS_PATH)?;
    for pointer in [&pointers.stdout, &pointers.stderr, &pointers.invocation] {
        if Path::new(&pointer.path.0)
            .components()
            .nth(1)
            .is_none_or(|part| part.as_os_str() != expected_key.as_str())
        {
            return Err(constants::RAW_PROCESS_ARTIFACT_BELONGS_TO_ANOTHER.into());
        }
    }
    if raw(artifacts, &pointers.stdout)? != stdout || raw(artifacts, &pointers.stderr)? != stderr {
        return Err(constants::CAPTURED_PROCESS_BYTES_DIFFER_FROM_REGISTERE.into());
    }
    let invocation: Value = serde_json::from_slice(raw(artifacts, &pointers.invocation)?)
        .map_err(|error| error.to_string())?;
    if !invocation[constants::METADATA_ERROR].is_null() {
        return Err(constants::PROCESS_METADATA_CAPTURE_FAILED.into());
    }
    // Unspecified historical observations omit this optional field; required metadata remains exact.
    if invocation[constants::FILE_METADATA_SHA256] != text(case.file_metadata_sha256.as_ref()) {
        return Err(constants::RAW_PROCESS_INVOCATION_DIFFERS_AT_FILE.into());
    }
    let case_files = Value::Array(
        case.files
            .iter()
            .map(|file| Value::String(file.0.clone()))
            .collect(),
    );
    let exit = Value::from(captured.exit.0);
    for (key, required) in [
        (constants::TOOL, Value::String(identity.tool.0.clone())),
        (
            constants::VERSION,
            Value::String(identity.version.0.clone()),
        ),
        (
            constants::ACTUAL_REQUEST_ROOT,
            Value::String(root.to_owned()),
        ),
        (constants::FILES, case_files),
        (
            constants::SOURCE_SHA256,
            Value::String(case.source_sha256.0.clone()),
        ),
        (
            constants::EXECUTABLE_SHA256,
            Value::String(executable.to_owned()),
        ),
        (
            constants::SETTINGS_SHA256,
            Value::String(identity.settings.0.clone()),
        ),
        (
            constants::PROFILE_SHA256,
            Value::String(identity.profile.0.clone()),
        ),
        (constants::EXIT, exit),
        (constants::SIGNAL, Value::Null),
        (constants::OBSERVER_NO_DEADLINE, Value::Bool(true)),
    ] {
        if invocation.get(key) != Some(&required) {
            return Err(format!("raw process invocation differs at {key}"));
        }
    }
    // Pinned canonical settings authenticate configuration fields; derive argv/env/cwd again from raw metadata.
    let mut settings = invocation[constants::NORMALIZED_SETTINGS].clone();
    if !settings.is_object()
        || Value::String(super::discrepancy::json_digest(&settings).0)
            != Value::String(identity.settings.0.clone())
    {
        return Err(constants::MISSING_PINNED_NORMALIZED_INVOCATION.into());
    }
    settings =
        super::invocation_identity::reconstruct(&invocation).map_err(|error| error.to_string())?;
    if settings != invocation[constants::NORMALIZED_SETTINGS] {
        return Err(constants::RAW_ARGV_ENVIRONMENT_DO_NOT_ESTABLISH.into());
    }
    Ok(())
}
