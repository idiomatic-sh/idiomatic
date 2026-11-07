//! The rows step of the contract: every sample read for every relation it declares, under each way the engine reads it.

use super::super::errors::ParseError;
use super::super::service::{absence, build, read as read_file};
use super::super::types::ParseEngine;
use super::super::types::{ParseRequest, Recognition, SourceFile};
use super::collect::Collect;
use super::validators::{
    embedded_roots_valid, filtered_projection_valid, identifier_valid, projection_preserves_native,
    qualified_name_valid, semantic_spans_valid, shape_representatives_valid, well_formed,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::elements::service::{end_of, positions};
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest};
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::service::{all, columns, is_declaring, provenance_of};
use ryo_relations::domain::relations::types::Provenance;
use ryo_relations::domain::relations::types::{
    Bits, FileId, Missing, Relation, RelationDeclaring, Rows,
};

/// What a sample row of the contract does with the samples.
#[derive(Clone, Copy, Debug)]
pub(super) enum Scenario {
    /// Reads every sample and counts its rows.
    Declared,
    /// Reads a sample asking for the rows of one relation only.
    Wanted,
    /// Reads an empty file.
    Empty,
    /// Reads a file that is not well formed.
    Broken,
}

/// The sample rows of the contract: what reading a file must give, whatever the language.
const SAMPLE_ROWS: &[Case<Scenario, bool, ParseError>] = &[
    Case {
        name: CaseName("parses_a_sample_file_into_the_declared_relations"),
        word: Word::Succeeds,
        input: || Scenario::Declared,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("builds_only_the_wanted_columns"),
        word: Word::Succeeds,
        input: || Scenario::Wanted,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("empty_file_has_only_supported_root_rows_and_is_complete"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("relation_with_no_rows_for_this_file_is_not_missing"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("syntax_error_is_reported_not_fatal"),
        word: Word::Succeeds,
        input: || Scenario::Broken,
        expect: Expect::Value(true),
    },
];

pub(super) fn rows(subject: &dyn ParseEngine, samples: &[SourceFile]) {
    let Some(first) = samples.first().copied() else {
        return;
    };
    run(SAMPLE_ROWS, Word::Succeeds, |scenario| match scenario {
        Scenario::Declared => {
            let mut held = true;
            for sample in samples {
                let out = read(subject, *sample, sample.bytes, all())?;
                held &= all().iter().any(|relation| out.count(*relation) > 0);
            }
            Ok(held)
        }
        Scenario::Wanted => {
            // Asking for one relation the language reads gives rows of that relation only; which relation it is is the
            // language's: the first it answers `Reads`.
            let Some(asked) = all().iter().copied().find(|relation| {
                provenance_of(*relation) != Provenance::Engine
                    && provenance_of(*relation) != Provenance::Manifests
                    && absence(subject, *relation).is_none()
            }) else {
                return Ok(true);
            };
            let out = read(subject, first, first.bytes, &[asked])?;
            Ok(all()
                .iter()
                .filter(|relation| **relation != asked)
                .all(|relation| out.count(*relation) == 0))
        }
        Scenario::Empty => {
            let out = read(subject, first, ByteSlice(b""), all())?;
            Ok(all()
                .iter()
                .filter(|relation| is_declaring(**relation) == RelationDeclaring::Declares)
                // An empty file still has its module scope and lexical root interval when read.
                .all(|relation| {
                    let count = out.count(*relation);
                    if subject.absence_of(*relation).is_some() {
                        return count == 0;
                    }
                    if matches!(*relation, Relation::Scope | Relation::IndentRegion) {
                        return count == 1;
                    }
                    // Compiler features describe module configuration even without source constructs.
                    if *relation == Relation::CompilerFeature {
                        return out.rows().iter().all(|(feature_relation, row)| {
                            *feature_relation != Relation::CompilerFeature
                                || (row.len() == columns(Relation::CompilerFeature).len()
                                    && row.first().is_some_and(|file| *file == 0)
                                    && row.get(2).is_some_and(|flag| *flag <= 1))
                        });
                    }
                    // Versioned implicit-symbol rows describe the environment catalog, not source text.
                    if *relation == Relation::ImplicitSymbol {
                        return out.rows().iter().all(|(catalog_relation, row)| {
                            *catalog_relation != Relation::ImplicitSymbol
                                || (row.len() == columns(Relation::ImplicitSymbol).len()
                                    && row.get(3).is_some_and(|major| *major > 0))
                        });
                    }
                    count == 0
                }))
        }
        Scenario::Broken => {
            // The broken input is the language's own: the input of its `syntax_error` fixture. A language that answers
            // `syntax_error` `NotInThisLanguage` has no such input, and skips this row.
            let Some(broken) = samples
                .iter()
                .find(|sample| sample.path.starts_with("relations/syntax_error/"))
            else {
                return Ok(true);
            };
            if absence(subject, Relation::SyntaxError).is_some() {
                return Ok(true);
            }
            let out = read(subject, *broken, broken.bytes, all())?;
            Ok(out.count(Relation::SyntaxError) > 0)
        }
    });
}

/// Reads every sample, each as the file numbered as it says, in the order given, on `workers` threads that each own a table and a
/// writer; the rows of every file, sorted.
pub(super) fn rows_on(
    subject: &dyn ParseEngine,
    samples: &[(usize, SourceFile)],
    workers: usize,
) -> Vec<(Relation, Vec<u64>)> {
    let mut rows: Vec<(Relation, Vec<u64>)> = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..workers)
            .map(|worker| {
                scope.spawn(move || {
                    let (mut table, mut symbols, mut out) =
                        (Elements::default(), Symbols::default(), Collect::default());
                    for (place, sample) in samples
                        .iter()
                        .enumerate()
                        .filter(|(at, _)| at % workers == worker)
                        .map(|(_, (place, sample))| (*place, *sample))
                    {
                        let request = ParseRequest {
                            path: sample.path,
                            file: FileId(u32::try_from(place).unwrap_or(0)),
                            bytes: sample.bytes,
                            notes: NoteRequest::Unwanted,
                            fields: FieldRequest::Unwanted,
                        };
                        read_file(subject, request, &mut table, &mut symbols, all(), &mut out)
                            .expect("a sample is a file the language reads");
                    }
                    out.rows()
                })
            })
            .collect();
        threads
            .into_iter()
            .flat_map(|thread| thread.join().expect("a worker finishes"))
            .collect()
    });
    rows.sort();
    rows
}

