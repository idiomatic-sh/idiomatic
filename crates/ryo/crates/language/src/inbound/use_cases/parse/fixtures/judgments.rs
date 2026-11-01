//! The judgments of a language's classes and reasons: what each kind of relation says of a language holds.

use super::super::service::absence as parse_absence;
use super::super::types::ParseEngine;
use super::expected::{Cell, Fixtures, Unfilled, Unshown};
use super::findings::Findings;
use crate::domain::answers::service::vocabulary_of;
use crate::domain::answers::types::AbsenceCause;
use crate::domain::relations::constants::{LOWERED, RAISED};
use crate::domain::relations::service::{
    all, class_of, column_name, columns, folder_of, kind, provenance_of,
};
use crate::domain::relations::types::CellKind;
use crate::domain::relations::types::Class;
use crate::domain::relations::types::Provenance;
use crate::domain::relations::types::Relation;
use crate::domain::terms::service::words_of;
use crate::domain::terms::types::Vocabulary;
use crate::inbound::use_cases::project::types::ProjectEngine;

/// The reasons a language may give for a relation of each class, which is all the engine can tell without knowing the language: the
/// shape of the tree has none, a manifest relation is absent only because there are no manifests, and `NotCode` and
/// `NotAKeyedDocument` are said of code and of keyed documents. Every class has its row, so a class added to the catalog does not
/// compile until it is judged.
pub(super) fn reasons_of(class: Class) -> &'static [AbsenceCause] {
    match class {
        Class::Structure | Class::Engine => &[],
        Class::Code => &[
            AbsenceCause::NoSuchConstruct,
            AbsenceCause::NotCode,
            AbsenceCause::NotYetRead,
        ],
        Class::Keyed => &[
            AbsenceCause::NoSuchConstruct,
            AbsenceCause::NotAKeyedDocument,
            AbsenceCause::NotYetRead,
        ],
        Class::Document => &[
            AbsenceCause::NoSuchConstruct,
            AbsenceCause::NotADocument,
            AbsenceCause::NotYetRead,
        ],
        Class::Literal | Class::Lexical => {
            &[AbsenceCause::NoSuchConstruct, AbsenceCause::NotYetRead]
        }
        Class::Manifest => &[AbsenceCause::NoManifests, AbsenceCause::NotYetRead],
    }
}

/// Holds each answer to the reasons its class takes, and the manifests the language names to the manifest relations it reads.
pub(super) fn judge_classes(
    parse: &dyn ParseEngine,
    language: &dyn ProjectEngine,
    findings: &mut Findings,
) {
    let manifests = !language.manifest_names().is_empty();
    for relation in all().iter().copied() {
        let class = class_of(relation);
        let absence = match class {
            Class::Manifest => language.absence_of(relation),
            Class::Engine => None,
            Class::Structure
            | Class::Code
            | Class::Keyed
            | Class::Literal
            | Class::Lexical
            | Class::Document => parse_absence(parse, relation),
        };
        if let Some(reason) = absence
            && !reasons_of(class).contains(&reason)
        {
            findings.0.push(format!(
                "{}: a relation of {class:?} cannot be absent for {reason:?}",
                folder_of(relation)
            ));
        }
        // A language with no manifests reads no relation of them, and says so; but `source_dependency` and `analyzed_source` are read
        // from the notes the parse leaves of each file, which a language with no manifests leaves too.
        let noted = matches!(
            relation,
            Relation::SourceDependency | Relation::AnalyzedSource
        );
        if class == Class::Manifest && !manifests && absence.is_none() && !noted {
            findings.0.push(format!(
                "{}: answers Reads, but manifests() names no manifest: answer NoManifests",
                folder_of(relation)
            ));
        }
        // A manifest relation a language with manifests does not read has no reason that fits.
        if class == Class::Manifest && manifests && absence == Some(AbsenceCause::NoManifests) {
            findings.0.push(format!(
                "{}: answers NoManifests, but manifests() names manifests",
                folder_of(relation)
            ));
        }
    }
    // A language that names manifests reads something from them.
    if manifests
        && !all().iter().any(|relation| {
            class_of(*relation) == Class::Manifest && language.absence_of(*relation).is_none()
        })
    {
        findings.0.push(
            "manifests() names manifests, but no manifest relation is read from them".to_owned(),
        );
    }
}

