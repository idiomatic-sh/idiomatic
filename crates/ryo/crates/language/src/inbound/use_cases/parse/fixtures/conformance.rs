//! The conformance of a language to the whole catalog, the same for every language, held on what the language's WALK writes. It reads the
//! language's fixtures and holds each answer the language gave to them, relation by relation:
//!
//! - a relation answered `Reads` has a fixture, `relations/<relation>/input.<ext>`, and the rows the walk writes for the input, each one
//!   in full, are the typed rows the language expects (`Expected`);
//! - a relation answered `NotInThisLanguage` has no such fixture (a fixture with the construct in it fails the answer);
//! - a manifest relation answered `Reads` gives rows from the project fixtures, `manifests/<case>/`, each a repository, and the rows of
//!   every case, in full, are the typed rows the language expects (`ManifestExpected`);
//! - every kind of the language's trees is shown by a sample (the sample's walk holds an element of the kind) or marked unread with a
//!   reason;
//! - a folder that names no relation of the catalog is a failure, and so are more kinds than a walk can tell apart.

use std::io::Write as _;
use std::path::Path;

use super::super::types::ParseEngine;
use super::expected::Fixtures;
use super::findings::{Findings, folders_in};
use super::judging::{Configuration, judge_kinds, judge_presence, judge_syntax};
use super::manifests::manifest_rows;
use crate::inbound::use_cases::project::types::ProjectEngine;
use crate::inbound::use_cases::walk::types::WalkEngine;
use ryo_relations::domain::relations::service::{KIND_LIMIT, all, folder_of, provenance_of};
use ryo_relations::domain::relations::types::{Provenance, Relation};

/// Holds a language to the whole catalog, with its fixtures under `fixtures`.
///
/// # Panics
///
/// When any answer of the language is not held by its fixtures: every one is printed as a to-do line first, and the test fails without a
/// panic trace.
pub fn conformance(
    name: &str,
    (parse, walk): (&dyn ParseEngine, &dyn WalkEngine),
    language: &dyn ProjectEngine,
    fixtures: &Path,
    expected: &Fixtures,
    configure: &Configuration<'_>,
) {
    let mut findings = Findings(Vec::new());
    if walk.kind_count().0 > KIND_LIMIT {
        findings.0.push(format!("kind_count is {}, more than the {KIND_LIMIT} kinds a walk can tell apart", walk.kind_count().0));
    }
    let syntax: Vec<Relation> = all()
        .iter()
        .copied()
        .filter(|relation| provenance_of(*relation) != Provenance::Engine && provenance_of(*relation) != Provenance::Manifests)
        .collect();
    judge_presence(walk, language, fixtures, &mut findings);
    if syntax.iter().any(|relation| walk.absence_of(*relation).is_none()) {
        judge_kinds(name, parse, walk, fixtures, expected, &mut findings);
    }
    for relation in &syntax {
        judge_syntax((parse, walk), configure, fixtures, *relation, &expected.relations, &mut findings);
    }
    for folder in folders_in(&fixtures.join("relations")) {
        if !syntax.iter().any(|relation| *folder_of(*relation) == *folder) {
            findings.0.push(format!("relations/{folder}: no relation of the catalog is read from trees under this name"));
        }
    }
    let rows = manifest_rows(parse, walk, language, fixtures, &expected.manifests, &mut findings);
    for relation in all().iter().copied().filter(|relation| provenance_of(*relation) == Provenance::Manifests) {
        if language.absence_of(relation).is_none() && rows.count(relation) == 0 {
            findings.0.push(format!("{}: answers Reads but no tests/fixtures/manifests/ case gives a row of it", folder_of(relation)));
        }
    }
    report(&findings);
}

/// Fails with a to-do line for each answer not yet held, and no panic trace: the failure is a list of what to write.
fn report(findings: &Findings) {
    if findings.0.is_empty() {
        return;
    }
    let mut errors = std::io::stderr().lock();
    for each in &findings.0 {
        // Nothing is left to say when stderr is gone.
        drop(writeln!(errors, "to do: {each}"));
    }
    std::panic::resume_unwind(Box::new(format!(
        "{} answers of the language are not held by its fixtures; the to-do lines are above",
        findings.0.len()
    )));
}
