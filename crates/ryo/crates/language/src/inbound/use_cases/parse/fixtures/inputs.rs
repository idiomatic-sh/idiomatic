//! The inputs the rails read: the files of a language's fixtures, its pinned official suite and its real-world corpus, each with the kind
//! of file it is.

use std::path::{Path, PathBuf};

use super::findings::{Findings, inputs_in as relation_inputs};

/// The kind of file a rail reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Category {
    /// A fixture of a relation.
    Fixture,
    /// A file the official suite says is valid.
    Valid,
    /// A file the official suite says is invalid.
    Invalid,
    /// A real-world file.
    Corpus,
}

/// A file a rail reads.
pub(super) struct Input {
    /// Where it is shown from.
    pub(super) shown: String,
    /// Where it is.
    pub(super) path: PathBuf,
    /// What kind of file it is.
    pub(super) category: Category,
}

/// The files of `folder` and of the folders below it, sorted.
#[expect(
    clippy::expect_used,
    reason = "a folder of the fixtures that cannot be read is a broken checkout: the test stops, never runs no cases"
)]
fn files_in(folder: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    // A language need not hold every folder of fixtures.
    if !folder.is_dir() {
        return found;
    }
    for entry in std::fs::read_dir(folder)
        .expect("a folder of the fixtures is read")
        .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
    {
        let path = entry.path();
        if path.is_dir() {
            found.extend(files_in(&path));
            continue;
        }
        if path.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            !name.starts_with("NOTICE") && !name.starts_with("README")
        }) {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Every file the rails read: the inputs of the relations' fixtures, the suite and the corpus.
#[expect(
    clippy::expect_used,
    reason = "a folder of the fixtures that cannot be read is a broken checkout: the test stops, never runs no cases"
)]
pub(super) fn inputs_in(fixtures: &Path, findings: &mut Findings) -> Vec<Input> {
    let mut found = Vec::new();
    let add = |folder: &Path, label: &str, category: Category, into: &mut Vec<Input>| {
        for path in files_in(folder) {
            // Joined by `/` on every platform, as the engine hands a language every path.
            let name = path
                .strip_prefix(folder)
                .unwrap_or(&path)
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            into.push(Input {
                shown: format!("{label}/{name}"),
                path,
                category,
            });
        }
    };
    let relations = if fixtures.join("relations").is_dir() {
        std::fs::read_dir(fixtures.join("relations"))
            .expect("a folder of the fixtures is read")
            .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    for relation in relations {
        if relation.path().is_dir() {
            let label = format!("relations/{}", relation.file_name().to_string_lossy());
            for input in relation_inputs(&relation.path()) {
                found.push(Input {
                    shown: super::findings::read_as(&format!(
                        "{label}/{}",
                        input
                            .file_name()
                            .map(|each| each.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    )),
                    path: input,
                    category: Category::Fixture,
                });
            }
        }
    }
    for (folder, label, category) in [
        ("suite/valid", "suite/valid", Category::Valid),
        ("suite/invalid", "suite/invalid", Category::Invalid),
        ("corpus", "corpus", Category::Corpus),
    ] {
        let before = found.len();
        add(&fixtures.join(folder), label, category, &mut found);
        if found.len() == before {
            findings.0.push(format!(
                "tests/fixtures/{folder}/ holds no file: the language is held to its official suite (suite/valid, suite/invalid) and a real-world corpus (corpus)"
            ));
        }
    }
    found
}

#[cfg(test)]
#[path = "inputs_test.rs"]
mod tests;
