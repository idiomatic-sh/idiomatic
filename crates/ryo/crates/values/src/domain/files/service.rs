//! The `files` service: free functions named by verb over the files of a run, with no IO and no ports.

use super::constants::separator;
use super::types::{NativeSeparator, PlatformPath};
use ryo_language::domain::files::types::FilePath;

/// The path as the engine writes it: the platform's separator `native` between directories becomes the engine's, so a path read
/// on Windows is the path read anywhere, and a backslash in a name on a platform that does not split on it stays.
#[must_use]
pub fn with_separator(text: PlatformPath<'_>, native: NativeSeparator) -> FilePath {
    FilePath(text.0.replace(native.0, separator()).into())
}
