//! What the `describe_operator` use case promises: the one operator asked for, or a refusal.

use super::errors::DescribeOperatorError;
use super::interface::DescribeOperatorUseCase as _;
use super::types::DescribeOperatorRequest;
use super::use_case::DescribeOperatorUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run, Matched, selects};
use ryo_values::domain::descriptions::types::OperatorName;

/// The name asked for.
type Input = &'static str;
/// How many fields the described operator has.
type Value = usize;
type Failure = DescribeOperatorError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("describes_one_operator"),
        word: Word::Succeeds,
        input: || "antijoin",
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("unknown_operator_is_refused"),
        word: Word::Refuses,
        input: || "sort",
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(
                error,
                DescribeOperatorError::UnknownOperator { .. }
            )))
        }),
    },
];

fn subject(name: Input) -> Result<Value, Failure> {
    DescribeOperatorUseCaseImpl::new(crate::inbound::use_cases::lint::fixtures::engine::metadata())
        .describe_operator(DescribeOperatorRequest {
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
