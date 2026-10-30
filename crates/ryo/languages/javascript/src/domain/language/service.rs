//! The `language` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It says which files are javascript, and how each is read.

use super::constants::{EXTENSIONS, MODULE_EXTENSION, SCRIPT_EXTENSION};
use crate::domain::syntax::types::Goal;
use ryo::languages;

/// Whether `path` names a javascript file: one ending in the language's extension.
#[must_use]
pub(crate) fn reads(path: languages::SourcePath<'_>) -> languages::Recognition {
    if EXTENSIONS
        .iter()
        .any(|extension| path.0.ends_with(extension))
    {
        languages::Recognition::Reads
    } else {
        languages::Recognition::Declines
    }
}

/// How the file at `path` is read: a `.mjs` file as a module, a `.cjs` file as a script, and every other file with module syntax
/// detected and JSX read.
#[must_use]
pub(crate) fn goal(path: languages::SourcePath<'_>) -> Goal {
    if path.0.ends_with(MODULE_EXTENSION) {
        Goal::Module
    } else if path.0.ends_with(SCRIPT_EXTENSION) {
        Goal::Script
    } else {
        Goal::Detected
    }
}
