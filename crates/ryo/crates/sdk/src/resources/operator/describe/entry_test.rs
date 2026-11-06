//! What `operator describe` promises its caller, one row of `CASES` for each case it has.

use super::entry::describe;
use super::errors::DescribeError;
use super::types::DescribeRequest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operator::types::OperatorName;

type Input = &'static str;
/// How many fields the operator has.
type Value = usize;
type Failure = DescribeError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("describes_the_requested_operator"),
        word: Word::Succeeds,
        input: || "join",
        expect: Expect::Value(5),
    },
    Case {
        name: CaseName("unknown_operator_is_refused"),
        word: Word::Refuses,
        input: || "sort",
        expect: Expect::Refusal(|error| selects!(error, DescribeError::UnknownOperator { .. })),
    },
];

fn subject(name: Input) -> Result<Value, Failure> {
    describe(DescribeRequest {
        name: OperatorName(name.into()),
    })
    .map(|report| report.operator.fields.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
