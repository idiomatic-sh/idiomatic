//! A logical build lends the program the operators plan.

use super::value::LogicalBuild;
use crate::domain::fused_program::service::logical;
use crate::domain::fused_program::types::program_inputs::value::ProgramInputs;
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase as _;
use ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// No input: the request is empty.
type Input = ();

type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_program_of_an_empty_request_plans_with_nothing_unserved"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(0),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let mut inputs = ProgramInputs::default();
    let build: LogicalBuild<'_> = logical(&[], &mut inputs).map_err(|_| ())?;
    let plan = PlanUseCaseImpl.plan(build.program()).map_err(|_| ())?;
    Ok(plan.unserved().len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
