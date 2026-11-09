//! The `text_gate` service's cases, on comparisons alone: which columns the comparisons left to decide read.

use super::service::needed_of_rest;
use crate::domain::table_cells::types::ColumnPlaces;
use crate::domain::table_scratch::types::ComparisonNumbers;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral, Test,
};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Names the columns of the comparisons numbered, each once.
    NeedsTheColumnsOfTheRest,
}

/// What a row of the cases does.
type Input = Scenario;
/// The columns that come out, written as bits: place `n` is the bit `1 << n`.
type Value = usize;
type Failure = ();

fn named(column: u32, test: Test, literal: &str) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(literal.into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

/// Four comparisons: a number of column 0, the kind named `word`, the text `a`, and the text `b` written the other way round.
fn comparisons() -> Vec<PlanComparison> {
    vec![
        PlanComparison {
            test: Test::GreaterThan,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Literal(QueryLiteral::Number(Integer(1))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        },
        named(1, Test::Equal, "word"),
        named(2, Test::Equal, "a"),
        PlanComparison {
            test: Test::Equal,
            left: Operand::Literal(QueryLiteral::Text(LiteralText("b".into()))),
            right: Operand::Column(ColumnIndex(2)),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        },
    ]
}

fn bits(places: &[usize]) -> Value {
    places.iter().fold(0, |all, place| all | (1 << place))
}

fn columns(which: &[usize]) -> Value {
    let mut comparisons = comparisons();
    comparisons.push(PlanComparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(2)),
        right: Operand::Parameter(ParameterId(0)),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    });
    let mut needed = ColumnPlaces(vec![9]);
    needed_of_rest(
        &comparisons,
        &ComparisonNumbers(which.to_vec()),
        &mut needed,
    );
    bits(&needed.0)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("only_the_columns_of_the_rest_are_needed"),
    word: Word::Succeeds,
    input: || Scenario::NeedsTheColumnsOfTheRest,
    expect: Expect::Value(0b101),
}];

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::NeedsTheColumnsOfTheRest => columns(&[0, 4]),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
