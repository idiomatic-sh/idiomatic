// ignore: idiomatic.arch/file-length - the contract is one check run in order over the samples, and its steps (rows, flow, determinism) share the reads and the sink they run against.
//! What every implementation of the `parse` use case's trait must pass.

#![expect(
    clippy::expect_used,
    reason = "a contract run over the samples fails by panicking: the panic is the failure of the check that calls it"
)]

use super::super::errors::ParseError;
use super::super::types::ParseEngine;
use super::super::types::{ParseRequest, Recognition, SourceFile};
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest};

use super::super::service::{absence, build, read as read_file};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::elements::service::{end_of, positions};
use crate::domain::elements::types::Elements;
use crate::domain::relations::collections::relations::interface::RelationsItem;
use crate::domain::relations::service::{all, columns, is_declaring, provenance_of};
use crate::domain::relations::types::Provenance;
use crate::domain::relations::types::{Bits, FileId, Missing, Relation, RelationDeclaring, Rows};
use crate::domain::symbols::types::Symbols;

/// A writer that keeps nothing: the contract looks at what the language answers, not at what it wrote.
#[derive(Default, Clone, Copy, Debug)]
pub struct Discard;

/// What a row of the contract is given: the path of a file.
type Input = &'static str;
/// What a row of the contract expects when it succeeds.
type Value = ();
/// What a row of the contract expects when it is refused.
type Failure = ParseError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_file_the_language_does_not_read_is_unsupported"),
    word: Word::Refuses,
    input: || "contract.not-a-language",
    expect: Expect::Error(ParseError::Unsupported),
}];

/// Runs the contract's rows against one implementation, in the crate that implements the trait.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(subject: &dyn ParseEngine) {
    run(CASES, Word::Refuses, |path| {
        let request = ParseRequest {
            path: SourcePath(path),
            file: FileId(0),
            bytes: ByteSlice(b"anything"),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        };
        assert!(
            subject.recognition(&request) == Recognition::Declines,
            "the language recognizes {path}, a file it refuses"
        );
        build(
            subject,
            request,
            &mut Elements::default(),
            &mut Symbols::default(),
        )
    });
}

/// A writer that keeps the rows it is given, each as the relation it is of and its numbers, one
/// row after another.
#[derive(Default, Debug)]
pub struct Collect {
    /// The relation of each row, in the order they were written.
    pub relations: Vec<Relation>,
    /// The numbers of every row, one row after another.
    pub words: Vec<u64>,
    /// The relations said to be partly read, each with its cause.
    pub partials: Vec<(Relation, String, Missing)>,
    /// The relations said to be partly read in one file of the run, each with the file and its cause.
    pub partials_at: Vec<(Relation, FileId, String)>,
}

impl Collect {
    /// Forgets the rows, keeping the room.
    pub fn clear(&mut self) {
        self.relations.clear();
        self.words.clear();
        self.partials.clear();
    }

    /// How many rows were written of `relation`.
    #[must_use]
    pub fn count(&self, relation: Relation) -> usize {
        self.relations
            .iter()
            .filter(|each| **each == relation)
            .count()
    }

    /// The rows, each a relation and its numbers.
    #[must_use]
    pub fn rows(&self) -> Vec<(Relation, Vec<u64>)> {
        let mut rest = self.words.as_slice();
        self.relations
            .iter()
            .map(|relation| {
                let (row, after) = rest.split_at(columns(*relation).len());
                rest = after;
                (*relation, row.to_vec())
            })
            .collect()
    }

    /// Keeps one row: its relation, and its words after those of the rows kept before.
    fn keep(&mut self, relation: Relation, words: &[Bits]) {
        self.relations.push(relation);
        self.words.extend(words.iter().map(|word| word.0));
    }
}

