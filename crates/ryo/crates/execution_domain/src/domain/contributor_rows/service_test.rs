//! The `contributor_rows` service's cases: what an answer built from the absence of rows keeps, from what its operands lack.
use std::borrow::Cow;

use super::service::filter_contributor_rows;
use crate::domain::completion::types::CoverageState;
use crate::domain::coverage::service::drop_uncertain;
use crate::domain::operator::contract::service::semantics;
use crate::domain::operator::contract::types::AbsenceMask;
use crate::domain::output_coverage::service::output_coverage;
use crate::domain::output_coverage::types::{InputCoverage, OperandOrdinal};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value as Cell};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId},
    plans::{service::kind_of, types::Operation},
    rows::types::{RowCount, Table},
};

/// An operation of a step over steps 0 (and 1).
#[derive(Clone, Copy, Debug)]
enum Op {
    /// An anti-join on the first column of each.
    Antijoin,
    /// The rows of the left not in the right.
    Difference,
    /// A count grouped by these columns.
    GroupCount(&'static [u32]),
    /// A digest of the second column grouped by the first.
    GroupDigest,
}

/// What a row asks: the rows an answer keeps.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// Of four candidate rows (files 1, 0, 0, 2 with numbers 10, 20, 20, 30; a digest's rows end in 1), those the operation keeps
    /// under the absence its semantics trust, both its file-and-number operands partial in these files.
    Contributed(Op, &'static [Option<FileId>]),
    /// Of a global count of 4, what is kept when its file operand is partial in file 1.
    GlobalCount,
    /// Of a count keyed by its operand's second column (files 0, 1 and 2, each counted 1), what is kept when these files could
    /// miss rows, the schema probed being the operand's (symbol, file) or the answer's.
    Uncertain(&'static [CellKind], &'static [Option<FileId>]),
    /// Of three answers (names or files 0, 1 and 2, each counted 1), what the operation keeps under this absence mask, its
    /// operand of this ordinal (symbol, file) inapplicable in file 1, and the coverage of a group's answer.
    Inapplicable(Op, u8, usize),
}

/// What an answer keeps, as plain values.
#[derive(Debug, PartialEq)]
enum Covered {
    Complete,
    Inapplicable(Cow<'static, [FileId]>),
    Partial,
}

/// The rows an answer keeps, their cells, and (for a group of an inapplicable file) its coverage.
#[derive(Debug, PartialEq)]
struct Kept {
    rows: usize,
    cells: Cow<'static, [Cell]>,
    coverage: Option<Covered>,
}

type Input = Ask;
type Value = Kept;
type Failure = crate::domain::operating::errors::OperatingError;

fn operation(op: Op) -> Operation {
    let (left, right) = (PlanInput::Step(StepId(0)), PlanInput::Step(StepId(1)));
    match op {
        Op::Antijoin => Operation::Antijoin {
            left,
            right,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        },
        Op::Difference => Operation::Difference { left, right },
        Op::GroupCount(keys) => Operation::GroupCount {
            from: left,
            keys: keys.iter().copied().map(ColumnIndex).collect(),
            distinct: None,
        },
        Op::GroupDigest => Operation::GroupDigest {
            from: left,
            keys: vec![ColumnIndex(0)],
            over: ColumnIndex(1),
        },
    }
}

/// A table of these kinds whose rows are the cells, one row for each run of as many cells as kinds.
fn table(kinds: &[CellKind], cells: Vec<Cell>) -> Table {
    let count = RowCount(cells.len() / kinds.len());
    Table {
        kinds: kinds.to_vec(),
        cells,
        count,
    }
}

fn observed(rows: &Table, coverage: Option<Covered>) -> Kept {
    Kept {
        rows: rows.count.0,
        cells: Cow::Owned(rows.cells.clone()),
        coverage,
    }
}

fn contributed(op: Op, affected: &[Option<FileId>]) -> Value {
    let digest = matches!(op, Op::GroupDigest);
    let mut kinds = vec![CellKind::File, CellKind::Number];
    let mut cells = Vec::new();
    for (file, number) in [(1, 10), (0, 20), (0, 20), (2, 30)] {
        cells.extend([Cell::File(FileId(file)), Cell::Number(Integer(number))]);
        if digest {
            cells.push(Cell::Number(Integer(1)));
        }
    }
    if digest {
        kinds.push(CellKind::Number);
    }
    let mut rows = table(&kinds, cells);
    let operation = operation(op);
    let operand = [CellKind::File, CellKind::Number];
    let inputs: Vec<InputCoverage<'_>> = (0..2)
        .map(|ordinal| InputCoverage {
            ordinal: OperandOrdinal(ordinal),
            kinds: &operand,
            coverage: CoverageState::Partial {
                files: affected.to_vec(),
                lacks: vec![],
            },
            lineage: Default::default(),
        })
        .collect();
    filter_contributor_rows(
        &operation,
        semantics(kind_of(&operation)).absence,
        &inputs,
        &mut rows,
    );
    observed(&rows, None)
}

fn global_count() -> Value {
    let mut count = table(&[CellKind::Number], vec![Cell::Number(Integer(4))]);
    filter_contributor_rows(
        &operation(Op::GroupCount(&[])),
        AbsenceMask(1),
        &[InputCoverage {
            ordinal: OperandOrdinal(0),
            kinds: &[CellKind::File],
            coverage: CoverageState::Partial {
                files: vec![Some(FileId(1))],
                lacks: vec![],
            },
            lineage: Default::default(),
        }],
        &mut count,
    );
    observed(&count, None)
}

/// A count keyed by a later column of its operand answers where its key sits in the answer, whichever schema the caller probes with.
fn uncertain(probed: &[CellKind], affected: &[Option<FileId>]) -> Value {
    let cells = (0..3)
        .flat_map(|file| [Cell::File(FileId(file)), Cell::Number(Integer(1))])
        .collect();
    let mut answer = table(&[CellKind::File, CellKind::Number], cells);
    drop_uncertain(
        &operation(Op::GroupCount(&[1])),
        probed,
        &mut answer,
        affected,
    );
    observed(&answer, None)
}

/// A file no language reads, or whose syntax a rule skipped, is out of scope for a group not keyed by file and costs a group keyed
/// by file only its own rows; it is never reported complete for the rows keyed to it.
fn inapplicable(op: Op, mask: u8, ordinal: usize) -> Result<Value, Failure> {
    let operation = operation(op);
    let by_file = matches!(op, Op::GroupCount(&[1]));
    let cells = (0..3)
        .flat_map(|at| {
            let key = if by_file {
                Cell::File(FileId(at))
            } else {
                Cell::Symbol(Symbol(at))
            };
            [key, Cell::Number(Integer(1))]
        })
        .collect();
    let key = if by_file {
        CellKind::File
    } else {
        CellKind::Symbol
    };
    let mut rows = table(&[key, CellKind::Number], cells);
    let operand = [CellKind::Symbol, CellKind::File];
    let input = || InputCoverage {
        ordinal: OperandOrdinal(ordinal),
        kinds: &operand,
        coverage: CoverageState::Inapplicable {
            files: vec![FileId(1)],
        },
        lineage: Default::default(),
    };
    filter_contributor_rows(&operation, AbsenceMask(mask), &[input()], &mut rows);
    let coverage = match op {
        Op::GroupCount(_) => Some(
            match output_coverage(&operation, &rows.kinds, &[input()])? {
                CoverageState::Complete => Covered::Complete,
                CoverageState::Inapplicable { files } => Covered::Inapplicable(files.into()),
                CoverageState::Partial { .. } => Covered::Partial,
            },
        ),
        Op::Antijoin | Op::Difference | Op::GroupDigest => None,
    };
    Ok(observed(&rows, coverage))
}

fn subject(ask: Input) -> Result<Value, Failure> {
    match ask {
        Ask::Contributed(op, affected) => Ok(contributed(op, affected)),
        Ask::GlobalCount => Ok(global_count()),
        Ask::Uncertain(probed, affected) => Ok(uncertain(probed, affected)),
        Ask::Inapplicable(op, mask, ordinal) => inapplicable(op, mask, ordinal),
    }
}

/// Files 2 and 1 lack rows.
const TWO_FILES: &[Option<FileId>] = &[Some(FileId(2)), Some(FileId(1))];
/// The repository lacks rows.
const ANYWHERE: &[Option<FileId>] = &[None];
/// The operand's kinds of the keyed count, and its answer's.
const OPERAND: &[CellKind] = &[CellKind::Number, CellKind::File];
/// The answer's kinds of the keyed count.
const ANSWER: &[CellKind] = &[CellKind::File, CellKind::Number];

/// This many rows kept, of these cells, with no coverage asked.
const fn kept(rows: usize, cells: &'static [Cell]) -> Kept {
    Kept {
        rows,
        cells: Cow::Borrowed(cells),
        coverage: None,
    }
}
/// The two rows of file 0 with the number 20.
const FILE_0: &[Cell] = &[
    Cell::File(FileId(0)),
    Cell::Number(Integer(20)),
    Cell::File(FileId(0)),
    Cell::Number(Integer(20)),
];
/// The answers of files 0 and 2.
const FILES_0_AND_2: &[Cell] = &[
    Cell::File(FileId(0)),
    Cell::Number(Integer(1)),
    Cell::File(FileId(2)),
    Cell::Number(Integer(1)),
];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "contributor_partiality_filters_absence_answers_before_keys_can_be_projected_away",
        ),
        word: Word::Succeeds,
        input: || Ask::Contributed(Op::Antijoin, TWO_FILES),
        expect: Expect::Value(kept(2, FILE_0)),
    },
    Case {
        name: CaseName("an_antijoin_partial_anywhere_keeps_no_row"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Op::Antijoin, ANYWHERE),
        expect: Expect::Value(kept(0, &[])),
    },
    Case {
        name: CaseName("a_difference_keeps_only_the_rows_of_files_that_miss_none"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Op::Difference, TWO_FILES),
        expect: Expect::Value(kept(2, FILE_0)),
    },
    Case {
        name: CaseName("a_difference_partial_anywhere_keeps_no_row"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Op::Difference, ANYWHERE),
        expect: Expect::Value(kept(0, &[])),
    },
    Case {
        name: CaseName("a_count_keyed_by_file_keeps_only_the_rows_of_files_that_miss_none"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Op::GroupCount(&[0]), TWO_FILES),
        expect: Expect::Value(kept(2, FILE_0)),
    },
    Case {
        name: CaseName("a_count_partial_anywhere_keeps_no_row"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Op::GroupCount(&[0]), ANYWHERE),
        expect: Expect::Value(kept(0, &[])),
    },
    Case {
        name: CaseName("a_digest_keyed_by_file_keeps_only_the_rows_of_files_that_miss_none"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Op::GroupDigest, TWO_FILES),
        expect: Expect::Value(kept(
            2,
            &[
                Cell::File(FileId(0)),
                Cell::Number(Integer(20)),
                Cell::Number(Integer(1)),
                Cell::File(FileId(0)),
                Cell::Number(Integer(20)),
                Cell::Number(Integer(1)),
            ],
        )),
    },
    Case {
        name: CaseName("a_digest_partial_anywhere_keeps_no_row"),
        word: Word::Succeeds,
        input: || Ask::Contributed(Op::GroupDigest, ANYWHERE),
        expect: Expect::Value(kept(0, &[])),
    },
    Case {
        name: CaseName("an_unkeyed_aggregate_cannot_localize_uncertainty"),
        word: Word::Succeeds,
        input: || Ask::GlobalCount,
        expect: Expect::Value(kept(0, &[])),
    },
    Case {
        name: CaseName("a_grouped_answer_withholds_only_the_partitions_of_the_affected_files"),
        word: Word::Succeeds,
        input: || Ask::Uncertain(OPERAND, &[Some(FileId(1))]),
        expect: Expect::Value(kept(2, FILES_0_AND_2)),
    },
    Case {
        name: CaseName("a_grouped_answer_probed_by_its_own_schema_withholds_the_same_partition"),
        word: Word::Succeeds,
        input: || Ask::Uncertain(ANSWER, &[Some(FileId(1))]),
        expect: Expect::Value(kept(2, FILES_0_AND_2)),
    },
    Case {
        name: CaseName("an_unlocated_cause_withholds_every_partition"),
        word: Word::Succeeds,
        input: || Ask::Uncertain(OPERAND, ANYWHERE),
        expect: Expect::Value(kept(0, &[])),
    },
    Case {
        name: CaseName("an_unlocated_cause_withholds_every_partition_whichever_schema_is_probed"),
        word: Word::Succeeds,
        input: || Ask::Uncertain(ANSWER, ANYWHERE),
        expect: Expect::Value(kept(0, &[])),
    },
    Case {
        name: CaseName(
            "an_inapplicable_file_withholds_nothing_from_a_symbol_keyed_group_and_only_its_own_rows_from_a_file_keyed_one",
        ),
        word: Word::Succeeds,
        input: || Ask::Inapplicable(Op::GroupCount(&[0]), 1, 0),
        expect: Expect::Value(Kept {
            rows: 3,
            cells: Cow::Borrowed(&[
                Cell::Symbol(Symbol(0)),
                Cell::Number(Integer(1)),
                Cell::Symbol(Symbol(1)),
                Cell::Number(Integer(1)),
                Cell::Symbol(Symbol(2)),
                Cell::Number(Integer(1)),
            ]),
            coverage: Some(Covered::Complete),
        }),
    },
    Case {
        name: CaseName("a_file_keyed_group_withholds_the_inapplicable_files_own_rows"),
        word: Word::Succeeds,
        input: || Ask::Inapplicable(Op::GroupCount(&[1]), 1, 0),
        expect: Expect::Value(Kept {
            rows: 2,
            cells: Cow::Borrowed(FILES_0_AND_2),
            coverage: Some(Covered::Inapplicable(Cow::Borrowed(&[FileId(1)]))),
        }),
    },
    Case {
        name: CaseName("a_skipped_file_is_still_unavailable_for_an_absence_check"),
        word: Word::Succeeds,
        input: || Ask::Inapplicable(Op::Antijoin, 2, 1),
        expect: Expect::Value(kept(0, &[])),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
