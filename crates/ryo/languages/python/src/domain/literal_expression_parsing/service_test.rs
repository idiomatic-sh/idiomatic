//! The `literal_expression_parsing` service's cases: the name of each outcome.
use super::service::outcome_name;
use super::types::LiteralParseOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Nothing is given.
type Input = ();
/// The name of every outcome, joined by `|`.
type Value = &'static str;
/// Nothing is refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("each_outcome_has_its_name"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value("parsed|malformed|unavailable"),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let names: Vec<&str> = [LiteralParseOutcome::Parsed, LiteralParseOutcome::Malformed, LiteralParseOutcome::Unavailable]
        .into_iter()
        .map(|each| outcome_name(each).0)
        .collect();
    Ok(Box::leak(names.join("|").into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
