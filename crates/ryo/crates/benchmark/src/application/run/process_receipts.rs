//! Reconstruct expected bytes from registration and actual request roots, never case_passed alone.
use serde_json::Value;
use sha2::Digest as _;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn raw<'a>(artifacts: &'a BTreeMap<PathBuf, Vec<u8>>, pointer: &Value) -> Result<&'a [u8], String> {
    let path = pointer["path"]
        .as_str()
        .ok_or("process artifact path is missing")?;
    let bytes = artifacts
        .get(Path::new(path))
        .ok_or("missing captured process bytes")?;
    if pointer["sha256"] != format!("{:x}", sha2::Sha256::digest(bytes)) {
        return Err("captured process byte digest differs".into());
    }
    Ok(bytes)
}
/// Validate only the explicit reference process variant; candidate diagnostics are checked by the caller.
pub(super) fn validate(
    case: &Value,
    reference: &Value,
    artifacts: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), String> {
    let certificate = &case["reference_discrepancy"];
    let wire = serde_json::from_value(certificate["process"].clone())
        .map_err(|error| format!("invalid process registration: {error}"))?;
    let process = super::process_discrepancy::convert(wire).map_err(|error| error.to_string())?;
    let captured = &reference["process_observation"];
    let proof = &reference["discrepancy_validation"];
    let identity = &certificate["identity"];
    let executable = proof["actual_executable_sha256"]
        .as_str()
        .ok_or("missing actual process executable")?;
    if case["comparison"] != "full"
        || proof["reference_expectation_kind"] != "process-failure"
        || captured["kind"] != "process-failure"
        || !captured["metadata_error"].is_null()
        || captured["observer_no_deadline"] != true
        || captured["exit"].as_i64() != Some(i64::from(process.exit))
        || captured.get("signal") != Some(&Value::Null)
        || captured["executable_sha256"] != executable
        || captured["settings_sha256"] != identity["settings-sha256"]
        || captured["template_sha256"] != super::process_discrepancy::output_digest(&process).0
        || proof["process_observation_group_sha256"]
            != super::process_discrepancy::group_digest(&process, executable)
                .map_err(|error| error.to_string())?
                .0
        || !reference["diagnostics"].is_null()
        || !reference["skipped"].is_null()
        || !reference["coverage"].is_null()
        || !reference["execution_metadata"].is_null()
        || reference["raw_parity_matches_intended"] != false
    {
        return Err("reference process variant or exact termination identity differs".into());
    }
    let root = reference["actual_request_root"]
        .as_str()
        .ok_or("process receipt request root is missing")?;
    if captured["actual_request_root"] != root {
        return Err("process observation root differs from receipt".into());
    }
    let files = case["files"]
        .as_array()
        .ok_or("registered process files are missing")?
        .iter()
        .map(|file| {
            file.as_str()
                .map(PathBuf::from)
                .ok_or("invalid process source filename")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let stdout =
        crate::domain::suite::process::service::expand(&process.stdout, Path::new(root), &files)?;
    let stderr =
        crate::domain::suite::process::service::expand(&process.stderr, Path::new(root), &files)?;
    let expected_key = crate::domain::suite::process::service::observation_key(
        case["rule"].as_str().ok_or("missing registered rule")?,
        case["name"].as_str().ok_or("missing registered case")?,
        "reference",
    )?;
    for label in ["stdout", "stderr", "invocation"] {
        let path = captured["artifacts"][label]["path"]
            .as_str()
            .ok_or("missing raw process path")?;
        if Path::new(path)
            .components()
            .nth(1)
            .is_none_or(|part| part.as_os_str() != expected_key.as_str())
        {
            return Err("raw process artifact belongs to another case identity".into());
        }
    }
    let pointers = &captured["artifacts"];
    if raw(artifacts, &pointers["stdout"])? != stdout
        || raw(artifacts, &pointers["stderr"])? != stderr
    {
        return Err("captured process bytes differ from registered templates".into());
    }
    let invocation: Value = serde_json::from_slice(raw(artifacts, &pointers["invocation"])?)
        .map_err(|error| error.to_string())?;
    if !invocation["metadata_error"].is_null() {
        return Err("process metadata capture failed".into());
    }
    for (key, required) in [
        ("tool", &identity["tool"]),
        ("version", &identity["version"]),
        ("actual_request_root", &reference["actual_request_root"]),
        ("files", &case["files"]),
        ("source_sha256", &case["source_sha256"]),
        ("executable_sha256", &proof["actual_executable_sha256"]),
        ("settings_sha256", &identity["settings-sha256"]),
        ("profile_sha256", &identity["profile-sha256"]),
        ("exit", &captured["exit"]),
        ("signal", &Value::Null),
        ("observer_no_deadline", &Value::Bool(true)),
    ] {
        if invocation.get(key) != Some(required) {
            return Err(format!("raw process invocation differs at {key}"));
        }
    }
    // Pinned canonical settings authenticate configuration fields; derive argv/env/cwd again from raw metadata.
    let mut settings = invocation["normalized_settings"].clone();
    if !settings.is_object()
        || super::discrepancy::json_digest(&settings).0 != identity["settings-sha256"]
    {
        return Err("missing pinned normalized invocation".into());
    }
    settings =
        super::invocation_identity::reconstruct(&invocation).map_err(|error| error.to_string())?;
    if settings != invocation["normalized_settings"] {
        return Err("raw argv/environment do not establish pinned settings".into());
    }
    Ok(())
}
