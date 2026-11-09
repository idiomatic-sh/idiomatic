//! What every implementation of the `parse` use case's trait must pass.

#![expect(
    clippy::expect_used,
    reason = "a contract run over the samples fails by panicking: the panic is the failure of the check that calls it"
)]

use super::super::errors::ParseError;
use super::super::types::ParseEngine;
use super::super::types::{ParseRequest, Recognition, SourceFile};
use super::collect::Collect;
use super::row_checks::{Scenario, read, rows, rows_on};
use super::validators::{
    embedded_roots_valid, filtered_projection_valid, identifier_valid, projection_preserves_native,
    qualified_name_valid, semantic_spans_valid, shape_representatives_valid, well_formed,
};
use crate::domain::files::types::SourcePath;
use crate::domain::symbols::types::ByteSlice;
use crate::inbound::use_cases::parse::types::{FieldRequest, NoteRequest};

use super::super::service::{absence, build, read as read_file};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::elements::service::{end_of, positions};
use crate::domain::elements::types::Elements;
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::collections::relations::interface::RelationsItem;
use ryo_relations::domain::relations::service::{all, columns, is_declaring, provenance_of};
use ryo_relations::domain::relations::types::Provenance;
use ryo_relations::domain::relations::types::{
    Bits, FileId, Missing, Relation, RelationDeclaring, Rows,
};

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

/// What `Discard` writes rows with, one `Rows` for each row type of the catalog, generated from its one list.
macro_rules! discarders {
    (
        $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )*
    ) => {
        $( $(
            impl Rows<ryo_relations::domain::relations::collections::relations::$module::item::$row> for Discard {
                fn write(&mut self, _row: ryo_relations::domain::relations::collections::relations::$module::item::$row) {}

                fn partial(&mut self, _cause: ryo_relations::domain::relations::types::ReportText<'_>, _missing: Missing) {}

                fn partial_at(&mut self, _file: ryo_relations::domain::relations::types::FileId, _cause: ryo_relations::domain::relations::types::ReportText<'_>, _missing: Missing) {}
            }
        )* )*
    };
}

ryo_relations::each_relation!(discarders);

/// Runs the flow rows against one implementation: every sample gives a well-formed pre-order
/// table, and the same rows whether one worker reads the samples or ten.
///
/// # Panics
///
/// When the implementation breaks the contract, refuses a sample or a worker panics.
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
    let on = |files: &[(usize, SourceFile)], workers: usize| {
        let read = rows_on(subject, files, workers);
        assert!(
            read.is_ok(),
            "a sample is a file the language reads: {read:?}"
        );
        read.unwrap_or_default()
    };
    let alone = on(&numbered, 1);
    assert!(!alone.is_empty(), "the samples give no rows to compare");
    assert_eq!(
        alone,
        on(&numbered, 10),
        "one worker and ten give different rows"
    );
    assert_eq!(
        alone,
        on(&reversed, 3),
        "the files read in reverse by three workers give different rows"
    );
    assert_eq!(
        alone,
        on(&rotated, 10),
        "the files read in another order by ten workers give different rows"
    );
}

#[cfg(test)]
#[path = "contract_test.rs"]
mod tests;
