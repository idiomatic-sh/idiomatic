//! The `language` service: free functions named by verb over the domain's types, with no IO and no ports. It says which files are
//! CSS.

use super::constants::EXTENSION;
use ryo::languages;

/// Whether `path` names a CSS file: one ending in `.css`, in any case.
#[must_use]
pub(crate) fn reads(path: languages::SourcePath<'_>) -> languages::Recognition {
    let ending = path
        .0
        .len()
        .checked_sub(EXTENSION.len())
        .and_then(|from| path.0.get(from..));
    if ending.is_some_and(|ending| ending.eq_ignore_ascii_case(EXTENSION)) {
        languages::Recognition::Reads
    } else {
        languages::Recognition::Declines
    }
}
