//! Reconstruct expected bytes from registration and actual request roots, never case_passed alone.
use crate::application::run::constants;
use serde_json::Value;
use sha2::Digest as _;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn raw<'a>(artifacts: &'a BTreeMap<PathBuf, Vec<u8>>, pointer: &Value) -> Result<&'a [u8], String> {
    let path = pointer[constants::PATH]
        .as_str()
        .ok_or(constants::PROCESS_ARTIFACT_PATH_IS_MISSING)?;
    let bytes = artifacts
        .get(Path::new(path))
        .ok_or(constants::MISSING_CAPTURED_PROCESS_BYTES)?;
    if pointer[constants::SHA256] != format!("{:x}", sha2::Sha256::digest(bytes)) {
        return Err(constants::CAPTURED_PROCESS_BYTE_DIGEST_DIFFERS.into());
    }
    Ok(bytes)
}
/// Validate only the explicit reference process variant; candidate diagnostics are checked by the caller.
pub(crate) fn validate(
    case: &Value,
    reference: &Value,
    artifacts: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), String> {
    let certificate = &case[constants::REFERENCE_DISCREPANCY];
    let wire = serde_json::from_value(certificate[constants::PROCESS].clone())
        .map_err(|error| format!("invalid process registration: {error}"))?;
    let process = super::process_discrepancy::convert(wire).map_err(|error| error.to_string())?;
    let captured = &reference[constants::PROCESS_OBSERVATION];
    let proof = &reference[constants::DISCREPANCY_VALIDATION];
    let identity = &certificate[constants::IDENTITY];
    let executable = proof[constants::ACTUAL_EXECUTABLE_SHA256]
        .as_str()
        .ok_or(constants::MISSING_ACTUAL_PROCESS_EXECUTABLE)?;
    if case[constants::COMPARISON] != constants::FULL
        || proof[constants::REFERENCE_EXPECTATION_KIND] != constants::PROCESS_FAILURE
        || captured[constants::KIND] != constants::PROCESS_FAILURE
        || !captured[constants::METADATA_ERROR].is_null()
        || captured[constants::OBSERVER_NO_DEADLINE] != true
        || captured[constants::EXIT].as_i64() != Some(i64::from(process.exit))
        || captured.get(constants::SIGNAL) != Some(&Value::Null)
        || captured[constants::EXECUTABLE_SHA256] != executable
        || captured[constants::SETTINGS_SHA256] != identity[constants::SETTINGS_SHA256_2]
        || captured[constants::TEMPLATE_SHA256]
            != super::process_discrepancy::output_digest(&process).0
        || proof[constants::PROCESS_OBSERVATION_GROUP_SHA256]
            != super::process_discrepancy::group_digest(&process, executable)
                .map_err(|error| error.to_string())?
                .0
        || !reference[constants::DIAGNOSTICS].is_null()
        || !reference[constants::SKIPPED].is_null()
        || !reference[constants::COVERAGE].is_null()
        || !reference[constants::EXECUTION_METADATA].is_null()
        || reference[constants::RAW_PARITY_MATCHES_INTENDED] != false
    {
        return Err(constants::REFERENCE_PROCESS_VARIANT_OR_EXACT_TERMINATI.into());
    }
    if identity[constants::FILE_METADATA_SHA256_2] != case[constants::FILE_METADATA_SHA256]
        || proof[constants::FILE_METADATA_SHA256] != case[constants::FILE_METADATA_SHA256]
    {
        return Err(constants::PROCESS_CERTIFICATE_FILE_METADATA_IDENTITY_D.into());
    }
    let root = reference[constants::ACTUAL_REQUEST_ROOT]
        .as_str()
        .ok_or(constants::PROCESS_RECEIPT_REQUEST_ROOT_IS_MISSING)?;
    if captured[constants::ACTUAL_REQUEST_ROOT] != root {
        return Err(constants::PROCESS_OBSERVATION_ROOT_DIFFERS_FROM_RECEIP.into());
    }
    let files = case[constants::FILES]
        .as_array()
        .ok_or(constants::REGISTERED_PROCESS_FILES_ARE_MISSING)?
        .iter()
        .map(|file| {
            file.as_str()
                .map(PathBuf::from)
                .ok_or(constants::INVALID_PROCESS_SOURCE_FILENAME)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let stdout =
        crate::domain::suite::process::service::expand(&process.stdout, Path::new(root), &files)?;
    let stderr =
        crate::domain::suite::process::service::expand(&process.stderr, Path::new(root), &files)?;
    let expected_key = crate::domain::suite::process::service::observation_key(
        case[constants::RULE]
            .as_str()
            .ok_or(constants::MISSING_REGISTERED_RULE)?,
        case[constants::NAME]
            .as_str()
            .ok_or(constants::MISSING_REGISTERED_CASE)?,
        constants::REFERENCE,
    )?;
    for label in [constants::STDOUT, constants::STDERR, constants::INVOCATION] {
        let path = captured[constants::ARTIFACTS][label][constants::PATH]
            .as_str()
            .ok_or(constants::MISSING_RAW_PROCESS_PATH)?;
        if Path::new(path)
            .components()
            .nth(1)
            .is_none_or(|part| part.as_os_str() != expected_key.as_str())
        {
            return Err(constants::RAW_PROCESS_ARTIFACT_BELONGS_TO_ANOTHER.into());
        }
    }
    let pointers = &captured[constants::ARTIFACTS];
    if raw(artifacts, &pointers[constants::STDOUT])? != stdout
        || raw(artifacts, &pointers[constants::STDERR])? != stderr
    {
        return Err(constants::CAPTURED_PROCESS_BYTES_DIFFER_FROM_REGISTERE.into());
    }
    let invocation: Value =
        serde_json::from_slice(raw(artifacts, &pointers[constants::INVOCATION])?)
            .map_err(|error| error.to_string())?;
    if !invocation[constants::METADATA_ERROR].is_null() {
        return Err(constants::PROCESS_METADATA_CAPTURE_FAILED.into());
    }
    // Unspecified historical observations omit this optional field; required metadata remains exact.
    if invocation[constants::FILE_METADATA_SHA256] != case[constants::FILE_METADATA_SHA256] {
        return Err(constants::RAW_PROCESS_INVOCATION_DIFFERS_AT_FILE.into());
    }
    for (key, required) in [
        (constants::TOOL, &identity[constants::TOOL]),
        (constants::VERSION, &identity[constants::VERSION]),
        (
            constants::ACTUAL_REQUEST_ROOT,
            &reference[constants::ACTUAL_REQUEST_ROOT],
        ),
        (constants::FILES, &case[constants::FILES]),
        (constants::SOURCE_SHA256, &case[constants::SOURCE_SHA256]),
        (
            constants::EXECUTABLE_SHA256,
            &proof[constants::ACTUAL_EXECUTABLE_SHA256],
        ),
        (
            constants::SETTINGS_SHA256,
            &identity[constants::SETTINGS_SHA256_2],
        ),
        (
            constants::PROFILE_SHA256,
            &identity[constants::PROFILE_SHA256_2],
        ),
        (constants::EXIT, &captured[constants::EXIT]),
        (constants::SIGNAL, &Value::Null),
        (constants::OBSERVER_NO_DEADLINE, &Value::Bool(true)),
    ] {
        if invocation.get(key) != Some(required) {
            return Err(format!("raw process invocation differs at {key}"));
        }
    }
    // Pinned canonical settings authenticate configuration fields; derive argv/env/cwd again from raw metadata.
    let mut settings = invocation[constants::NORMALIZED_SETTINGS].clone();
    if !settings.is_object()
        || super::discrepancy::json_digest(&settings).0 != identity[constants::SETTINGS_SHA256_2]
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
