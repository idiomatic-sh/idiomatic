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
use super::contract::{check, flow};
use super::expected::{Cell, Expected, Fixtures, expected_source, shown as row_shown};
use super::findings::{Findings, folders_in, input_in, read_as};
use super::judgments::{judge_classes, judge_columns, judge_reasons, judge_unknowns};
use super::manifests::manifest_rows;
use super::not_yet_read::report_not_yet_read;
use super::rails::judge_rails;
use super::rows::{compare, rows_of, rows_of_relation};
use super::samples::samples_in;
use super::unread;
use crate::domain::elements::service::{kind_at, positions};
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use crate::domain::relations::constants::TOKEN;
use crate::domain::relations::service::{KIND_LIMIT, all, folder_of, provenance_of};
use crate::domain::relations::types::Provenance;
use crate::domain::relations::types::{FileId, Relation};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::project::types::ProjectEngine;

/// Whether this run keeps the rows the language writes as the rows it expects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Blessing {
    /// It does: the `expected.rs` of each relation is written from what the language wrote (`--bless`).
    Writes,
    /// It does not: the rows are held to what the language expects.
    Holds,
}

/// Judges the answer of the language for one relation read from trees.
fn judge_syntax(
    parse: &dyn ParseEngine,
    fixtures: &Path,
    relation: Relation,
    expected: &[Expected],
    bless: Blessing,
    findings: &mut Findings,
) {
    let folder = folder_of(relation);
    let positive = fixtures.join("relations").join(folder);
    if parse_absence(parse, relation).is_some() {
        if positive.exists() {
            findings.0.push(format!(
            "{folder}: answers NotInThisLanguage but tests/fixtures/relations/{folder}/ shows the construct: delete it, or answer Reads"
        ));
        }
        return;
    }
    let Some(input) = input_in(&positive) else {
        findings.0.push(format!(
            "{folder}: answers Reads but tests/fixtures/relations/{folder}/ has no input"
        ));
        return;
    };
    let name = input
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let shown = read_as(&format!("{folder}/{name}"));
    let Some(read) = rows_of(parse, &input, &shown, findings) else {
        return;
    };
    if bless == Blessing::Writes {
        let mut written = rows_of_relation(&read, relation, &shown);
        written.sort();
        let target = positive.join(super::findings::EXPECTED_FILE);
        let source = expected_source(relation, &name, &written);
        if let Err(error) = std::fs::write(&target, source) {
            findings.0.push(format!(
                "{folder}: cannot write {}: {error}",
                target.display()
            ));
        }
        return;
    }
    let mut wanted: Vec<Vec<Cell>> = expected
        .iter()
        .filter(|each| each.relation == &*folder)
        .flat_map(|each| each.rows.clone())
        .collect();
    wanted.sort();
    if wanted.is_empty() {
        findings.0.push(format!(
            "{folder}: answers Reads but the language expects no row of it (`Expected` for `{folder}`)"
        ));
    }
    compare(
        &format!("{folder}: tests/fixtures/relations/{folder}/"),
        &rows_of_relation(&read, relation, &shown),
        &wanted,
        &|row| row_shown(relation, row),
        findings,
    );
}

/// Holds that a linked language is a language: it reads at least one relation, it has samples of what it reads and recognizes them, and
/// its trees have kinds. A crate that answers every relation as not in it, reads no file and has no fixtures is none, and fails here.
fn judge_presence(
    parse: &dyn ParseEngine,
    language: &dyn ProjectEngine,
    fixtures: &Path,
    findings: &mut Findings,
) {
    let reads = all().iter().any(|relation| {
        provenance_of(*relation) != Provenance::Engine
            && if provenance_of(*relation) == Provenance::Manifests {
                language.absence_of(*relation).is_none()
            } else {
                parse_absence(parse, *relation).is_none()
            }
    });
    if !reads {
        findings.0.push(
            "the language reads no relation: every relation is answered as not in it".to_owned(),
        );
    }
    if parse.kind_count().0 == 0 {
        findings
            .0
            .push("kind_count is 0: the trees of the language have no kinds".to_owned());
    }
    let names = parse.kind_names().0;
    if names.len() != parse.kind_count().0 {
        findings.0.push(format!(
            "kind_names lists {} names but kind_count is {}: each kind has one name",
            names.len(),
            parse.kind_count().0
        ));
    }
    let distinct: BTreeSet<&str> = names.iter().map(|name| name.0).collect();
    if distinct.len() != names.len() {
        findings
            .0
            .push("kind_names lists a name twice: a name is one kind".to_owned());
    }
    let samples = samples_in(fixtures);
    if samples.is_empty() {
        findings.0.push(
            "tests/fixtures has no sample file: a relation's input.<ext> or a file of allocation/"
                .to_owned(),
        );
    }
    for sample in &samples {
        let request = ParseRequest {
            path: sample.path,
            file: FileId(0),
            bytes: sample.bytes,
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        };
        if parse.recognition(&request) == Recognition::Declines {
            findings.0.push(format!(
                "{}: a sample the language does not recognize: give it a path the language reads",
                sample.path
            ));
        }
    }
}

