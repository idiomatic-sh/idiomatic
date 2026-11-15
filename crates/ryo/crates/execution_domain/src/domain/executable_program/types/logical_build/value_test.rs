//! A logical build lends the program the operators plan.

use super::value::LogicalBuild;
use crate::domain::executable_program::service::logical;
use crate::domain::executable_program::types::program_inputs::value::ProgramInputs;
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase as _;
use ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// No input: the request is empty.
type Input = ();

/// Whether the operators planned the build's program.
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_program_of_an_empty_request_plans"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let mut inputs = ProgramInputs::default();
    let build: LogicalBuild<'_> = logical(&[], &mut inputs).map_err(|_| ())?;
    Ok(PlanUseCaseImpl.plan(build.program()).is_ok())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
