//! The sample files a language's fixtures hold, and what reading them allocates.

#![expect(
    clippy::expect_used,
    reason = "a sample that cannot be read fails the check that calls for it by panicking"
)]

use super::super::types::SourceFile;
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use std::path::Path;

/// Leaks `text` for the life of the process: a sample of the contract is `'static`, and a test is short.
fn leaked(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// The files of the language's fixtures that are samples of it, each by its path under `fixtures`: the inputs of the
/// relations it reads, and the files of `allocation/`.
///
/// # Panics
///
/// When a folder of the fixtures cannot be read: a broken checkout stops the test, never runs no cases.
#[must_use]
pub fn samples_in(fixtures: &Path) -> Vec<SourceFile> {
    let mut samples = Vec::new();
    for group in ["relations"] {
        // A language need not hold every group of fixtures.
        if !fixtures.join(group).is_dir() {
            continue;
        }
        let mut folders: Vec<_> = std::fs::read_dir(fixtures.join(group))
            .expect("a folder of the fixtures is read")
            .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
            .map(|entry| entry.path())
            .collect();
        folders.sort();
        for folder in folders {
            for input in super::findings::inputs_in(&folder) {
                push_sample(&mut samples, fixtures, &input);
            }
        }
    }
    if !fixtures.join("allocation").is_dir() {
        return samples;
    }
    let mut files: Vec<_> = std::fs::read_dir(fixtures.join("allocation"))
        .expect("a folder of the fixtures is read")
        .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    for file in files {
        push_sample(&mut samples, fixtures, &file);
    }
    samples
}

/// The files of the `spelled` folder of the fixtures, one source file each, read as the name they carry: the constructs the relation
/// fixtures do not show, so that every kind a language spells is read with text on both routes (the routes' coverage check). Only
/// the routes read them: no relation expects rows of them.
///
/// # Panics
///
/// When the folder cannot be read.
#[must_use]
pub fn spelled_in(fixtures: &Path) -> Vec<SourceFile> {
    let folder = fixtures.join("spelled");
    if !folder.is_dir() {
        return Vec::new();
    }
    let mut files: Vec<_> = std::fs::read_dir(&folder)
        .expect("the spelled folder of the fixtures is read")
        .map(|entry| entry.expect("an entry of the spelled folder is read").path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    let mut samples = Vec::new();
    for file in files {
        push_sample(&mut samples, fixtures, &file);
    }
    samples
}

/// EVERY sample the routes row holds a language to: the relation and allocation samples, then the files of `spelled/`. The old/walk
/// comparison (`routes`), its coverage check and the frozen rows all list their samples here, so that they cannot hold different sets.
///
/// # Panics
///
/// When a folder of the fixtures cannot be read.
#[must_use]
pub fn routes_samples(fixtures: &Path) -> Vec<SourceFile> {
    samples_in(fixtures).into_iter().chain(spelled_in(fixtures)).collect()
}

/// Adds the file at `path`, under `fixtures`, to `samples`.
fn push_sample(samples: &mut Vec<SourceFile>, fixtures: &Path, path: &Path) {
    let (Ok(bytes), Ok(within)) = (std::fs::read(path), path.strip_prefix(fixtures)) else {
        return;
    };
    samples.push(SourceFile {
        // Separated by `/` on every platform, as the engine hands a language every path.
        path: SourcePath(leaked(super::findings::read_as(
            &within
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/"),
        ))),
        bytes: ByteSlice(Box::leak(bytes.into_boxed_slice())),
    });
}
