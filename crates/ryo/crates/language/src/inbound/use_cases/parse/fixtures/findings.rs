//! What a language's fixtures say about it, gathered before they are judged, and the folders they are found in.

use std::path::{Path, PathBuf};

/// Every regular `input.<something>` file, sorted by path; absent folders have no inputs.
#[expect(
    clippy::expect_used,
    reason = "a folder of the fixtures that cannot be read is a broken checkout: the test stops, never runs no cases"
)]
pub(super) fn inputs_in(folder: &Path) -> Vec<PathBuf> {
    if !folder.is_dir() {
        return Vec::new();
    }
    let mut inputs: Vec<PathBuf> = std::fs::read_dir(folder)
        .expect("a folder of the fixtures is read")
        .map(|entry| {
            entry
                .expect("an entry of a folder of the fixtures is read")
                .path()
        })
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(INPUT_PREFIX))
        })
        .collect();
    inputs.sort();
    inputs
}

/// The path the language reads a fixture's file as: its own, but for an input named `input.<file name>`, whose rest holds a dot of
/// its own (`input.go.mod`), which is read as that file name (`go.mod`): a language that reads a file only by its whole name has its
/// input read as the file it reads. The rest keeps the last extension, so a language that reads by extension reads either.
pub(super) fn read_as(path: &str) -> String {
    let (folder, name) = path
        .rsplit_once('/')
        .map_or(("", path), |(folder, name)| (folder, name));
    match name.strip_prefix(INPUT_PREFIX) {
        Some(rest) if rest.contains('.') && folder.is_empty() => rest.to_owned(),
        Some(rest) if rest.contains('.') => format!("{folder}/{rest}"),
        Some(_) | None => path.to_owned(),
    }
}

/// What the name of a fixture's input starts with.
const INPUT_PREFIX: &str = "input.";
