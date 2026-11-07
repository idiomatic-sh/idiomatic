//! The `language` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It says which files are html.

use super::constants::{EXTENSION, SHORT_EXTENSION};
use ryo::languages;

/// Whether `path` names a html file: one ending in `.html` or `.htm`.
#[must_use]
pub(crate) fn reads(path: languages::SourcePath<'_>) -> languages::Recognition {
    if path.0.ends_with(EXTENSION) || path.0.ends_with(SHORT_EXTENSION) {
        languages::Recognition::Reads
    } else {
        languages::Recognition::Declines
    }
}
