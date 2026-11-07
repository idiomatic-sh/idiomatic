//! Byte-exact path substitution; no regular expressions or inferred normalization.
use crate::domain::suite::constants;
use ryo_benchmark_contracts::domain::process::types::OutputPart;
use ryo_benchmark_contracts::domain::process::types::PathEncoding;
use std::path::{Component, Path, PathBuf};

/// Expand only declared input paths from an absolute UTF8 request root.
///
/// # Errors
/// Rejects relative/empty/nonUTF8 roots, escaping paths and undeclared inputs.
pub fn expand(parts: &[OutputPart], root: &Path, files: &[PathBuf]) -> Result<Vec<u8>, String> {
    if !root.is_absolute() || root.as_os_str().is_empty() || root.to_str().is_none() {
        return Err(constants::PROCESS_OBSERVATION_REQUIRES_AN_ABSOLUTE_UTF.into());
    }
    let mut bytes = Vec::new();
    for part in parts {
        match part {
            OutputPart::Literal(value) => bytes.extend_from_slice(value.as_bytes()),
            OutputPart::InputPath { file, encoding } => {
                if file.to_str().is_none_or(|text| text.contains(['\\', ':']))
                    || file.as_os_str().is_empty()
                    || !files.contains(file)
                    || file
                        .components()
                        .any(|part| !matches!(part, Component::Normal(_)))
                {
                    return Err(constants::PROCESS_OUTPUT_SLOT_IS_NOT_A.into());
                }
                let path = root.join(file);
                let text = path
                    .to_str()
                    .ok_or(constants::PROCESS_INPUT_PATH_IS_NOT_UTF8)?;
                match encoding {
                    PathEncoding::Utf8Bytes => bytes.extend_from_slice(text.as_bytes()),
                    PathEncoding::JsonStringContent => {
                        let quoted =
                            serde_json::to_string(text).map_err(|error| error.to_string())?;
                        bytes.extend_from_slice(&quoted.as_bytes()[1..quoted.len() - 1]);
                    }
                }
            }
        }
    }
    Ok(bytes)
}

/// Frame receipt identities without using case names as filesystem components.
///
/// # Errors
/// Refuses lengths that cannot be represented by the versioned byte framing.
pub fn observation_key(rule: &str, name: &str, side: &str) -> Result<String, String> {
    use sha2::Digest as _;
    let mut hash = sha2::Sha256::new();
    hash.update(constants::RYO_CASE_PROCESS_V1);
    for part in [rule, name, side] {
        hash.update(
            u64::try_from(part.len())
                .map_err(|error| error.to_string())?
                .to_be_bytes(),
        );
        hash.update(part.as_bytes());
    }
    Ok(format!("{:x}", hash.finalize()))
}