/// Holds the reasons the language gives to what it answers: a reason that its answers contradict is no reason.
pub(super) fn judge_reasons(
    parse: &dyn ParseEngine,
    language: &dyn ProjectEngine,
    findings: &mut Findings,
) {
    let answered = |reason: AbsenceCause| {
        all().iter().any(|relation| {
            parse_absence(parse, *relation) == Some(reason)
                || language.absence_of(*relation) == Some(reason)
        })
    };
    let reads = |class: Class| -> Vec<&'static str> {
        all()
            .iter()
            .copied()
            .filter(|relation| {
                class_of(*relation) == class
                    && provenance_of(*relation) != Provenance::Engine
                    && parse_absence(parse, *relation).is_none()
            })
            .map(|relation| folder_of(relation).0)
            .collect()
    };
    if answered(AbsenceCause::NotCode) {
        for read in reads(Class::Code) {
            findings.0.push(format!(
                "{read}: answers Reads but another relation says the language is not code (NotCode)"
            ));
        }
    }
    if answered(AbsenceCause::NotAKeyedDocument) {
        for read in reads(Class::Keyed) {
            findings.0.push(format!("{read}: answers Reads but another relation says the language has no keyed documents (NotAKeyedDocument)"));
        }
    }
    if answered(AbsenceCause::NotADocument) {
        for read in reads(Class::Document) {
            findings.0.push(format!("{read}: answers Reads but another relation says the language is not a language of documents (NotADocument)"));
        }
    }
    if answered(AbsenceCause::NoManifests) && !language.manifest_names().is_empty() {
        findings.0.push(
            "a manifest relation answers NoManifests but manifests() names manifests".to_owned(),
        );
    }
}

/// Holds the reasons the language declares for a value it cannot resolve to its fixtures: each is named once, each is shown by a fixture
/// row that carries it, and no fixture row carries one the language does not declare.
pub(super) fn judge_unknowns(
    parse: &dyn ParseEngine,
    expected: &Fixtures,
    findings: &mut Findings,
) {
    let declared: Vec<&str> = parse
        .unknown_reasons()
        .iter()
        .map(|name| name.text.0)
        .collect();
    let mut numbers = Vec::new();
    for name in parse.unknown_reasons() {
        if numbers.contains(&name.reason) {
            findings.0.push(format!(
                "unknown reason `{}` has the number of another: number each once",
                name.text.0
            ));
        }
        numbers.push(name.reason);
    }
    let mut seen: Vec<&str> = Vec::new();
    for name in &declared {
        if seen.contains(name) {
            findings.0.push(format!(
                "unknown reason `{name}` is declared twice: name each once"
            ));
        }
        seen.push(name);
    }
    let rows = expected
        .relations
        .iter()
        .flat_map(|each| each.rows.iter())
        .chain(
            expected
                .manifests
                .iter()
                .flat_map(|each| each.rows.iter().map(|(_, row)| row)),
        );
    let used: Vec<&String> = rows
        .flatten()
        .filter_map(|cell| match cell {
            Cell::Unknown(reason) => Some(reason),
            Cell::Node(_)
            | Cell::Name(_)
            | Cell::File(_)
            | Cell::Unlocated
            | Cell::Span(..)
            | Cell::Number(_)
            | Cell::Flag(_) => None,
        })
        .collect();
    for name in &declared {
        if !used.iter().any(|reason| reason.as_str() == *name) {
            findings.0.push(format!(
                "unknown reason `{name}` has no fixture: state a row whose value is `unknown(\"{name}\")`"
            ));
        }
    }
    for reason in used {
        if !declared.contains(&reason.as_str()) {
            findings.0.push(format!(
                "a fixture row has the unknown reason `{reason}`, which the language does not declare (`unknown_reasons`)"
            ));
        }
    }
}

/// Whether a cell holds a value: a name or a file that is not empty, a number that is not zero, a raised flag, a node, a located span.
/// A value the language cannot resolve is no value.
fn filled(cell: &Cell) -> bool {
    match cell {
        Cell::Name(text) | Cell::File(text) => !text.is_empty(),
        Cell::Number(number) => number.0 != 0,
        Cell::Flag(flag) => *flag,
        Cell::Node(_) | Cell::Span(..) => true,
        Cell::Unknown(_) | Cell::Unlocated => false,
    }
}

