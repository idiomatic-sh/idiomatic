//! A planned program answers the plan it was assembled with.

use crate::domain::fused_program::service::{assemble, logical, physical};
use super::value::FusedProgram;
use crate::domain::fused_program::types::program_inputs::value::ProgramInputs;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// No input: the request is empty.
type Input = ();

type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_empty_program_answers_an_empty_plan"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(0),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let mut inputs = ProgramInputs::default();
    let build = logical(&[], &mut inputs).map_err(|_| ())?;
    let program: FusedProgram = assemble(build, PhysicalPlan::default());
    Ok(physical(&program).unserved().len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
