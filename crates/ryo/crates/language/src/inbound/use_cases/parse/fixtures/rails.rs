//! The rails every language is held to besides its fixtures' rows, over every input of its fixtures, its pinned official suite and its
//! real-world corpus:
//!
//! 1. the tokens of the tree, in order, spell the file byte for byte (nothing is lost, nothing is added);
//! 2. the tree has the shape of the library's own tree: the same nesting, the kind the language maps each kind of the library to, and
//!    the same spans (the language supplies its library's walk);
//! 3. the rows the language writes say what the library's own semantic model says (the language supplies its oracle);
//! 4. the suite's `valid` files and the corpus's files give no syntax error, and the suite's invalid files give at least one.
//!
//! `tests/fixtures/suite/valid/`, `tests/fixtures/suite/invalid/` (in folders below them, as the official suite lays them out) and `tests/fixtures/corpus/` are each required to hold a file.

use std::path::{Path, PathBuf};

use super::super::service::build;
use super::super::types::ParseEngine;
use super::super::types::{FieldRequest, NoteRequest, ParseRequest, Recognition};
use super::expected::{Fixtures, Leafness, OracleRows, RelationRows, Shape, Walked};
use super::findings::{Findings, input_in};
use super::rows::{rows_of, rows_of_relation};
use crate::domain::elements::service::{
    children, diagnostic_count, end_at, kind_at, positions, start_at,
};
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use crate::domain::relations::constants::TOKEN;
use crate::domain::relations::service::{all, provenance_of};
use crate::domain::relations::types::{FileId, Position, Provenance};
use crate::domain::symbols::types::{ByteSlice, Symbols};