/// What the cells of one column of the fixtures do not show: each word of a closed set none holds, `true` or `false` of a flag none
/// holds, or, for any other column, a value when none is filled.
fn unshown(column: crate::domain::relations::types::Column, cells: &[&Cell]) -> Vec<Unshown> {
    if let Vocabulary::Closed(block) = vocabulary_of(column) {
        return words_of(block)
            .into_iter()
            .filter(|word| {
                !cells
                    .iter()
                    .any(|cell| matches!(cell, Cell::Name(text) if text.as_str() == word.0))
            })
            .map(|word| Unshown::Word(word.0))
            .collect();
    }
    if kind(column) == CellKind::Flag {
        return [(true, RAISED), (false, LOWERED)]
            .into_iter()
            .filter(|(flag, _)| !cells.iter().any(|cell| **cell == Cell::Flag(*flag)))
            .map(|(_, word)| Unshown::Word(word))
            .collect();
    }
    if cells.iter().any(|cell| filled(cell)) {
        Vec::new()
    } else {
        vec![Unshown::Filled]
    }
}

/// Whether the language reads `relation`, from its trees or from its manifests.
fn reads(parse: &dyn ParseEngine, language: &dyn ProjectEngine, relation: Relation) -> bool {
    match provenance_of(relation) {
        Provenance::Engine => false,
        Provenance::Manifests => language.absence_of(relation).is_none(),
        Provenance::Trees => parse_absence(parse, relation).is_none(),
    }
}

/// Holds every column of every relation the language reads to its fixtures: a column of a closed set of words shows each word, a
/// flag shows `true` and `false`, and any other column a value that is not empty, or the language declares what its fixtures cannot
/// show and why (`Fixtures::unfilled`). A declaration of what a fixture shows, of a column the language does not read, or with no
/// reason, is a finding too: a column a language never fills is never silent.
pub(super) fn judge_columns(
    parse: &dyn ParseEngine,
    language: &dyn ProjectEngine,
    expected: &Fixtures,
    findings: &mut Findings,
) {
    let mut held: Vec<&Unfilled> = Vec::new();
    for relation in all().iter().copied() {
        if !reads(parse, language, relation) {
            continue;
        }
        let folder = folder_of(relation).0;
        let rows: Vec<&Vec<Cell>> = if provenance_of(relation) == Provenance::Manifests {
            expected
                .manifests
                .iter()
                .flat_map(|each| each.rows.iter())
                .filter(|(of, _)| *of == folder)
                .map(|(_, row)| row)
                .collect()
        } else {
            expected
                .relations
                .iter()
                .filter(|each| each.relation == folder)
                .flat_map(|each| each.rows.iter())
                .collect()
        };
        if rows.is_empty() {
            // A relation read with no rows is a finding of its own judgment.
            held.extend(
                expected
                    .unfilled
                    .iter()
                    .filter(|each| each.relation == folder),
            );
            continue;
        }
        for (index, column) in columns(relation).into_iter().enumerate() {
            let name = column_name(column).0;
            let cells: Vec<&Cell> = rows.iter().filter_map(|row| row.get(index)).collect();
            let missing = unshown(column, &cells);
            let declared: Vec<&Unfilled> = expected
                .unfilled
                .iter()
                .filter(|each| each.relation == folder && each.column == name)
                .collect();
            for each in &missing {
                if !declared.iter().any(|said| said.unshown == *each) {
                    let what = match each {
                        Unshown::Filled => "a value that is not empty".to_owned(),
                        Unshown::Word(word) => format!("`{word}`"),
                    };
                    findings.0.push(format!(
                        "{folder}.{name}: no fixture shows {what}: show it, or declare it in `unfilled` with the reason it never applies"
                    ));
                }
            }
            for said in declared {
                held.push(said);
                if !missing.contains(&said.unshown) {
                    findings.0.push(format!(
                        "{folder}.{name}: declared unfilled ({:?}), but a fixture shows it: remove the declaration",
                        said.unshown
                    ));
                }
                if said.reason.0.trim().is_empty() {
                    findings.0.push(format!(
                        "{folder}.{name}: declared unfilled with no reason: cite the standard or the construct that never fills it"
                    ));
                }
            }
        }
    }
    for each in &expected.unfilled {
        if !held.iter().any(|said| std::ptr::eq(*said, each)) {
            findings.0.push(format!(
                "{}.{}: declared unfilled, but the language reads no such column",
                each.relation, each.column
            ));
        }
    }
}
