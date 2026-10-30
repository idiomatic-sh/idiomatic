//! The `language` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It says which files are Python.

use super::constants::EXTENSIONS;
use ryo::languages;

/// Whether `path` names a Python file: a module, a stub or a module run without a console.
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
