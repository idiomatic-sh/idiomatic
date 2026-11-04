//! The manifests of a fixture project, read through a language and shown as cells.

use super::super::service::build;
use super::super::types::ParseEngine;
use super::super::types::{ParseRequest, Recognition};
use super::contract::Collect;
use super::expected::{Cell, ManifestExpected, shown};
use super::findings::{Findings, folders_in};
use super::rows::{Read, compare, rows_of_relation};
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest};
use crate::inbound::use_cases::project::types::ProjectEngine;
use ryo_relations::domain::relations::service::{all, folder_of, provenance_of};
use ryo_relations::domain::relations::types::Provenance;
use ryo_relations::domain::relations::types::{Bytes, FileId};

use crate::domain::files::types::FilePath;
use crate::inbound::use_cases::project::types::{
    ManifestFile, ManifestText, ProjectRequest, RootPath,
};
use std::path::{Path, PathBuf};

/// The files under `root`, by path relative to it, sorted.
#[expect(
    clippy::expect_used,
    reason = "a folder of the fixtures that cannot be read is a broken checkout: the test stops, never runs no cases"
)]
pub(super) fn files_under(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(root.join(&directory))
            .expect("a folder of the fixtures is read")
            .map(|entry| entry.expect("an entry of a folder of the fixtures is read"))
        {
            let relative = directory.join(entry.file_name());
            if entry.path().is_dir() {
                pending.push(relative);
            } else {
                // Separated by `/` on every platform, as the engine hands a language every path.
                found.push(
                    relative
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    found.sort();
    found
}

/// A row of a manifest relation, led by the relation's folder, so the rows of all the manifest relations are compared together.
pub(super) fn with_folder(folder: &str, row: Vec<Cell>) -> Vec<Cell> {
    std::iter::once(Cell::Name(folder.to_owned()))
        .chain(row)
        .collect()
}

/// What the language leaves in the notes of each file of the repository at `root`, in the order of `files`; none for a file it does not
/// read.
fn notes_of(parse: &dyn ParseEngine, root: &Path, files: &[String]) -> Vec<Bytes> {
    files
        .iter()
        .map(|path| {
            let Ok(bytes) = std::fs::read(root.join(path)) else {
                return Bytes::default();
            };
            let request = ParseRequest {
                path: SourcePath(path),
                file: FileId(0),
                bytes: ByteSlice(&bytes),
                notes: NoteRequest::Wanted,
                fields: FieldRequest::Unwanted,
            };
            let mut table = Elements::default();
            if parse.recognition(&request) == Recognition::Reads
                && build(parse, request, &mut table, &mut Symbols::default()).is_ok()
            {
                crate::domain::elements::service::take_notes(&mut table)
            } else {
                Bytes::default()
            }
        })
        .collect()
}

/// The rows the manifests of each project fixture give, as one writer, and each case's rows held to the rows the language expects.
pub(super) fn manifest_rows(
    parse: &dyn ParseEngine,
    language: &dyn ProjectEngine,
    fixtures: &Path,
    expected: &[ManifestExpected],
    findings: &mut Findings,
) -> Collect {
    let mut rows = Collect::default();
    for case in folders_in(&fixtures.join("manifests")) {
        let root = fixtures.join("manifests").join(&case);
        let files = files_under(&root);
        let mut held = Read {
            out: Collect::default(),
            symbols: Symbols::default(),
            bytes: Vec::new(),
            texts: files
                .iter()
                .map(|path| std::fs::read(root.join(path)).unwrap_or_default())
                .collect(),
            files: files.clone(),
            unknowns: parse.unknown_reasons(),
        };
        let manifests: Vec<ManifestFile> = files
            .iter()
            .filter(|path| {
                Path::new(path)
                    .file_name()
                    .is_some_and(|name| language.manifest_names().iter().any(|each| name == each.0))
            })
            .map(|path| ManifestFile {
                path: FilePath(path.as_str().into()),
                text: ManifestText(
                    std::fs::read_to_string(root.join(path))
                        .unwrap_or_default()
                        .into(),
                ),
            })
            .collect();
        let notes = notes_of(parse, &root, &files);
        let request = ProjectRequest {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            observed_entries: Vec::new(),
            repository: crate::inbound::use_cases::project::types::Repository::Disk,
            root: RootPath(root.to_string_lossy().into()),
            manifests,
            files: files
                .iter()
                .map(|each| FilePath(each.as_str().into()))
                .collect(),
            notes,
            wanted: all().to_vec(),
        };
        if let Err(error) = language.project(&request, &mut held.symbols, &mut held.out) {
            findings.0.push(format!(
                "tests/fixtures/manifests/{case}/: a project the language fails to read: {error:?}"
            ));
        }
        let mut got: Vec<Vec<Cell>> = all()
            .iter()
            .copied()
            .filter(|relation| {
                provenance_of(*relation) == Provenance::Manifests
                    && language.absence_of(*relation).is_none()
            })
            .flat_map(|relation| {
                rows_of_relation(&held, relation, "")
                    .into_iter()
                    .map(move |row| with_folder(folder_of(relation).0, row))
            })
            .collect();
        got.sort();
        let mut wanted: Vec<Vec<Cell>> = expected
            .iter()
            .filter(|each| each.case == case)
            .flat_map(|each| each.rows.iter())
            .map(|(folder, row)| with_folder(folder, row.clone()))
            .collect();
        wanted.sort();
        if wanted.is_empty() {
            findings.0.push(format!(
                "tests/fixtures/manifests/{case}/: the language expects no row of the case (`ManifestExpected` for `{case}`)"
            ));
        }
        compare(
            &format!("tests/fixtures/manifests/{case}/"),
            &got,
            &wanted,
            &|row| match row.split_first() {
                Some((Cell::Name(folder), cells)) => all()
                    .iter()
                    .find(|relation| *folder_of(**relation) == **folder)
                    .map_or_else(|| format!("{row:?}"), |relation| shown(*relation, cells)),
                _ => format!("{row:?}"),
            },
            findings,
        );
        rows.relations.extend(held.out.relations);
        rows.words.extend(held.out.words);
    }
    rows
}
