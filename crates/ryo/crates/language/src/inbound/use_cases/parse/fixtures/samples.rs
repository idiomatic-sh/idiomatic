//! The sample files a language's fixtures hold, and what reading them allocates.

#![expect(
    clippy::expect_used,
    reason = "a sample that cannot be read fails the check that calls for it by panicking"
)]

use super::super::errors::ParseError;
use super::super::service::{baseline, read as read_file};
use super::super::types::ParseEngine;
use super::super::types::{ParseRequest, SourceFile};
use super::collect::Collect;
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest};
use ryo_relations::domain::relations::service::all;
use ryo_relations::domain::relations::types::FileId;
use std::path::Path;

/// Runs the allocation row against one implementation: after a warm-up, reading a file allocates
/// no more than the parser library `library` allocates on its own for the same file. The
/// caller's test binary installs `dhat::Alloc` as its global allocator and has no other thread running.
///
/// # Panics
///
/// When reading the sample allocates, beyond what the library allocates, any block per file.
pub fn allocations(
    subject: &dyn ParseEngine,
    sample: SourceFile,
    library: &dyn Fn(ByteSlice<'_>) -> Result<(), ParseError>,
) {
    const TIMES: u64 = 20;
    let _profiler = dhat::Profiler::builder().testing().build();
    let (mut table, mut symbols, mut out) =
        (Elements::default(), Symbols::default(), Collect::default());
    let mut read = || {
        out.clear();
        let request = ParseRequest {
            path: sample.path,
            file: FileId(0),
            bytes: sample.bytes,
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        };
        read_file(subject, request, &mut table, &mut symbols, all(), &mut out)
            .expect("the sample is a file the language reads");
    };
    for _ in 0_i32..3_i32 {
        read();
        library(sample.bytes).expect("the language's library reads the sample the language reads");
    }
    let before = dhat::HeapStats::get().total_blocks;
    for _ in 0..TIMES {
        read();
    }
    let reading = dhat::HeapStats::get().total_blocks - before;
    let library_start = dhat::HeapStats::get().total_blocks;
    for _ in 0..TIMES {
        library(sample.bytes).expect("the language's library reads the sample the language reads");
    }
    let parser = dhat::HeapStats::get().total_blocks - library_start;
    // The allocations of the library are in both counts, whatever they are: the library parse is `read_tree`, which `build` and the baseline
    // both run. What is left of reading, once the baseline is taken away, is the table and the readers, and the engine allows them
    // none after the warm-up, as the table keeps its room.
    let beyond = i128::from(reading) - i128::from(parser);
    assert!(
        beyond <= i128::from(BEYOND_THE_LIBRARY * TIMES),
        "reading {} allocated {beyond} blocks in {TIMES} files beyond the library's {parser}: the table and the readers allocate nothing per file after warm-up",
        sample.path
    );
}

/// How many blocks per file reading may allocate beyond what the language's own library parse allocates, after the warm-up.
const BEYOND_THE_LIBRARY: u64 = 0;

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

/// Runs the allocation row over every sample of the language's fixtures: reading a file allocates no more than the
/// the baseline, what the language's library does for the file alone through a builder that builds nothing. The caller's test binary installs
/// `dhat::Alloc` as its global allocator and has no other thread running.
///
/// # Panics
///
/// When reading a sample allocates more than the baseline does.
pub fn allocations_of_fixtures(subject: &dyn ParseEngine, fixtures: &Path) {
    for sample in samples_in(fixtures) {
        allocations(subject, sample, &|bytes| {
            baseline(
                subject,
                &ParseRequest {
                    path: sample.path,
                    file: FileId(0),
                    bytes,
                    notes: NoteRequest::Unwanted,
                    fields: FieldRequest::Unwanted,
                },
            )
        });
    }
}
