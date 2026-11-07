//! Byte-exact path substitution; no regular expressions or inferred normalization.
use super::constants;
use super::errors::ProcessOutputError as Error;
use super::types::ReceiptKeyParts;
use crate::domain::canonical_text::{
    service::render,
    types::{Fragment, Label},
};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::process::types::OutputPart;
use ryo_benchmark_contracts::domain::process::types::PathEncoding;
use sha2::Digest as _;
use std::path::{Component, Path, PathBuf};

/// The content of a JSON string, exactly as the canonical document renderer escapes it.
fn json_string_content(text: &Label) -> CapturedBytes {
    let quoted = render(&Fragment::Text(text.clone())).0;
    CapturedBytes(quoted.as_bytes()[1..quoted.len() - 1].to_vec())
}

/// Expand only declared input paths from an absolute UTF8 request root.
///
/// # Errors
/// Rejects relative/empty/nonUTF8 roots, escaping paths and undeclared inputs.
pub(crate) fn expand(
    parts: &[OutputPart],
    root: &Path,
    files: &[PathBuf],
) -> Result<CapturedBytes, Error> {
    if !root.is_absolute() || root.as_os_str().is_empty() || root.to_str().is_none() {
        return Err(Error::RootNotAbsoluteUtf8);
    }
    let mut bytes = Vec::new();
    for part in parts {
        match part {
            OutputPart::Literal(value) => bytes.extend_from_slice(value.0.as_bytes()),
            OutputPart::InputPath { file, encoding } => {
                if file.to_str().is_none_or(|text| text.contains(['\\', ':']))
                    || file.as_os_str().is_empty()
                    || !files.contains(file)
                    || file
                        .components()
                        .any(|part| !matches!(part, Component::Normal(_)))
                {
                    return Err(Error::SlotNotDeclared);
                }
                let path = root.join(file);
                let text = path.to_str().ok_or(Error::RootNotAbsoluteUtf8)?;
                match encoding {
                    PathEncoding::Utf8Bytes => bytes.extend_from_slice(text.as_bytes()),
                    PathEncoding::JsonStringContent => {
                        bytes.extend_from_slice(&json_string_content(&Label::from(text)).0);
                    }
                }
            }
        }
    }
    Ok(CapturedBytes(bytes))
}

/// Frame receipt identities without using case names as filesystem components.
#[must_use]
pub(crate) fn observation_key(identity: &ReceiptKeyParts) -> ArtifactDigest {
    let mut hash = sha2::Sha256::new();
    hash.update(constants::RYO_CASE_PROCESS_V1);
    for part in [&identity.rule, &identity.name, &identity.side] {
        hash.update(
            u64::try_from(part.0.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        hash.update(part.0.as_bytes());
    }
    ArtifactDigest(format!("{:x}", hash.finalize()))
}
