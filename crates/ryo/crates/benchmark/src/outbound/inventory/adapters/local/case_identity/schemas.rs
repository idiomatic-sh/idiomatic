//! A case's identity beside its table's digest: the framed digest of the file metadata it declares and the digest of its
//! sources.
use super::super::canonical::schemas::{Canonical, invalid};
use super::super::constants;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;

/// The framed SHA-256 of a case's declared file metadata, as the benchmark contracts frame it.
pub(in super::super) fn file_metadata_digest(
    values: &BTreeMap<String, Canonical>,
) -> Result<String, Error> {
    let entries = values
        .iter()
        .map(|(path, executable)| match executable {
            Canonical::Flag(executable) => {
                Ok((ryo::linter::RelativePath(path.as_str().into()), *executable))
            }
            Canonical::Null
            | Canonical::Integer(_)
            | Canonical::Float(_)
            | Canonical::Text(_)
            | Canonical::List(_)
            | Canonical::Table(_) => Err(invalid(constants::FILE_METADATA_FLAGS)),
        })
        .collect::<Result<Vec<_>, _>>()?;
    ArtifactDigest::file_metadata(Some(entries.iter().map(|(path, executable)| {
        FileMetadataEntry {
            path,
            executable: if *executable {
                FileExecutable::Set
            } else {
                FileExecutable::Unset
            },
        }
    })))
    .map_err(|error| Error::Invalid(error.to_string()))?
    .map(|digest| digest.0)
    .ok_or_else(|| invalid(constants::FILE_METADATA_FLAGS))
}

/// The SHA-256 of a case's sources: each name, a zero byte and its text, in name order.
pub(in super::super) fn source_digest(
    files: &BTreeMap<String, Canonical>,
) -> Result<String, Error> {
    let mut digest = Sha256::new();
    for (file, source) in files {
        digest.update(file.as_bytes());
        digest.update([0]);
        digest.update(
            source
                .as_text()
                .ok_or_else(|| invalid(constants::CASE_FILES))?
                .as_bytes(),
        );
    }
    Ok(format!("{:x}", digest.finalize()))
}
