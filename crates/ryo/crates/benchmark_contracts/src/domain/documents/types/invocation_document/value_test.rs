//! Environment variables of an invocation iterate in name order.
use super::value::{EnvironmentName, EnvironmentValue};
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::collections::BTreeMap;

type Input = &'static [&'static str];
type Value = std::borrow::Cow<'static, str>;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("names iterate sorted"),
    word: Word::Succeeds,
    input: || &["B", "A", "C"],
    expect: Expect::Value(std::borrow::Cow::Borrowed("A,B,C")),
}];
fn subject(names: Input) -> Result<Value, Failure> {
    let environment: BTreeMap<EnvironmentName, EnvironmentValue> = names
        .iter()
        .map(|name| {
            (
                EnvironmentName((*name).into()),
                EnvironmentValue(String::new()),
            )
        })
        .collect();
    Ok(environment
        .keys()
        .map(|name| name.0.as_str())
        .collect::<Vec<_>>()
        .join(",")
        .into())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