/// The kind of file a rail reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Category {
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
struct Input {
    /// Where it is shown from.
    shown: String,
    /// Where it is.
    path: PathBuf,
    /// What kind of file it is.
    category: Category,
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
fn inputs_in(fixtures: &Path, findings: &mut Findings) -> Vec<Input> {
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
            if let Some(input) = input_in(&relation.path()) {
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

/// The finding for a file whose tokens do not spell it, when they do not.
fn lossless(elements: &Elements, bytes: &[u8]) -> Option<String> {
    let mut at = 0_usize;
    for position in positions(elements) {
        if crate::domain::elements::service::flags_at(elements, position).0 & TOKEN.0 == 0 {
            continue;
        }
        let (start, end) = (
            usize::try_from(start_at(elements, position).0).unwrap_or(usize::MAX),
            usize::try_from(end_at(elements, position).0).unwrap_or(usize::MAX),
        );
        if start != at || end < start || end > bytes.len() {
            return Some(format!(
                "the tokens do not spell the file: the token at place {} covers bytes {start}..{end} but the file is spelled up to byte {at}",
                position.0
            ));
        }
        at = end;
    }
    (at != bytes.len()).then(|| {
        format!(
            "the tokens do not spell the file: they end at byte {at} of {}",
            bytes.len()
        )
    })
}

/// What differs between our tree at `position` and the library's `walked`, one sentence for each of the first few.
fn shape_differences(
    elements: &Elements,
    position: Position,
    walked: &Walked,
    path: &str,
    found: &mut Vec<String>,
) {
    if found.len() >= 5 {
        return;
    }
    let ours = (
        kind_at(elements, position),
        start_at(elements, position),
        end_at(elements, position),
        if crate::domain::elements::service::flags_at(elements, position).0 & TOKEN.0 == 0 {
            Leafness::Node
        } else {
            Leafness::Token
        },
    );
    let theirs = (walked.kind, walked.start, walked.end, walked.leaf);
    if ours != theirs {
        found.push(format!(
            "{path}: our element (kind {}, {}..{}, {:?}) is not the library's (kind {}, {}..{}, {:?})",
            ours.0.0, ours.1.0, ours.2.0, ours.3, theirs.0.0, theirs.1.0, theirs.2.0, theirs.3
        ));
        return;
    }
    let below: Vec<Position> = children(elements, position).collect();
    if below.len() != walked.children.len() {
        found.push(format!(
            "{path}: we nest {} elements under kind {} and the library nests {}",
            below.len(),
            ours.0.0,
            walked.children.len()
        ));
        return;
    }
    for (place, (child, walked_child)) in below.iter().zip(&walked.children).enumerate() {
        shape_differences(
            elements,
            *child,
            walked_child,
            &format!("{path}/{place}"),
            found,
        );
    }
}

/// Compare an accepted reference tree, or require a diagnostic for an explicit reference refusal.
fn reference_differences(
    elements: &Elements,
    shape: Shape,
    text: &str,
    category: Category,
    found: &mut Vec<String>,
) {
    let reference = match shape {
        Shape::Walk(walk) | Shape::Standard(walk) => Ok(walk(text)),
        Shape::Reference(read) => read(text),
    };
    if let Ok(walked) = reference {
        if positions(elements).next().is_some() {
            shape_differences(elements, Position(0), &walked, "root", found);
        } else {
            found.push("the reference has a tree and the language has no root".to_owned());
        }
    } else {
        if diagnostic_count(elements).0 == 0 {
            found.push(
                "the reference rejects this source but the language reports no syntax error"
                    .to_owned(),
            );
        }
        if category == Category::Valid {
            found.push(
                "the official suite says this source is valid but the reference rejects it"
                    .to_owned(),
            );
        }
    }
}

/// What a file's syntax errors say against its kind: the suite's valid files and the corpus's real-world files, which the language reads
/// whole (the corpus of a language with comments or trailing commas, as JSON's, is valid in it), give none, and the suite's invalid files
/// give at least one; a relation fixture may carry them.
fn syntax_errors_verdict(shown: &str, category: Category, broken: usize) -> Option<String> {
    match category {
        Category::Valid if broken > 0 => Some(format!(
            "{shown}: the official suite says it is valid and the language reports {broken} syntax errors"
        )),
        Category::Corpus if broken > 0 => Some(format!(
            "{shown}: the corpus holds real-world source the language reads whole, and the language reports {broken} syntax errors"
        )),
        Category::Invalid if broken == 0 => Some(format!(
            "{shown}: the official suite says it is invalid and the language reports no syntax error"
        )),
        Category::Fixture | Category::Valid | Category::Invalid | Category::Corpus => None,
    }
}

/// The finding for an input whose tree carries the syntax errors its kind does not allow, or none that its kind needs.
fn syntax_errors_finding(input: &Input, table: &Elements) -> Option<String> {
    syntax_errors_verdict(&input.shown, input.category, diagnostic_count(table).0)
}

/// Holds one input to the rails.
fn judge_input(
    parse: &dyn ParseEngine,
    input: &Input,
    expected: &Fixtures,
    findings: &mut Findings,
) {
    let Ok(bytes) = std::fs::read(&input.path) else {
        findings.0.push(format!("{}: cannot be read", input.shown));
        return;
    };
    let request = ParseRequest {
        path: SourcePath(&input.shown),
        file: FileId(0),
        bytes: ByteSlice(&bytes),
        notes: NoteRequest::Unwanted,
        fields: FieldRequest::Unwanted,
    };
    if parse.recognition(&request) == Recognition::Declines {
        findings.0.push(format!(
            "{}: a file the language does not recognize: name it with an extension the language reads",
            input.shown
        ));
        return;
    }
    let (mut table, mut symbols) = (Elements::default(), Symbols::default());
    if let Err(error) = build(parse, request, &mut table, &mut symbols) {
        findings.0.push(format!(
            "{}: the language fails to read it: {error:?}",
            input.shown
        ));
        return;
    }
    if let Some(difference) = lossless(&table, &bytes) {
        findings.0.push(format!("{}: {difference}", input.shown));
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return;
    };
    let mut differences = Vec::new();
    reference_differences(
        &table,
        expected.shape,
        text,
        input.category,
        &mut differences,
    );
    findings.0.extend(differences.into_iter().map(|each| {
        format!(
            "{}: not the shape of the library's tree: {each}",
            input.shown
        )
    }));
    if let Some(finding) = syntax_errors_finding(input, &table) {
        findings.0.push(finding);
    }
    if input.category == Category::Invalid {
        return;
    }
    let Some(read) = rows_of(parse, &input.path, &input.shown, findings) else {
        return;
    };
    let rows = OracleRows(
        all()
            .iter()
            .copied()
            .filter(|relation| provenance_of(*relation) == Provenance::Trees)
            .filter(|relation| super::super::service::absence(parse, *relation).is_none())
            .map(|relation| RelationRows {
                relation,
                rows: rows_of_relation(&read, relation, &input.shown),
            })
            .collect(),
    );
    findings
        .0
        .extend((expected.oracle)(text, &rows).into_iter().map(|each| {
            format!(
                "{}: the rows disagree with the library: {each}",
                input.shown
            )
        }));
}

/// Holds every input of the language's fixtures, its suite and its corpus to the rails.
pub(super) fn judge_rails(
    parse: &dyn ParseEngine,
    fixtures: &Path,
    expected: &Fixtures,
    findings: &mut Findings,
) {
    for input in inputs_in(fixtures, findings) {
        judge_input(parse, &input, expected, findings);
    }
}

#[cfg(test)]
#[path = "rails_test.rs"]
mod tests;