/// The extension of the language's files, from the input of the first relation it reads.
fn extension_of(fixtures: &Path) -> String {
    folders_in(&fixtures.join("relations"))
        .iter()
        .filter_map(|folder| input_in(&fixtures.join("relations").join(folder)))
        .find_map(|input| {
            input
                .extension()
                .map(|each| each.to_string_lossy().into_owned())
        })
        .unwrap_or_default()
}

/// Holds every kind of the language's trees to its sample: the sample's tree holds an element of the kind, a kind no reader looks at
/// is marked unread with a reason, and there is a sample for each kind.
fn judge_kinds(
    name: &str,
    parse: &dyn ParseEngine,
    fixtures: &Path,
    expected: &Fixtures,
    findings: &mut Findings,
) {
    let declared: BTreeSet<u16> = (0..parse.kind_count().0)
        .filter_map(|number| u16::try_from(number).ok())
        .collect();
    let shown: BTreeSet<u16> = expected.kinds.iter().map(|each| each.bits.0).collect();
    if shown.len() != expected.kinds.len() {
        findings.0.push(format!(
            "the language shows {} kinds but only {} of them are different: each kind has one sample or one Sample::Unread",
            expected.kinds.len(),
            shown.len()
        ));
    }
    for number in declared.difference(&shown) {
        findings.0.push(format!(
            "kind {number} of the {} the language declares has no sample: show it with a sample or Sample::Unread",
            declared.len()
        ));
    }
    for number in shown.difference(&declared) {
        findings.0.push(format!(
            "kind {number} is shown but the language declares only {} kinds (kind_count)",
            declared.len()
        ));
    }
    findings
        .0
        .extend(unread::findings(name, &expected.kinds, &kinds_read(parse)));
    let path = format!("kinds/sample.{}", extension_of(fixtures));
    for each in &expected.kinds {
        let (source, token) = match each.sample {
            Sample::Node(source) => (source, false),
            Sample::Token(source) => (source, true),
            Sample::Unread(_) => continue,
        };
        let request = ParseRequest {
            path: SourcePath(&path),
            file: FileId(0),
            bytes: ByteSlice(source.as_bytes()),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        };
        let (mut table, mut symbols) = (Elements::default(), Symbols::default());
        let held = build(parse, request, &mut table, &mut symbols).is_ok()
            && positions(&table).any(|at| {
                kind_at(&table, at) == each.bits
                    && (crate::domain::elements::service::flags_at(&table, at).0 & TOKEN.0 != 0)
                        == token
            });
        if let Some(outside) = positions(&table)
            .map(|at| kind_at(&table, at).0)
            .find(|number| !declared.contains(number))
        {
            findings.0.push(format!(
                "the sample of kind {} holds an element of kind {outside}, which the language does not declare (kind_count is {})",
                each.bits.0,
                declared.len()
            ));
        }
        if !held {
            findings.0.push(format!(
                "{name}: the sample of `{}` ({}) has no {} of that kind",
                each.kind,
                source.escape_debug(),
                if token { "token" } else { "node" }
            ));
        }
    }
}

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
    judge_unknowns(parse, expected, &mut findings);
    if syntax
        .iter()
        .any(|relation| parse_absence(parse, *relation).is_none())
    {
        judge_kinds(name, parse, fixtures, expected, &mut findings);
    }
    for relation in &syntax {
        judge_syntax(
            parse,
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
    judge_rails(parse, fixtures, expected, &mut findings);
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
