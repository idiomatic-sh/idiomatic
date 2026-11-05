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
    let name = receipt
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("receipt filename is not UTF8")?;
    let parent = receipt
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let prefix = format!("{name}.process-");
    let mut bytes = BTreeMap::new();
    let mut directories = BTreeSet::new();
    for row in rows {
        let Some(artifacts) = row
            .get("process_observation")
            .and_then(|value| value.get("artifacts"))
        else {
            continue;
        };
        let object = artifacts
            .as_object()
            .ok_or("process artifacts must be an object")?;
        if object.len() != 3 {
            return Err("process observation needs three exact raw artifacts".into());
        }
        for (label, filename) in [
            ("stdout", "stdout.bin"),
            ("stderr", "stderr.bin"),
            ("invocation", "invocation.json"),
        ] {
            let raw = object.get(label).ok_or("missing named raw artifact")?;
            let text = raw["path"].as_str().ok_or("missing raw artifact path")?;
            let path = PathBuf::from(text);
            let components = path.components().collect::<Vec<_>>();
            if text.contains(['\\', ':'])
                || components.len() != 3
                || !components
                    .iter()
                    .all(|part| matches!(part, Component::Normal(_)))
            {
                return Err("unsafe raw process artifact path".into());
            }
            let attempt = components[0]
                .as_os_str()
                .to_str()
                .ok_or("nonUTF8 attempt")?;
            let key = components[1]
                .as_os_str()
                .to_str()
                .ok_or("nonUTF8 case key")?;
            if !attempt.starts_with(&prefix)
                || attempt.len() == prefix.len()
                || key.len() != 64
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                || components[2].as_os_str() != filename
            {
                return Err("raw artifact is outside its receipt-file namespace".into());
            }
            let full = parent
                .join(&path)
                .canonicalize()
                .map_err(|error| error.to_string())?;
            if !full.starts_with(&parent) || !full.is_file() {
                return Err("raw artifact escapes its receipt boundary".into());
            }
            let data = std::fs::read(&full).map_err(|error| error.to_string())?;
            if raw["sha256"] != format!("{:x}", Sha256::digest(&data))
                || bytes.insert(path.clone(), data).is_some()
            {
                return Err(
                    "raw process artifact is stale or reused by another observation".into(),
                );
            }
            directories.insert(
                full.parent()
                    .ok_or("raw artifact has no case directory")?
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
        if names != ["invocation.json", "stderr.bin", "stdout.bin"].map(std::ffi::OsString::from) {
            return Err("extra raw artifacts in referenced case directory".into());
        }
    }
    Ok(bytes)
}
