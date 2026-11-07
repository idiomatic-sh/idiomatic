//! Boundary failures retain their original exact reason and source category.
use super::errors::ReportsError;
use ryo::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

type Input = ReportsError;
type Value = (Cow<'static, str>, Cow<'static, str>);
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an invalid observation keeps its reason and displays it unchanged"),
    word: Word::Succeeds,
    input: || ReportsError::Invalid("coverage is missing".into()),
    expect: Expect::Value((
        Cow::Borrowed("coverage is missing"),
        Cow::Borrowed("coverage is missing"),
    )),
}];

/// The retained reason and the displayed message of an invalid observation.
fn subject(error: Input) -> Result<Value, Failure> {
    let message = error.to_string();
    match error {
        ReportsError::Invalid(reason) => Ok((reason.into(), message.into())),
        other => Err(other.to_string().into()),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