/// What `Discard` and `Collect` write rows with, one `Rows` for each row type of the catalog, generated from its one list.
macro_rules! writers {
    (
        $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )*
    ) => {
        $( $(
            impl Rows<$crate::domain::relations::collections::relations::$module::item::$row> for Discard {
                fn write(&mut self, _row: $crate::domain::relations::collections::relations::$module::item::$row) {}

                fn partial(&mut self, _cause: $crate::domain::relations::types::ReportText<'_>, _missing: Missing) {}

                fn partial_at(&mut self, _file: $crate::domain::relations::types::FileId, _cause: $crate::domain::relations::types::ReportText<'_>, _missing: Missing) {}
            }

            impl Rows<$crate::domain::relations::collections::relations::$module::item::$row> for Collect {
                fn write(&mut self, row: $crate::domain::relations::collections::relations::$module::item::$row) {
                    let words = <$crate::domain::relations::collections::relations::$module::item::$variant as RelationsItem>::words(row);
                    self.keep(Relation::$variant, words.as_ref());
                }

                fn partial(&mut self, cause: $crate::domain::relations::types::ReportText<'_>, missing: Missing) {
                    if !self.partials.iter().any(|(relation, each, how)| *relation == Relation::$variant && each == cause.0 && *how == missing) {
                        self.partials.push((Relation::$variant, cause.0.to_owned(), missing));
                    }
                }

                fn partial_at(&mut self, file: $crate::domain::relations::types::FileId, cause: $crate::domain::relations::types::ReportText<'_>, _missing: Missing) {
                    if !self.partials_at.iter().any(|(relation, at, each)| *relation == Relation::$variant && *at == file && each == cause.0) {
                        self.partials_at.push((Relation::$variant, file, cause.0.to_owned()));
                    }
                }
            }
        )* )*
    };
}

crate::each_relation!(writers);

/// What a sample row of the contract does with the samples.
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
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
        name: CaseName("empty_file_gives_zero_rows_and_is_complete"),
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

/// Reads `bytes` as the file `sample` names, asking for `wanted`, and answers what was written.
fn read(
    subject: &dyn ParseEngine,
    sample: SourceFile,
    bytes: ByteSlice<'_>,
    wanted: &[Relation],
) -> Result<Collect, ParseError> {
    let mut out = Collect::default();
    read_file(
        subject,
        ParseRequest {
            path: sample.path,
            file: FileId(0),
            bytes,
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        },
        &mut Elements::default(),
        &mut Symbols::default(),
        wanted,
        &mut out,
    )?;
    Ok(out)
}

/// Runs the sample rows against one implementation.
fn rows(subject: &dyn ParseEngine, samples: &[SourceFile]) {
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
                // An empty file still has its module, which is a scope: the one row it gives.
                .all(|relation| {
                    out.count(*relation)
                        == usize::from(
                            *relation == Relation::Scope
                                && subject.absence_of(Relation::Scope).is_none(),
                        )
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

/// Whether `elements` is a pre-order tree: every element's subtree lies inside the table and
/// inside its parent's, and the first element is its own parent.
fn well_formed(elements: &Elements) -> bool {
    let count = end_of(elements);
    positions(elements).all(|position| {
        let end = crate::domain::elements::service::last_at(elements, position);
        let parent = crate::domain::elements::service::parent_at(elements, position);
        end < count
            && (parent == position
                || (parent < position
                    && end <= crate::domain::elements::service::last_at(elements, parent)))
    })
}

/// Reads every sample, each as the file numbered as it says, in the order given, on `workers` threads that each own a table and a
/// writer; the rows of every file, sorted.
fn rows_on(
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

/// Runs the flow rows against one implementation: every sample gives a well-formed pre-order
/// table, and the same rows whether one worker reads the samples or ten.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn flow(subject: &dyn ParseEngine, samples: &[SourceFile]) {
    for sample in samples {
        let (mut table, mut symbols) = (Elements::default(), Symbols::default());
        let request = ParseRequest {
            path: sample.path,
            file: FileId(0),
            bytes: sample.bytes,
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Unwanted,
        };
        read_file(
            subject,
            request,
            &mut table,
            &mut symbols,
            all(),
            &mut Collect::default(),
        )
        .expect("a sample is a file the language reads");
        assert!(
            well_formed(&table),
            "the table of {} is not a well-formed pre-order tree",
            sample.path
        );
    }
    rows(subject, samples);
    // The module of a file is a scope that encloses itself: of the scopes of each file, exactly one is its own parent (parent == child), and every other has a
    // parent that is a scope of the file.
    if subject.absence_of(Relation::Scope).is_none() {
        for sample in samples {
            let out = read(subject, *sample, sample.bytes, &[Relation::Scope])
                .expect("a sample is a file the language reads");
            let scopes: Vec<(u64, u64)> = out
                .rows()
                .into_iter()
                .filter(|(relation, _)| *relation == Relation::Scope)
                .filter_map(|(_, row)| match row.as_slice() {
                    [scope, parent, ..] => Some((*scope, *parent)),
                    _ => None,
                })
                .collect();
            let self_parented = scopes
                .iter()
                .filter(|(parent, child)| parent == child)
                .count();
            assert_eq!(
                self_parented, 1,
                "{} has {self_parented} rows that are their own parent, and every file has exactly one: its module",
                sample.path
            );
            assert!(
                scopes
                    .iter()
                    .all(|(parent, _)| scopes.iter().any(|(_, child)| child == parent)),
                "a scope of {} has a parent that is no scope of the file",
                sample.path
            );
        }
    }
    // The rows of a file are the same whichever worker reads it and whichever files it read before: the samples numbered as they
    // are, read in order by one worker, in reverse by three, and rotated by ten.
    let numbered: Vec<(usize, SourceFile)> = samples.iter().copied().enumerate().collect();
    let mut reversed = numbered.clone();
    reversed.reverse();
    let mut rotated = numbered.clone();
    rotated.rotate_left(numbered.len() / 2);
    let alone = rows_on(subject, &numbered, 1);
    assert!(!alone.is_empty(), "the samples give no rows to compare");
    assert_eq!(
        alone,
        rows_on(subject, &numbered, 10),
        "one worker and ten give different rows"
    );
    assert_eq!(
        alone,
        rows_on(subject, &reversed, 3),
        "the files read in reverse by three workers give different rows"
    );
    assert_eq!(
        alone,
        rows_on(subject, &rotated, 10),
        "the files read in another order by ten workers give different rows"
    );
}
