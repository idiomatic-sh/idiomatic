//! The judgments of a language's answers for the relations its fixtures read from trees, from presence and from kinds.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::Path;

use super::super::service::{absence as parse_absence, build, kinds_read};
use super::super::types::ParseEngine;
use super::super::types::Sample;
use super::super::types::{FieldRequest, NoteRequest, ParseRequest, Recognition};
use super::configuration::{Configuration, Selection};
use super::contract::{check, flow};
use super::expected::{Cell, Expected, Fixtures};
use super::findings::{Findings, folders_in, input_in, inputs_in};
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

/// Whether this run keeps the rows the language writes as the rows it expects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Blessing {
    /// It does: the `expected.rs` of each relation is written from what the language wrote (`--bless`).
    Writes,
    /// It does not: the rows are held to what the language expects.
    Holds,
}

/// Judges the answer of the language for one relation read from trees.
pub(super) fn judge_syntax(
    selection: &Selection<'_>,
    fixtures: &Path,
    relation: Relation,
    expected: &[Expected],
    bless: Blessing,
    findings: &mut Findings,
) {
    let parse = selection.default;
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
    let inputs = inputs_in(&positive);
    if inputs.is_empty() {
        findings.0.push(format!(
            "{folder}: answers Reads but tests/fixtures/relations/{folder}/ has no input"
        ));
        return;
    }
    let Some(written) = rows_of_inputs_with(selection, &inputs, relation, &folder, findings) else {
        return;
    };
    if bless == Blessing::Writes {
        let target = positive.join(super::findings::EXPECTED_FILE);
        let names = inputs
            .iter()
            .filter_map(|path| path.file_name())
            .map(|name| name.to_string_lossy())
            .collect::<Vec<_>>()
            .join(", ");
        let source = expected_source(relation, &names, &written);
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
        &written,
        &wanted,
        &|row| row_shown(relation, row),
        findings,
    );
}

/// Holds that a linked language is a language: it reads at least one relation, it has samples of what it reads and recognizes them, and
/// its trees have kinds. A crate that answers every relation as not in it, reads no file and has no fixtures is none, and fails here.
pub(super) fn judge_presence(
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
pub(super) fn extension_of(fixtures: &Path) -> String {
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
pub(super) fn judge_kinds(
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
        // A kind may hold only elements of a tree the language parsed under one of its own (a pattern under its carrier).
        let held = build(parse, request, &mut table, &mut symbols).is_ok()
            && positions(&table)
                .chain(crate::domain::elements::service::expanded_positions(&table))
                .any(|at| {
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
