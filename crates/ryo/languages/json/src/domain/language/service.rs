//! The `language` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It says which files are json, and which of them are read as json with comments.

use super::constants::{
    COMMENTED_DOTFILES, COMMENTED_EXTENSION, COMMENTED_PREFIXES, EXTENSION, MANIFEST_EXTENSION,
    WORKSPACE_EXTENSION,
};
use crate::domain::syntax::types::{Dialect, Text};
use ryo::languages;

/// The name of the file a path ends with.
fn file_name(path: languages::SourcePath<'_>) -> Text<'_> {
    Text(path.0.rsplit('/').next().unwrap_or(path.0))
}

/// Whether `path` names a json file: one ending in `.json`, `.jsonc`, `.code-workspace` or `.webmanifest`, or one of the
/// configurations named without an extension (`.babelrc`, `.eslintrc`, `.swcrc`).
#[must_use]
pub(crate) fn reads(path: languages::SourcePath<'_>) -> languages::Recognition {
    if [
        EXTENSION,
        COMMENTED_EXTENSION,
        WORKSPACE_EXTENSION,
        MANIFEST_EXTENSION,
    ]
    .iter()
    .any(|ending| path.0.ends_with(ending))
        || COMMENTED_DOTFILES.contains(&file_name(path).0)
    {
        languages::Recognition::Reads
    } else {
        languages::Recognition::Declines
    }
}

/// How the file at `path` is read, as TypeScript and VS Code read it: a `.jsonc` or `.code-workspace` file, `tsconfig.json`, `jsconfig.json`, the
/// configurations named like them, and `.babelrc`, `.eslintrc` and `.swcrc` are json with comments; every other file is json as the standard defines it.
#[must_use]
pub(crate) fn dialect(path: languages::SourcePath<'_>) -> Dialect {
    let name = file_name(path);
    let extended = name.0.ends_with(EXTENSION)
        && COMMENTED_PREFIXES
            .iter()
            .any(|prefix| name.0.starts_with(prefix));
    if path.0.ends_with(COMMENTED_EXTENSION)
        || path.0.ends_with(WORKSPACE_EXTENSION)
        || COMMENTED_DOTFILES.contains(&name.0)
        || extended
    {
        Dialect::Jsonc
    } else {
        Dialect::Strict
    }
}
