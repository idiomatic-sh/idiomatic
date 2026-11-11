//! The `text_gate` service's cases, on comparisons alone: which columns the comparisons left to decide read.

use std::borrow::Cow;

use super::service::needed_of_rest;
use crate::domain::table_cells::types::ColumnPlaces;
use crate::domain::table_scratch::types::ComparisonNumbers;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral, Test,
};

/// The numbers of the comparisons already decided, among the four of `comparisons` and an equality of column 2 with a
/// parameter.
type Input = &'static [usize];
/// The column places needed to decide the rest, the needed places having held the place 9 before.
type Value = Cow<'static, [usize]>;
type Failure = std::convert::Infallible;

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

fn subject(which: Input) -> Result<Value, Failure> {
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
    Ok(Cow::Owned(needed.0))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("only_the_columns_of_the_rest_are_needed"),
    word: Word::Succeeds,
    input: || &[0, 4],
    expect: Expect::Value(Cow::Borrowed(&[0, 2])),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
