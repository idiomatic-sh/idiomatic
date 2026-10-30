//! The `language` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It says which files are Rust.

use super::constants::EXTENSION;
use ryo::languages;

/// Whether `path` names a Rust source file.
#[must_use]
pub(crate) fn reads(path: languages::SourcePath<'_>) -> languages::Recognition {
    if path.0.ends_with(EXTENSION) {
        languages::Recognition::Reads
    } else {
        languages::Recognition::Declines
    }
}
