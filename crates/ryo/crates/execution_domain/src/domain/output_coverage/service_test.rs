//! The `output_coverage` service's cases: how far a join, an ordered match and an ordered reduction carry what an operand lacks.
use std::borrow::Cow;

use super::service::{ordered_output_coverage, output_coverage};
use super::types::{InputCoverage, OperandOrdinal};
use crate::domain::completion::types::CoverageState;
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, JoinColumn, Side},
    plans::types::Operation,
};
#[cfg(test)]
mod lineage_test;

/// What a row asks: the coverage of an answer whose operand lacks rows of the files named (none: anywhere).
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// An ordered reduction partitioned by this column of a file and a number column, its operand partial in file 1.
    Ordered(u32),
    /// A join of a complete `function` (node, file, symbol) and an `identifier` (file, symbol) partial in these files, on these
    /// columns of each, answering the left's node and file.
    Join(u32, u32, &'static [Option<FileId>]),
    /// An ordered match of `function` (node, file, number, symbol) with itself on this column, partial in file 1.
    Matched(u32),
}

/// A coverage, as plain values: partial in these files with this many lacks said.
#[derive(Debug, PartialEq)]
enum Covered {
    Complete,
    Partial {
        files: Cow<'static, [Option<FileId>]>,
        lacks: usize,
    },
    Inapplicable(Cow<'static, [FileId]>),
}

type Input = Ask;
type Value = Covered;
type Failure = OperatingError;

fn covered(state: CoverageState) -> Covered {
    match state {
        CoverageState::Complete => Covered::Complete,
        CoverageState::Partial { files, lacks } => Covered::Partial {
            files: files.into(),
            lacks: lacks.len(),
        },
        CoverageState::Inapplicable { files } => Covered::Inapplicable(files.into()),
    }
}

/// Partial in these files, each lacking any row.
fn partial(files: &[Option<FileId>]) -> CoverageState {
    CoverageState::Partial {
        files: files.to_vec(),
        lacks: vec![],
    }
}

/// The two columns every join and match answers: the left's first and second.
fn left_two() -> Vec<JoinColumn> {
    [0, 1]
        .map(|column| JoinColumn {
            side: Side::Left,
            column: ColumnIndex(column),
        })
        .to_vec()
}

fn ordered(key: u32) -> CoverageState {
    let source = InputCoverage {
        ordinal: OperandOrdinal(0),
        kinds: &[CellKind::File, CellKind::Number],
        coverage: partial(&[Some(FileId(1))]),
        lineage: Default::default(),
    };
    ordered_output_coverage(&[ColumnIndex(key)], &[CellKind::File], &source)
}

/// Two files, one syntax-rejected (file 1): a join whose rows are anchored to the file it joins on withholds only the rejected
/// file's partition, so the other file's answer is complete; a join on a key that is no file leaves the missing rows anywhere.
fn joined(
    left_on: u32,
    right_on: u32,
    missing: &[Option<FileId>],
) -> Result<CoverageState, Failure> {
    let join = Operation::Join {
        left: PlanInput::Relation(Relation::Function),
        right: PlanInput::Relation(Relation::Identifier),
        left_on: vec![ColumnIndex(left_on)],
        right_on: vec![ColumnIndex(right_on)],
        output: left_two(),
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    };
    output_coverage(
        &join,
        &[CellKind::Node, CellKind::File],
        &[
            InputCoverage {
                ordinal: OperandOrdinal(0),
                kinds: &[CellKind::Node, CellKind::File, CellKind::Symbol],
                coverage: CoverageState::Complete,
                lineage: Default::default(),
            },
            InputCoverage {
                ordinal: OperandOrdinal(1),
                kinds: &[CellKind::File, CellKind::Symbol],
                coverage: partial(missing),
                lineage: Default::default(),
            },
        ],
    )
}

/// An ordered match pairs rows that agree on every key: keyed by file it withholds only the rejected file's partition, and keyed by
/// a symbol it cannot place the missing rows.
fn matched(key: u32) -> Result<CoverageState, Failure> {
    let matched = Operation::OrderedMatch {
        left: PlanInput::Relation(Relation::Function),
        right: PlanInput::Relation(Relation::Function),
        left_on: vec![ColumnIndex(key)],
        right_on: vec![ColumnIndex(key)],
        left_order: ColumnIndex(2),
        right_order: ColumnIndex(2),
        output: left_two(),
    };
    output_coverage(
        &matched,
        &[CellKind::Node, CellKind::File],
        &[InputCoverage {
            ordinal: OperandOrdinal(0),
            kinds: &[
                CellKind::Node,
                CellKind::File,
                CellKind::Number,
                CellKind::Symbol,
            ],
            coverage: partial(&[Some(FileId(1))]),
            lineage: Default::default(),
        }],
    )
}

fn subject(ask: Ask) -> Result<Value, Failure> {
    Ok(covered(match ask {
        Ask::Ordered(key) => ordered(key),
        Ask::Join(left_on, right_on, missing) => joined(left_on, right_on, missing)?,
        Ask::Matched(key) => matched(key)?,
    }))
}

/// Partial in these files, no lacks said.
const fn partial_in(files: &'static [Option<FileId>]) -> Covered {
    Covered::Partial {
        files: Cow::Borrowed(files),
        lacks: 0,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "ordered_reduction_requires_a_file_partition_before_preserving_file_scoped_uncertainty",
        ),
        word: Word::Succeeds,
        input: || Ask::Ordered(0),
        expect: Expect::Value(partial_in(&[Some(FileId(1))])),
    },
    Case {
        name: CaseName("ordered_reduction_by_a_number_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || Ask::Ordered(1),
        expect: Expect::Value(partial_in(&[None])),
    },
    Case {
        name: CaseName(
            "a_rejected_file_withholds_only_its_own_partition_from_a_join_anchored_by_file",
        ),
        word: Word::Succeeds,
        input: || Ask::Join(1, 0, &[Some(FileId(1))]),
        expect: Expect::Value(partial_in(&[Some(FileId(1))])),
    },
    Case {
        name: CaseName("a_genuinely_partial_valid_file_is_still_reported_partial"),
        word: Word::Succeeds,
        input: || Ask::Join(1, 0, &[Some(FileId(0))]),
        expect: Expect::Value(partial_in(&[Some(FileId(0))])),
    },
    Case {
        name: CaseName("a_join_key_that_is_no_file_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || Ask::Join(2, 1, &[Some(FileId(1))]),
        expect: Expect::Value(partial_in(&[None])),
    },
    Case {
        name: CaseName(
            "a_rejected_file_withholds_only_its_own_partition_from_an_ordered_match_keyed_by_file",
        ),
        word: Word::Succeeds,
        input: || Ask::Matched(1),
        expect: Expect::Value(partial_in(&[Some(FileId(1))])),
    },
    Case {
        name: CaseName("a_symbol_key_of_an_ordered_match_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || Ask::Matched(3),
        expect: Expect::Value(partial_in(&[None])),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
