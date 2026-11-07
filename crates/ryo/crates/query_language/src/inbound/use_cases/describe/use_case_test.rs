//! What the `describe` use case promises, against the fake of the port it calls.

use std::sync::Arc;

use super::errors::DescribeError;
use super::interface::DescribeUseCase as _;
use super::use_case::DescribeUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::fixtures::fake::SyntaxFake;

type Input = ();
/// How many productions are answered, and whether the first is the fake's one.
type Value = (usize, bool);
type Failure = DescribeError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("answers_the_grammar_the_syntax_port_reads"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((1, true)),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let report = DescribeUseCaseImpl::new(Arc::new(SyntaxFake::default())).describe()?;
    let first = report
        .grammar
        .0
        .first()
        .is_some_and(|production| &*production.name.0 == "program");
    Ok((report.grammar.0.len(), first))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