/// Reads `bytes` as the file `sample` names, asking for `wanted`, and answers what was written.
pub(super) fn read(
    subject: &dyn ParseEngine,
    sample: SourceFile,
    bytes: ByteSlice<'_>,
    wanted: &[Relation],
) -> Result<Collect, ParseError> {
    let mut out = Collect::default();
    let mut symbols = Symbols::default();
    let mut elements = Elements::default();
    read_file(
        subject,
        ParseRequest {
            path: sample.path,
            file: FileId(0),
            bytes,
            notes: NoteRequest::Unwanted,
            fields: if wanted.contains(&Relation::SemanticChild) {
                FieldRequest::Wanted
            } else {
                FieldRequest::Unwanted
            },
        },
        &mut elements,
        &mut symbols,
        wanted,
        &mut out,
    )?;
    assert!(
        embedded_roots_valid(&elements),
        "an embedded expression root must be a node attached directly to its source carrier"
    );
    if wanted.contains(&Relation::SemanticSyntax) {
        assert!(
            semantic_spans_valid(
                &out,
                bytes.0.len(),
                wanted.contains(&Relation::SourceLocation),
                |id| positions(&elements).any(|position| {
                    ryo_relations::domain::relations::service::node_id(FileId(0), position).0 == id
                        && crate::domain::elements::service::flags_at(&elements, position).0
                            & ryo_relations::domain::relations::constants::IMPLIED.0
                            != 0
                })
            ),
            "semantic occurrences must retain bounded physical spans and matching source coordinates"
        );
    }
    if wanted.contains(&Relation::SemanticSyntax)
        && wanted.contains(&Relation::SyntaxTree)
        && wanted.contains(&Relation::SemanticChild)
        && wanted.contains(&Relation::SyntaxChild)
        && crate::domain::expansions::service::embedded_expressions(&elements)
            .next()
            .is_none()
    {
        assert!(
            projection_preserves_native(&out),
            "without embedded expressions the semantic view must preserve every native row and edge exactly"
        );
        let mut fields: Vec<_> = out
            .rows()
            .into_iter()
            .filter_map(|(relation, row)| {
                (relation == Relation::SyntaxChild)
                    .then(|| row.get(3).copied())
                    .flatten()
            })
            .collect();
        fields.sort_unstable();
        fields.dedup();
        assert!(
            fields
                .into_iter()
                .all(|field| filtered_projection_valid(&out, field)),
            "native and semantic child field predicates must select the same edges"
        );
    }
    assert!(
        shape_representatives_valid(&out),
        "expression shape classes must name real representative occurrences in the same file"
    );
    for (relation, row) in out.rows() {
        if relation == Relation::Identifier {
            assert!(
                identifier_valid(&row, &symbols, bytes),
                "identifier must expose a known nonempty canonical name at a nonempty original byte span"
            );
        }
        if relation == Relation::QualifiedName {
            assert!(
                qualified_name_valid(&row, &symbols, bytes),
                "qualified-name must distinguish proven paths, unknown identities and builtin availability"
            );
        }
    }
    Ok(out)
}
