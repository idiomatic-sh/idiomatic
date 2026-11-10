//! The conformance of a language to the whole catalog, the same for every language. It reads the
//! language's fixtures and holds each answer the language gave to them, relation by relation:
//!
//! - a relation answered `Reads` has a fixture, `relations/<relation>/input.<ext>`, and the rows the language
//!   writes for the input, each one in full, are the typed rows the language expects (`Expected`);
//! - a relation answered `NotInThisLanguage` has no such fixture (a fixture with the construct in it fails the
//!   answer); its `AbsenceCause` is required by the trait, and nothing else is asked of it;
//! - a manifest relation answered `Reads` gives rows from the project fixtures, `manifests/<case>/`, each a repository,
//!   and the rows of every case, in full, are the typed rows the language expects (`ManifestExpected`);
//! - every kind of the language's trees is shown by a sample or marked unread with a reason, and a kind with a sample that no
//!   relation reads is a finding (the unread check);
//! - `syntax_tree` and `syntax_error` are relations like the others: `syntax_error` is how a language shows a broken
//!   file, and its fixture is the broken input the contract reads;
//! - every relation answered `NotYetRead` is listed by name on every run, a pass included;
//! - a folder that names no relation of the catalog is a failure, and so are more kinds than a walk can tell apart.

use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest};
use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::Path;

use super::super::service::{absence as parse_absence, build, kinds_read};
use super::super::types::ParseEngine;
use super::super::types::Sample;
use super::super::types::{ParseRequest, Recognition};
use super::configuration::{Configuration, Selection};
use super::contract::{check, flow};
use super::expected::{Cell, Expected, Fixtures};
use super::findings::{Findings, folders_in, input_in, inputs_in};
use super::judging::{Blessing, extension_of, judge_kinds, judge_presence, judge_syntax};
use super::judgments::{judge_classes, judge_columns, judge_reasons, judge_unknowns};
use super::manifests::manifest_rows;
use super::not_yet_read::report_not_yet_read;
use super::rails::judge_rails;
use super::rendered::{expected_source, shown as row_shown};
use super::rows::{compare, rows_of_inputs_with};
use super::samples::samples_in;
use super::semantics::judge as judge_semantics;
use super::unread;
use crate::domain::elements::service::{kind_at, positions};
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::project::types::ProjectEngine;
use ryo_relations::domain::relations::constants::TOKEN;
use ryo_relations::domain::relations::service::{KIND_LIMIT, all, folder_of, provenance_of};
use ryo_relations::domain::relations::types::Provenance;
use ryo_relations::domain::relations::types::{FileId, Relation};

/// Holds a language to the whole catalog, with its fixtures under `fixtures`.
///
/// # Panics
///
/// When any answer of the language is not held by its fixtures: every one is printed as a to-do line first, and the
/// test fails without a panic trace.
pub fn conformance(
    name: &str,
    parse: &dyn ParseEngine,
    language: &dyn ProjectEngine,
    fixtures: &Path,
    expected: &Fixtures,
) {
    conformance_with(name, parse, language, fixtures, expected, &|_| Ok(None));
}

/// Run conformance with language-owned, per-input fixture configuration.
///
/// # Panics
/// When any fixture or its declared configuration fails the conformance contract.
pub fn conformance_with(
    name: &str,
    parse: &dyn ParseEngine,
    language: &dyn ProjectEngine,
    fixtures: &Path,
    expected: &Fixtures,
    configure: &Configuration<'_>,
) {
    let selection = Selection {
        default: parse,
        configure,
    };
    let mut findings = Findings(Vec::new());
    let bless = if std::env::args().any(|each| each == "--bless") {
        Blessing::Writes
    } else {
        Blessing::Holds
    };
    // The refusals and the flow of the contract hold for every language, over the samples its fixtures give.
    check(parse);
    let samples = samples_in(fixtures);
    if !samples.is_empty() {
        flow(parse, &samples);
    }
    if parse.kind_count().0 > KIND_LIMIT {
        findings.0.push(format!(
            "kind_count is {}, more than the {KIND_LIMIT} kinds a walk can tell apart",
            parse.kind_count().0
        ));
    }
    let syntax: Vec<Relation> = all()
        .iter()
        .copied()
        .filter(|relation| {
            provenance_of(*relation) != Provenance::Engine
                && provenance_of(*relation) != Provenance::Manifests
        })
        .collect();
    let home = fixtures.parent().and_then(Path::parent).unwrap_or(fixtures);
    report_not_yet_read(name, home, parse, language, &mut findings);
    judge_presence(parse, language, fixtures, &mut findings);
    judge_reasons(parse, language, &mut findings);
    judge_classes(parse, language, &mut findings);
    let semantic_reasons = judge_semantics(parse, &expected.semantic, &mut findings);
    judge_unknowns(parse, expected, &semantic_reasons, &mut findings);
    if syntax
        .iter()
        .any(|relation| parse_absence(parse, *relation).is_none())
    {
        judge_kinds(name, parse, fixtures, expected, &mut findings);
    }
    for relation in &syntax {
        judge_syntax(
            &selection,
            fixtures,
            *relation,
            &expected.relations,
            bless,
            &mut findings,
        );
    }
    for directory in ["relations"] {
        for folder in folders_in(&fixtures.join(directory)) {
            if !syntax
                .iter()
                .any(|relation| *folder_of(*relation) == *folder)
            {
                findings
                    .0
                    .push(format!("{directory}/{folder}: no relation of the catalog is read from trees under this name"));
            }
        }
    }
    judge_rails(&selection, fixtures, expected, &mut findings);
    if bless == Blessing::Writes {
        // The run wrote the expected rows of the relations; what the manifests give is held in a run that does not.
        report(&findings);
        return;
    }
    judge_columns(parse, language, expected, &mut findings);
    let rows = manifest_rows(
        parse,
        language,
        fixtures,
        &expected.manifests,
        &mut findings,
    );
    for relation in all()
        .iter()
        .copied()
        .filter(|relation| provenance_of(*relation) == Provenance::Manifests)
    {
        let produced = rows.count(relation);
        let asked = language.absence_of(relation);
        if asked.is_none() && produced == 0 {
            findings.0.push(format!(
                "{}: answers Reads but no tests/fixtures/manifests/ case gives a row of it",
                folder_of(relation)
            ));
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

#[cfg(test)]
#[path = "conformance_test.rs"]
mod tests;
