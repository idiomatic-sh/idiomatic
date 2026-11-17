//! The `file_dag` service's cases, on values alone: a request's program prepared once over the operators.

use super::service::prepare;
use crate::application::file_dag::program::prepared_execution::interface::ProgramPreparedExecutionPort as _;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::observation_schedule::types::RuleSchedule;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::{Compiled, ObservationIdentity};
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;

/// How many rule digests the request is prepared with, beside its rules.
type Input = usize;
/// How many rules the prepared program runs.
type Value = usize;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_request_of_no_rule_prepares_a_program_of_none"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("digests_that_do_not_pair_with_the_rules_are_refused"),
        word: Word::Refuses,
        input: || 1,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];

fn subject(digests: Input) -> Result<Value, Failure> {
    let compiled = Compiled {
        schedule: RuleSchedule::Independent,
        observations: Vec::new(),
        plans: Vec::new(),
        bindings: Vec::new(),
    };
    let digests = vec![Digest([0; 16]); digests];
    let program = prepare(
        compiled,
        &[],
        &digests,
        &[],
        ObservationIdentity::Independent,
        Arc::new(crate::application::file_dag::fixtures::operators()),
    )?;
    Ok(program.rule_count().0)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
