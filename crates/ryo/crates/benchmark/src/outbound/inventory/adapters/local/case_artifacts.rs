//! Read only referenced receipt-file attempts; retain unreferenced resume history untouched.
use crate::outbound::inventory::adapters::local::constants;
use crate::outbound::suite::types::{
    observation_document::ObservationDocument, reference_discrepancy::ArtifactPointer,
};
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

pub(super) fn read(
    receipt: &Path,
    rows: &[ObservationDocument],
) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    let name = receipt
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(constants::RECEIPT_FILENAME_IS_NOT_UTF8)?;
    let parent = receipt
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new(constants::DOT))
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let prefix = format!("{name}.process-");
    let mut bytes = BTreeMap::new();
    let mut directories = BTreeSet::new();
    for row in rows {
        let Some(artifacts) = row
            .process_observation
            .as_ref()
            .and_then(|process| process.artifacts.as_ref())
        else {
            continue;
        };
        for (raw, filename) in [
            (&artifacts.stdout, constants::STDOUT_BIN),
            (&artifacts.stderr, constants::STDERR_BIN),
            (&artifacts.invocation, constants::INVOCATION_JSON),
        ] {
            let ArtifactPointer { path: text, sha256 } = raw;
            let text = text.0.as_str();
            let path = PathBuf::from(text);
            let components = path.components().collect::<Vec<_>>();
            if text.contains(['\\', ':'])
                || components.len() != 3
                || !components
                    .iter()
                    .all(|part| matches!(part, Component::Normal(_)))
            {
                return Err(constants::UNSAFE_RAW_PROCESS_ARTIFACT_PATH.into());
            }
            let attempt = components[0]
                .as_os_str()
                .to_str()
                .ok_or(constants::NONUTF8_ATTEMPT)?;
            let key = components[1]
                .as_os_str()
                .to_str()
                .ok_or(constants::NONUTF8_CASE_KEY)?;
            if !attempt.starts_with(&prefix)
                || attempt.len() == prefix.len()
                || key.len() != 64
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                || components[2].as_os_str() != filename
            {
                return Err(constants::RAW_ARTIFACT_IS_OUTSIDE_ITS_RECEIPT.into());
            }
            let full = parent
                .join(&path)
                .canonicalize()
                .map_err(|error| error.to_string())?;
            if !full.starts_with(&parent) || !full.is_file() {
                return Err(constants::RAW_ARTIFACT_ESCAPES_ITS_RECEIPT_BOUNDARY.into());
            }
            let data = std::fs::read(&full).map_err(|error| error.to_string())?;
            if sha256.0 != format!("{:x}", Sha256::digest(&data))
                || bytes.insert(path.clone(), data).is_some()
            {
                return Err(constants::RAW_PROCESS_ARTIFACT_IS_STALE_OR.into());
            }
            directories.insert(
                full.parent()
                    .ok_or(constants::RAW_ARTIFACT_HAS_NO_CASE_DIRECTORY)?
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
                constants::INVOCATION_JSON,
                constants::STDERR_BIN,
                constants::STDOUT_BIN,
            ]
            .map(std::ffi::OsString::from)
        {
            return Err(constants::EXTRA_RAW_ARTIFACTS_IN_REFERENCED_CASE.into());
        }
    }
    Ok(bytes)
}
