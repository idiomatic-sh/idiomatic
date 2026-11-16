//! The content seal of a listed project context: declared bytes, declared absences and the stable logical root, never the
//! temporary physical directory.
use super::constants::{AUXILIARY, LINK, LISTED_DISK_CONTEXT, MISSING, PORTABLE_SEPARATOR, SOURCE};
use super::types::ContextManifest;
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use sha2::{Digest as _, Sha256};
use std::ffi::{OsStr, OsString};
use std::path::Path;

/// Hash one field with its length before it, so no two field sequences share bytes.
fn frame(hash: &mut Sha256, field: &OsStr) {
    let bytes = field.as_encoded_bytes();
    hash.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hash.update(bytes);
}

/// The path as one platform-independent field: its components joined by `/`, whatever separator the host's paths use, so the
/// same sealed context has the same digest on every platform.
fn portable(path: &Path) -> OsString {
    let mut joined = OsString::new();
    for (index, part) in path.components().enumerate() {
        if index > 0 {
            joined.push(PORTABLE_SEPARATOR);
        }
        joined.push(part.as_os_str());
    }
    joined
}

/// The content seal of a manifest: its logical root, each source and auxiliary file with its digest, and each
/// declared absence, then each sealed link with its target, framed; a manifest without links seals as it did before links existed.
#[must_use]
pub fn fingerprint(manifest: &ContextManifest) -> ArtifactDigest {
    let mut hash = Sha256::new();
    frame(&mut hash, OsStr::new(LISTED_DISK_CONTEXT));
    frame(&mut hash, &portable(&manifest.logical_root));
    for (kind, files) in [
        (SOURCE, &manifest.source_files),
        (AUXILIARY, &manifest.auxiliary_files),
    ] {
        frame(&mut hash, OsStr::new(kind));
        for (path, digest) in files {
            frame(&mut hash, &portable(path));
            frame(&mut hash, OsStr::new(&digest.0));
        }
    }
    let mut missing = manifest.missing.clone();
    missing.sort();
    for path in missing {
        frame(&mut hash, OsStr::new(MISSING));
        frame(&mut hash, &portable(&path));
    }
    for (path, target) in &manifest.links {
        frame(&mut hash, OsStr::new(LINK));
        frame(&mut hash, &portable(path));
        frame(&mut hash, OsStr::new(&target.0));
    }
    ArtifactDigest(format!("{:x}", hash.finalize()))
}
