//! The content seal of a listed project context: declared bytes, declared absences and the stable logical root, never the
//! temporary physical directory.
use super::constants::{AUXILIARY, LISTED_DISK_CONTEXT, MISSING, SOURCE};
use super::types::ContextManifest;
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use sha2::{Digest as _, Sha256};
use std::ffi::OsStr;

/// Hash one field with its length before it, so no two field sequences share bytes.
fn frame(hash: &mut Sha256, field: &OsStr) {
    let bytes = field.as_encoded_bytes();
    hash.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hash.update(bytes);
}

/// The content seal of a manifest: its logical root, each source and auxiliary file with its digest, and each
/// declared absence, framed.
#[must_use]
pub fn fingerprint(manifest: &ContextManifest) -> ArtifactDigest {
    let mut hash = Sha256::new();
    frame(&mut hash, OsStr::new(LISTED_DISK_CONTEXT));
    frame(&mut hash, manifest.logical_root.as_os_str());
    for (kind, files) in [
        (SOURCE, &manifest.source_files),
        (AUXILIARY, &manifest.auxiliary_files),
    ] {
        frame(&mut hash, OsStr::new(kind));
        for (path, digest) in files {
            frame(&mut hash, path.as_os_str());
            frame(&mut hash, OsStr::new(&digest.0));
        }
    }
    let mut missing = manifest.missing.clone();
    missing.sort();
    for path in missing {
        frame(&mut hash, OsStr::new(MISSING));
        frame(&mut hash, path.as_os_str());
    }
    ArtifactDigest(format!("{:x}", hash.finalize()))
}
