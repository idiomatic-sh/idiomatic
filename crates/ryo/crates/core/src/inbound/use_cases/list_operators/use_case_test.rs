//! What the `list_operators` use case promises: the operators, each once, in documentation order.

use super::errors::ListOperatorsError;
use super::interface::ListOperatorsUseCase as _;
use super::use_case::ListOperatorsUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
/// The names, as a count and whether they come in the documented order.
type Value = (usize, bool);
type Failure = ListOperatorsError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("lists_the_nineteen_operators_in_documentation_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((19, true)),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let report = ListOperatorsUseCaseImpl.list_operators()?;
    let names: Vec<&str> = report
        .operators
        .iter()
        .map(|operator| &*operator.name.0)
        .collect();
    let documented = [
        "replace",
        "ordered-match",
        "select",
        "project",
        "extend",
        "join",
        "union",
        "intersection",
        "difference",
        "antijoin",
        "group-count",
        "group-digest",
        "capture",
        "words",
        "rank",
        "axis",
        "rows",
        "closure",
        "cycles",
    ];
    Ok((names.len(), names == documented))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
