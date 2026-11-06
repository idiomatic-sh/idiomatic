//! Read only referenced receipt-file attempts; retain unreferenced resume history untouched.
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

pub(super) fn read(
    receipt: &Path,
    rows: &[serde_json::Value],
) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    let name = receipt.file_name().and_then(|name| name.to_str()).ok_or(
        crate::outbound::inventory::adapters::local::constants::RECEIPT_FILENAME_IS_NOT_UTF8,
    )?;
    let parent = receipt
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new(
            crate::outbound::inventory::adapters::local::constants::DOT,
        ))
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let prefix = format!("{name}.process-");
    let mut bytes = BTreeMap::new();
    let mut directories = BTreeSet::new();
    for row in rows {
        let Some(artifacts) = row
            .get(crate::outbound::inventory::adapters::local::constants::PROCESS_OBSERVATION)
            .and_then(|value| {
                value.get(crate::outbound::inventory::adapters::local::constants::ARTIFACTS)
            })
        else {
            continue;
        };
        let object = artifacts
            .as_object()
            .ok_or(crate::outbound::inventory::adapters::local::constants::PROCESS_ARTIFACTS_MUST_BE_AN_OBJECT)?;
        if object.len() != 3 {
            return Err(crate::outbound::inventory::adapters::local::constants::PROCESS_OBSERVATION_NEEDS_THREE_EXACT_RAW.into());
        }
        for (label, filename) in [
            (
                crate::outbound::inventory::adapters::local::constants::STDOUT,
                crate::outbound::inventory::adapters::local::constants::STDOUT_BIN,
            ),
            (
                crate::outbound::inventory::adapters::local::constants::STDERR,
                crate::outbound::inventory::adapters::local::constants::STDERR_BIN,
            ),
            (
                crate::outbound::inventory::adapters::local::constants::INVOCATION,
                crate::outbound::inventory::adapters::local::constants::INVOCATION_JSON,
            ),
        ] {
            let raw = object.get(label).ok_or(
                crate::outbound::inventory::adapters::local::constants::MISSING_NAMED_RAW_ARTIFACT,
            )?;
            let text = raw[crate::outbound::inventory::adapters::local::constants::PATH].as_str().ok_or(crate::outbound::inventory::adapters::local::constants::MISSING_RAW_ARTIFACT_PATH)?;
            let path = PathBuf::from(text);
            let components = path.components().collect::<Vec<_>>();
            if text.contains(['\\', ':'])
                || components.len() != 3
                || !components
                    .iter()
                    .all(|part| matches!(part, Component::Normal(_)))
            {
                return Err(crate::outbound::inventory::adapters::local::constants::UNSAFE_RAW_PROCESS_ARTIFACT_PATH.into());
            }
            let attempt = components[0]
                .as_os_str()
                .to_str()
                .ok_or(crate::outbound::inventory::adapters::local::constants::NONUTF8_ATTEMPT)?;
            let key = components[1]
                .as_os_str()
                .to_str()
                .ok_or(crate::outbound::inventory::adapters::local::constants::NONUTF8_CASE_KEY)?;
            if !attempt.starts_with(&prefix)
                || attempt.len() == prefix.len()
                || key.len() != 64
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                || components[2].as_os_str() != filename
            {
                return Err(crate::outbound::inventory::adapters::local::constants::RAW_ARTIFACT_IS_OUTSIDE_ITS_RECEIPT.into());
            }
            let full = parent
                .join(&path)
                .canonicalize()
                .map_err(|error| error.to_string())?;
            if !full.starts_with(&parent) || !full.is_file() {
                return Err(crate::outbound::inventory::adapters::local::constants::RAW_ARTIFACT_ESCAPES_ITS_RECEIPT_BOUNDARY.into());
            }
            let data = std::fs::read(&full).map_err(|error| error.to_string())?;
            if raw[crate::outbound::inventory::adapters::local::constants::SHA256]
                != format!("{:x}", Sha256::digest(&data))
                || bytes.insert(path.clone(), data).is_some()
            {
                return Err(
                    crate::outbound::inventory::adapters::local::constants::RAW_PROCESS_ARTIFACT_IS_STALE_OR.into(),
                );
            }
            directories.insert(
                full.parent()
                    .ok_or(crate::outbound::inventory::adapters::local::constants::RAW_ARTIFACT_HAS_NO_CASE_DIRECTORY)?
                    .to_path_buf(),
            );
        }
    }
    for directory in directories {
        let mut names = std::fs::read_dir(directory)
            .map_err(|error| error.to_string())?
            .map(|entry| {
                entry
                    .map(|entry| entry.file_name())
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        names.sort();
        if names
            != [
                crate::outbound::inventory::adapters::local::constants::INVOCATION_JSON,
                crate::outbound::inventory::adapters::local::constants::STDERR_BIN,
                crate::outbound::inventory::adapters::local::constants::STDOUT_BIN,
            ]
            .map(std::ffi::OsString::from)
        {
            return Err(crate::outbound::inventory::adapters::local::constants::EXTRA_RAW_ARTIFACTS_IN_REFERENCED_CASE.into());
        }
    }
    Ok(bytes)
}
