//! Digest membership, framing and malformed inputs through actual grouped execution.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Integer, Relation, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, HostedNumber, Input as PlanInput, StepId},
        plans::types::Operation,
        rows::errors::RowsError,
    },
};
use std::borrow::Cow;
/// The `[key, value]` rows grouped by key, the column digested, and whether the rows reach the
/// digest through its real hosted provider or it reads the function relation instead.
#[derive(Clone, Copy, Debug)]
struct Grouped {
    rows: &'static [&'static [i64]],
    over: u32,
    hosted: bool,
}

/// What the grouped digest publishes: the number of columns, each group's key and size in order,
/// and whether every group's digest is the first group's.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Digests {
    columns: usize,
    groups: Cow<'static, [(Cell, Cell)]>,
    digests_equal: bool,
}

type Input = Grouped;
type Value = Digests;
type Failure = OperatingError;

/// Rows of keys 1 and 2 over the values 10, 20 and 30.
const THREE_ROWS: &[&[i64]] = &[&[1, 10], &[1, 20], &[2, 30]];

/// Group `key` of `size` values.
const fn group(key: i64, size: i64) -> (Cell, Cell) {
    (Cell::Number(Integer(key)), Cell::Number(Integer(size)))
}

/// The rows grouped by key, digested over column 1, from the hosted provider.
const fn hosted(rows: &'static [&'static [i64]]) -> Grouped {
    Grouped {
        rows,
        over: 1,
        hosted: true,
    }
}

/// Publishes the digest of each key's group.
fn subject(grouped: Input) -> Result<Value, Failure> {
    let mut operations = vec![];
    let hosted = if grouped.hosted {
        operations.push(Operation::Hosted {
            relation: HostedNumber(0),
        });
        vec![fixtures::requests::numbers("groups", 2, grouped.rows)]
    } else {
        vec![]
    };
    operations.push(Operation::GroupDigest {
        from: if grouped.hosted {
            PlanInput::Step(StepId(0))
        } else {
            PlanInput::Relation(Relation::Function)
        },
        keys: vec![ColumnIndex(0)],
        over: ColumnIndex(grouped.over),
    });
    let output = fixtures::requests::published(
        inputs::plan(operations),
        vec![],
        hosted,
        &mut Symbols::default(),
    )?;
    let rows: Vec<&[Cell]> = output.cells.chunks_exact(3).collect();
    Ok(Digests {
        columns: output.kinds.len(),
        groups: Cow::Owned(
            rows.iter()
                .map(|row| (row[0].clone(), row[2].clone()))
                .collect(),
        ),
        digests_equal: rows.iter().all(|row| row[1] == rows[0][1]),
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_row_per_group_with_a_digest_and_a_size"),
        word: Word::Succeeds,
        input: || hosted(THREE_ROWS),
        expect: Expect::Value(Digests {
            columns: 3,
            groups: Cow::Borrowed(&[group(1, 2), group(2, 1)]),
            digests_equal: false,
        }),
    },
    Case {
        name: CaseName("equal_sets_give_equal_digests_whatever_the_order_or_the_repeats"),
        word: Word::Succeeds,
        input: || hosted(&[&[1, 10], &[1, 20], &[2, 20], &[2, 10], &[2, 10], &[2, 20]]),
        expect: Expect::Value(Digests {
            columns: 3,
            groups: Cow::Borrowed(&[group(1, 2), group(2, 2)]),
            digests_equal: true,
        }),
    },
    Case {
        name: CaseName("different_sets_give_different_digests"),
        word: Word::Succeeds,
        input: || hosted(&[&[1, 10], &[1, 20], &[2, 10], &[2, 30]]),
        expect: Expect::Value(Digests {
            columns: 3,
            groups: Cow::Borrowed(&[group(1, 2), group(2, 2)]),
            digests_equal: false,
        }),
    },
    Case {
        name: CaseName("a_set_of_one_value_is_not_a_set_of_two"),
        word: Word::Succeeds,
        input: || hosted(&[&[1, 10], &[2, 10], &[2, 20]]),
        expect: Expect::Value(Digests {
            columns: 3,
            groups: Cow::Borrowed(&[group(1, 1), group(2, 2)]),
            digests_equal: false,
        }),
    },
    Case {
        name: CaseName("digest_requires_its_real_provider"),
        word: Word::Refuses,
        input: || Grouped {
            hosted: false,
            ..hosted(THREE_ROWS)
        },
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("a_column_the_input_lacks_is_refused"),
        word: Word::Refuses,
        input: || Grouped {
            over: 9,
            ..hosted(&[&[1, 10]])
        },
        expect: Expect::Error(OperatingError::Table(RowsError::NoSuchColumn { index: 9 })),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
