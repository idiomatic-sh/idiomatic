//! Actual invocation storage measurements through normal compiled file execution.
use super::fixtures::{execute, hosted};
use crate::application::file_dag::fixtures::operator_contract::{
    expectations::assert_expected, fixture,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::{
    hosted::types::HostKind,
    plan_parts::types::{Operand, PlanComparison, Test},
    row_texts::types::Cell as HostedCell,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId},
    plans::types::Operation,
};
use std::borrow::Cow;
/// The operation a row runs over hosted rows.
#[derive(Clone, Copy, Debug)]
enum Operated {
    /// Projects the one number column of these rows, keeping or dropping repeats.
    Project {
        distinct: RepeatedRows,
        rows: &'static [i64],
    },
    /// Selects the text rows whose text matches itself as a pattern.
    SelfMatch { rows: &'static [&'static str] },
    /// Runs the operator contract's fixture of this aggregation.
    Aggregate(OperationKind),
}

/// What the operation delivered.
#[derive(Debug, PartialEq)]
enum Delivered {
    /// The number cells.
    Numbers(Cow<'static, [i64]>),
    /// The row count.
    Rows(usize),
    /// The pattern refusal's reason.
    InvalidRegex(Cow<'static, str>),
    /// The rows, and whether they are the operator contract's expected rows.
    Contract { rows: usize, as_expected: bool },
}

/// What the operation delivered, the scratch owners it initialized and the patterns it cached.
#[derive(Debug, PartialEq)]
struct Ran {
    delivered: Delivered,
    owners_initialized: usize,
    regexes: usize,
}

type Input = Operated;
type Value = Ran;
type Failure = OperatingError;

const fn ran(delivered: Delivered, owners_initialized: usize) -> Ran {
    Ran {
        delivered,
        owners_initialized,
        regexes: 0,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("projection_constructs_only_the_group_it_uses_and_keeps_bags"),
        word: Word::Succeeds,
        input: || Operated::Project {
            distinct: RepeatedRows::Kept,
            rows: &[2, 1, 2],
        },
        expect: Expect::Value(ran(Delivered::Numbers(Cow::Borrowed(&[2, 1, 2])), 0)),
    },
    Case {
        name: CaseName("a_distinct_projection_constructs_its_one_group"),
        word: Word::Succeeds,
        input: || Operated::Project {
            distinct: RepeatedRows::Dropped,
            rows: &[2, 1, 2],
        },
        expect: Expect::Value(ran(Delivered::Numbers(Cow::Borrowed(&[2, 1])), 1)),
    },
    Case {
        name: CaseName("dynamic_regex_initialization_is_execution_local_and_lazy"),
        word: Word::Succeeds,
        input: || Operated::SelfMatch { rows: &[] },
        expect: Expect::Value(ran(Delivered::Rows(0), 0)),
    },
    Case {
        name: CaseName("an_invalid_dynamic_pattern_is_refused_and_never_cached"),
        word: Word::Succeeds,
        input: || Operated::SelfMatch { rows: &["["] },
        expect: Expect::Value(ran(
            Delivered::InvalidRegex(Cow::Borrowed(
                "regex parse error:\n    [\n    ^\nerror: unclosed character class",
            )),
            0,
        )),
    },
    Case {
        name: CaseName("aggregation_resources_are_engine_owned_and_released"),
        word: Word::Succeeds,
        input: || Operated::Aggregate(OperationKind::GroupCount),
        expect: Expect::Value(ran(
            Delivered::Contract {
                rows: 2,
                as_expected: true,
            },
            1,
        )),
    },
    Case {
        name: CaseName("a_group_digest_owns_and_releases_its_resources"),
        word: Word::Succeeds,
        input: || Operated::Aggregate(OperationKind::GroupDigest),
        expect: Expect::Value(ran(
            Delivered::Contract {
                rows: 2,
                as_expected: true,
            },
            1,
        )),
    },
];

fn numbers(cells: &[Cell]) -> Cow<'static, [i64]> {
    cells
        .iter()
        .map(|cell| match cell {
            Cell::Number(Integer(number)) => *number,
            other => panic!("a number cell, not {other:?}"),
        })
        .collect()
}

/// The aggregation's operation over step 0, and its hosted rows, from the operator contract fixture.
fn aggregate(kind: OperationKind) -> Result<Ran, OperatingError> {
    let mut case = fixture(kind);
    let input = &case.inputs[0];
    let rows = input
        .cells
        .chunks_exact(input.kinds.len())
        .map(|row| {
            row.iter()
                .map(|value| match value {
                    Cell::Number(number) => HostedCell::Number(*number),
                    other => panic!("numeric aggregation fixture, got {other:?}"),
                })
                .collect()
        })
        .collect();
    let operation = match case.operation {
        Operation::GroupCount { keys, distinct, .. } => Operation::GroupCount {
            from: PlanInput::Step(StepId(0)),
            keys,
            distinct,
        },
        Operation::GroupDigest { keys, over, .. } => Operation::GroupDigest {
            from: PlanInput::Step(StepId(0)),
            keys,
            over,
        },
        _ => return Err(OperatingError::WrongOperation),
    };
    let (result, observed) = execute(
        operation,
        hosted(&vec![HostKind::Number; input.kinds.len()], rows),
        &mut case.symbols,
    );
    let out = result?;
    let rows = out
        .cells
        .chunks_exact(out.kinds.len())
        .map(<[_]>::to_vec)
        .collect::<Vec<_>>();
    // The contract's comparison asserts; whether it held is this row's observation.
    let as_expected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_expected(&case.expected, &rows, &case.symbols);
    }))
    .err()
    .is_none();
    Ok(Ran {
        delivered: Delivered::Contract {
            rows: rows.len(),
            as_expected,
        },
        owners_initialized: observed.owners_initialized,
        regexes: observed.regexes,
    })
}

/// Runs the operation and observes what it delivered and initialized.
fn subject(operated: Input) -> Result<Value, Failure> {
    let (result, observed) = match operated {
        Operated::Aggregate(kind) => return aggregate(kind),
        Operated::Project { distinct, rows } => execute(
            Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct,
            },
            hosted(
                &[HostKind::Number],
                rows.iter()
                    .map(|number| vec![HostedCell::Number(Integer(*number))])
                    .collect(),
            ),
            &mut Symbols::default(),
        ),
        Operated::SelfMatch { rows } => execute(
            Operation::Select {
                from: PlanInput::Step(StepId(0)),
                comparisons: vec![PlanComparison {
                    test: Test::Matches,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Column(ColumnIndex(0)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![ColumnIndex(0)],
            },
            hosted(
                &[HostKind::Text],
                rows.iter()
                    .map(|text| vec![HostedCell::Text((*text).into())])
                    .collect(),
            ),
            &mut Symbols::default(),
        ),
    };
    let delivered = result
        .map(|table| match operated {
            Operated::Project { .. } => Delivered::Numbers(numbers(&table.cells)),
            _ => Delivered::Rows(table.count.0),
        })
        .or_else(|error| match error {
            OperatingError::InvalidRegex { reason } => Ok(Delivered::InvalidRegex(reason.into())),
            other => Err(other),
        })?;
    Ok(Ran {
        delivered,
        owners_initialized: observed.owners_initialized,
        regexes: observed.regexes,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
