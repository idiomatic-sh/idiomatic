//! Real compiled source/line gates precede the use of the matcher, which preparation compiled.
use super::super::fixtures::span;
use super::fixtures::run as run_file;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::constants::NO_SPAN;
/// The source and line gates refuse a malformed row before its matcher is used: its error is the gate's, not the
/// pattern's. The literal pattern is compiled once, eagerly, when the request is prepared (L-span L1), so every run
/// counts that one build, and a row that passes the gates meets the compiled refusal.
fn malformed_source_and_line_refuse_before_the_prepared_regex() {
    for stream in [false, true] {
        assert_eq!(
            run_file(Some(span(0, 2)), 1, stream),
            (Err(OperatingError::InvalidSourceMatchSpan), 1)
        );
        assert_eq!(
            run_file(Some(span(0, 1)), 0, stream),
            (Err(OperatingError::InvalidSourceMatchSpan), 1)
        );
        assert_eq!(run_file(Some(NO_SPAN), 0, stream), (Ok(()), 1));
        assert_eq!(run_file(None, 1, stream), (Ok(()), 1));
        let (result, builds) = run_file(Some(span(0, 1)), 1, stream);
        assert!(matches!(result, Err(OperatingError::InvalidRegex { .. })));
        assert_eq!(builds, 1);
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    MalformedSourceAndLineRefuseBeforeThePreparedRegex,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("malformed_source_and_line_refuse_before_the_prepared_regex"),
    word: Word::Succeeds,
    input: || Scenario::MalformedSourceAndLineRefuseBeforeThePreparedRegex,
    expect: Expect::Value(()),
}];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::MalformedSourceAndLineRefuseBeforeThePreparedRegex => {
            malformed_source_and_line_refuse_before_the_prepared_regex();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
