//! Reconstruct expected bytes from registration and actual request roots, never case_passed alone.
use serde_json::Value;
use sha2::Digest as _;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn raw<'a>(artifacts: &'a BTreeMap<PathBuf, Vec<u8>>, pointer: &Value) -> Result<&'a [u8], String> {
    let path = pointer[crate::application::run::constants::PATH]
        .as_str()
        .ok_or(crate::application::run::constants::PROCESS_ARTIFACT_PATH_IS_MISSING)?;
    let bytes = artifacts
        .get(Path::new(path))
        .ok_or(crate::application::run::constants::MISSING_CAPTURED_PROCESS_BYTES)?;
    if pointer[crate::application::run::constants::SHA256]
        != format!("{:x}", sha2::Sha256::digest(bytes))
    {
        return Err(
            crate::application::run::constants::CAPTURED_PROCESS_BYTE_DIGEST_DIFFERS.into(),
        );
    }
    Ok(bytes)
}
/// Validate only the explicit reference process variant; candidate diagnostics are checked by the caller.
pub(super) fn validate(
    case: &Value,
    reference: &Value,
    artifacts: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), String> {
    let certificate = &case[crate::application::run::constants::REFERENCE_DISCREPANCY];
    let wire =
        serde_json::from_value(certificate[crate::application::run::constants::PROCESS].clone())
            .map_err(|error| format!("invalid process registration: {error}"))?;
    let process = super::process_discrepancy::convert(wire).map_err(|error| error.to_string())?;
    let captured = &reference[crate::application::run::constants::PROCESS_OBSERVATION];
    let proof = &reference[crate::application::run::constants::DISCREPANCY_VALIDATION];
    let identity = &certificate[crate::application::run::constants::IDENTITY];
    let executable = proof[crate::application::run::constants::ACTUAL_EXECUTABLE_SHA256]
        .as_str()
        .ok_or(crate::application::run::constants::MISSING_ACTUAL_PROCESS_EXECUTABLE)?;
    if case[crate::application::run::constants::COMPARISON]
        != crate::application::run::constants::FULL
        || proof[crate::application::run::constants::REFERENCE_EXPECTATION_KIND]
            != crate::application::run::constants::PROCESS_FAILURE
        || captured[crate::application::run::constants::KIND]
            != crate::application::run::constants::PROCESS_FAILURE
        || !captured[crate::application::run::constants::METADATA_ERROR].is_null()
        || captured[crate::application::run::constants::OBSERVER_NO_DEADLINE] != true
        || captured[crate::application::run::constants::EXIT].as_i64()
            != Some(i64::from(process.exit))
        || captured.get(crate::application::run::constants::SIGNAL) != Some(&Value::Null)
        || captured[crate::application::run::constants::EXECUTABLE_SHA256] != executable
        || captured[crate::application::run::constants::SETTINGS_SHA256]
            != identity[crate::application::run::constants::SETTINGS_SHA256_2]
        || captured[crate::application::run::constants::TEMPLATE_SHA256]
            != super::process_discrepancy::output_digest(&process).0
        || proof[crate::application::run::constants::PROCESS_OBSERVATION_GROUP_SHA256]
            != super::process_discrepancy::group_digest(&process, executable)
                .map_err(|error| error.to_string())?
                .0
        || !reference[crate::application::run::constants::DIAGNOSTICS].is_null()
        || !reference[crate::application::run::constants::SKIPPED].is_null()
        || !reference[crate::application::run::constants::COVERAGE].is_null()
        || !reference[crate::application::run::constants::EXECUTION_METADATA].is_null()
        || reference[crate::application::run::constants::RAW_PARITY_MATCHES_INTENDED] != false
    {
        return Err(
            crate::application::run::constants::REFERENCE_PROCESS_VARIANT_OR_EXACT_TERMINATI.into(),
        );
    }
    if identity[crate::application::run::constants::FILE_METADATA_SHA256_2]
        != case[crate::application::run::constants::FILE_METADATA_SHA256]
        || proof[crate::application::run::constants::FILE_METADATA_SHA256]
            != case[crate::application::run::constants::FILE_METADATA_SHA256]
    {
        return Err(
            crate::application::run::constants::PROCESS_CERTIFICATE_FILE_METADATA_IDENTITY_D.into(),
        );
    }
    let root = reference[crate::application::run::constants::ACTUAL_REQUEST_ROOT]
        .as_str()
        .ok_or(crate::application::run::constants::PROCESS_RECEIPT_REQUEST_ROOT_IS_MISSING)?;
    if captured[crate::application::run::constants::ACTUAL_REQUEST_ROOT] != root {
        return Err(
            crate::application::run::constants::PROCESS_OBSERVATION_ROOT_DIFFERS_FROM_RECEIP.into(),
        );
    }
    let files = case[crate::application::run::constants::FILES]
        .as_array()
        .ok_or(crate::application::run::constants::REGISTERED_PROCESS_FILES_ARE_MISSING)?
        .iter()
        .map(|file| {
            file.as_str()
                .map(PathBuf::from)
                .ok_or(crate::application::run::constants::INVALID_PROCESS_SOURCE_FILENAME)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let stdout =
        crate::domain::suite::process::service::expand(&process.stdout, Path::new(root), &files)?;
    let stderr =
        crate::domain::suite::process::service::expand(&process.stderr, Path::new(root), &files)?;
    let expected_key = crate::domain::suite::process::service::observation_key(
        case[crate::application::run::constants::RULE]
            .as_str()
            .ok_or(crate::application::run::constants::MISSING_REGISTERED_RULE)?,
        case[crate::application::run::constants::NAME]
            .as_str()
            .ok_or(crate::application::run::constants::MISSING_REGISTERED_CASE)?,
        crate::application::run::constants::REFERENCE,
    )?;
    for label in [
        crate::application::run::constants::STDOUT,
        crate::application::run::constants::STDERR,
        crate::application::run::constants::INVOCATION,
    ] {
        let path = captured[crate::application::run::constants::ARTIFACTS][label]
            [crate::application::run::constants::PATH]
            .as_str()
            .ok_or(crate::application::run::constants::MISSING_RAW_PROCESS_PATH)?;
        if Path::new(path)
            .components()
            .nth(1)
            .is_none_or(|part| part.as_os_str() != expected_key.as_str())
        {
            return Err(
                crate::application::run::constants::RAW_PROCESS_ARTIFACT_BELONGS_TO_ANOTHER.into(),
            );
        }
    }
    let pointers = &captured[crate::application::run::constants::ARTIFACTS];
    if raw(
        artifacts,
        &pointers[crate::application::run::constants::STDOUT],
    )? != stdout
        || raw(
            artifacts,
            &pointers[crate::application::run::constants::STDERR],
        )? != stderr
    {
        return Err(
            crate::application::run::constants::CAPTURED_PROCESS_BYTES_DIFFER_FROM_REGISTERE.into(),
        );
    }
    let invocation: Value = serde_json::from_slice(raw(
        artifacts,
        &pointers[crate::application::run::constants::INVOCATION],
    )?)
    .map_err(|error| error.to_string())?;
    if !invocation[crate::application::run::constants::METADATA_ERROR].is_null() {
        return Err(crate::application::run::constants::PROCESS_METADATA_CAPTURE_FAILED.into());
    }
    // Unspecified historical observations omit this optional field; required metadata remains exact.
    if invocation[crate::application::run::constants::FILE_METADATA_SHA256]
        != case[crate::application::run::constants::FILE_METADATA_SHA256]
    {
        return Err(
            crate::application::run::constants::RAW_PROCESS_INVOCATION_DIFFERS_AT_FILE.into(),
        );
    }
    for (key, required) in [
        (
            crate::application::run::constants::TOOL,
            &identity[crate::application::run::constants::TOOL],
        ),
        (
            crate::application::run::constants::VERSION,
            &identity[crate::application::run::constants::VERSION],
        ),
        (
            crate::application::run::constants::ACTUAL_REQUEST_ROOT,
            &reference[crate::application::run::constants::ACTUAL_REQUEST_ROOT],
        ),
        (
            crate::application::run::constants::FILES,
            &case[crate::application::run::constants::FILES],
        ),
        (
            crate::application::run::constants::SOURCE_SHA256,
            &case[crate::application::run::constants::SOURCE_SHA256],
        ),
        (
            crate::application::run::constants::EXECUTABLE_SHA256,
            &proof[crate::application::run::constants::ACTUAL_EXECUTABLE_SHA256],
        ),
        (
            crate::application::run::constants::SETTINGS_SHA256,
            &identity[crate::application::run::constants::SETTINGS_SHA256_2],
        ),
        (
            crate::application::run::constants::PROFILE_SHA256,
            &identity[crate::application::run::constants::PROFILE_SHA256_2],
        ),
        (
            crate::application::run::constants::EXIT,
            &captured[crate::application::run::constants::EXIT],
        ),
        (crate::application::run::constants::SIGNAL, &Value::Null),
        (
            crate::application::run::constants::OBSERVER_NO_DEADLINE,
            &Value::Bool(true),
        ),
    ] {
        if invocation.get(key) != Some(required) {
            return Err(format!("raw process invocation differs at {key}"));
        }
    }
    // Pinned canonical settings authenticate configuration fields; derive argv/env/cwd again from raw metadata.
    let mut settings = invocation[crate::application::run::constants::NORMALIZED_SETTINGS].clone();
    if !settings.is_object()
        || super::discrepancy::json_digest(&settings).0
            != identity[crate::application::run::constants::SETTINGS_SHA256_2]
    {
        return Err(
            crate::application::run::constants::MISSING_PINNED_NORMALIZED_INVOCATION.into(),
        );
    }
    settings =
        super::invocation_identity::reconstruct(&invocation).map_err(|error| error.to_string())?;
    if settings != invocation[crate::application::run::constants::NORMALIZED_SETTINGS] {
        return Err(
            crate::application::run::constants::RAW_ARGV_ENVIRONMENT_DO_NOT_ESTABLISH.into(),
        );
    }
    Ok(())
}
