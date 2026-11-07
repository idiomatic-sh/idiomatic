//! Digest membership, framing and malformed inputs through actual grouped execution.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
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
#[derive(Clone, Copy)]
enum Scenario {
    Groups,
    EqualSets,
    DifferentSets,
    DifferentSizes,
    Missing,
    MissingColumn,
}
type Input = Scenario;
type Value = usize;
type Failure = OperatingError;
fn subject(input: Input) -> Result<Value, Failure> {
    let rows: &[&[i64]] = match input {
        Scenario::EqualSets => &[&[1, 10], &[1, 20], &[2, 20], &[2, 10], &[2, 10], &[2, 20]],
        Scenario::DifferentSets => &[&[1, 10], &[1, 20], &[2, 10], &[2, 30]],
        Scenario::DifferentSizes => &[&[1, 10], &[2, 10], &[2, 20]],
        Scenario::MissingColumn => &[&[1, 10]],
        _ => &[&[1, 10], &[1, 20], &[2, 30]],
    };
    let mut operations = vec![];
    let hosted = if matches!(input, Scenario::Missing) {
        vec![]
    } else {
        operations.push(Operation::Hosted {
            relation: HostedNumber(0),
        });
        vec![fixtures::numbers("groups", 2, rows)]
    };
    operations.push(Operation::GroupDigest {
        from: if matches!(input, Scenario::Missing) {
            PlanInput::Relation(Relation::Function)
        } else {
            PlanInput::Step(StepId(0))
        },
        keys: vec![ColumnIndex(0)],
        over: ColumnIndex(if matches!(input, Scenario::MissingColumn) {
            9
        } else {
            1
        }),
    });
    let output = fixtures::published(
        fixtures::plan(operations),
        vec![],
        hosted,
        &mut Symbols::default(),
    )?;
    assert_eq!(output.count.0, 2);
    assert_eq!(output.kinds.len(), 3);
    assert_eq!(output.cells[0], Cell::Number(Integer(1)));
    assert_eq!(output.cells[3], Cell::Number(Integer(2)));
    match input {
        Scenario::Groups => Ok(output.count.0 * 10 + output.kinds.len()),
        Scenario::EqualSets => Ok(usize::from(output.cells[1..3] == output.cells[4..6])),
        Scenario::DifferentSets => Ok(usize::from(
            output.cells[1] != output.cells[4] && output.cells[2] == output.cells[5],
        )),
        Scenario::DifferentSizes => Ok(usize::from(
            output.cells[1] != output.cells[4]
                && output.cells[2] == Cell::Number(Integer(1))
                && output.cells[5] == Cell::Number(Integer(2)),
        )),
        _ => unreachable!("missing input or column must fail before publishing groups"),
    }
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_row_per_group_with_a_digest_and_a_size"),
        word: Word::Succeeds,
        input: || Scenario::Groups,
        expect: Expect::Value(23),
    },
    Case {
        name: CaseName("equal_sets_give_equal_digests_whatever_the_order_or_the_repeats"),
        word: Word::Succeeds,
        input: || Scenario::EqualSets,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("different_sets_give_different_digests"),
        word: Word::Succeeds,
        input: || Scenario::DifferentSets,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_set_of_one_value_is_not_a_set_of_two"),
        word: Word::Succeeds,
        input: || Scenario::DifferentSizes,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("digest_requires_its_real_provider"),
        word: Word::Refuses,
        input: || Scenario::Missing,
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("a_column_the_input_lacks_is_refused"),
        word: Word::Refuses,
        input: || Scenario::MissingColumn,
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
