//! The `language` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It says which files are go: its sources, and the `go.mod` of a module, read for what it requires.

use super::constants::EXTENSION;
use super::types::GoFile;
use crate::domain::modules::constants::{MODULE_FILE, SEPARATOR};
use ryo::languages;

/// Which of the language's files `path` names: a source ending in the language's extension, a module's `go.mod`, or neither.
#[must_use]
pub(crate) fn file_of(path: languages::SourcePath<'_>) -> GoFile {
    if path.0.ends_with(EXTENSION) {
        GoFile::Source
    } else if path.0.rsplit(SEPARATOR).next() == Some(MODULE_FILE) {
        GoFile::Module
    } else {
        GoFile::Other
    }
}

/// Whether `path` names a file the language reads: a go source or a module's `go.mod`.
#[must_use]
pub(crate) fn reads(path: languages::SourcePath<'_>) -> languages::Recognition {
    match file_of(path) {
        GoFile::Source | GoFile::Module => languages::Recognition::Reads,
        GoFile::Other => languages::Recognition::Declines,
    }
}
