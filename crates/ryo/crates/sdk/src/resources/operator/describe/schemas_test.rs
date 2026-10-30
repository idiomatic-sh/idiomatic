//! What the schemas of this entry accept and refuse, one row of `CASES` for each input.

use super::super::entry::describe;
use super::super::errors::DescribeError;
use super::super::types::DescribeRequest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operator::types::OperatorName;

type Input = &'static str;
/// How many operators the report describes under the name asked for.
type Value = usize;
type Failure = DescribeError;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_report_describes_the_operator_asked_for"),
    word: Word::Succeeds,
    input: || "join",
    expect: Expect::Value(1),
}];

/// Describes the operator and counts the report as one when it carries the name asked for.
fn subject(name: Input) -> Result<Value, Failure> {
    describe(DescribeRequest {
        name: OperatorName(name.into()),
    })
    .map(|report| usize::from(&*report.operator.name.0 == name))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
