//! The Python sources of the language's fixtures, read from the folder they are pinned in: what the walk is held to today's reading
//! over.

use std::path::{Path, PathBuf};

/// Every Python source under the language's fixtures, with its path under them: the relations' inputs, the corpus and the
/// parser's suite, the files it refuses among them. In the order of their paths.
///
/// # Errors
///
/// When a fixture folder or file cannot be read.
pub(crate) fn fixture_sources() -> std::io::Result<Vec<(String, Vec<u8>)>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut folders: Vec<PathBuf> = vec![root.clone()];
    let mut found = Vec::new();
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder)? {
            let path = entry?.path();
            if path.is_dir() {
                folders.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "py" || extension == "pyi")
            {
                let shown = path
                    .strip_prefix(&root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                found.push((shown, std::fs::read(&path)?));
            } else {
                // Any other file is no Python source: an expected-rows file, a notice, a manifest.
            }
        }
    }
    found.sort();
    Ok(found)
}
