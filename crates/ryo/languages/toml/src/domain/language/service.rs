//! The `language` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It says which files are TOML.

use super::constants::{EXTENSION, LOCK_FILES};
use ryo::languages;

/// Whether `path` names a TOML file: one ending in `.toml`, or a lock file written in TOML.
#[must_use]
pub(crate) fn reads(path: languages::SourcePath<'_>) -> languages::Recognition {
    let name = path.0.rsplit('/').next().unwrap_or(path.0);
    if path.0.ends_with(EXTENSION) || LOCK_FILES.contains(&name) {
        languages::Recognition::Reads
    } else {
        languages::Recognition::Declines
    }
}
