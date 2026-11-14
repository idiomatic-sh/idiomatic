//! The input lists a logical build leaves with its caller: one per node, the build's first pass clearing what a previous
//! build left.

use super::value::ProgramInputs;
use crate::domain::fused_program::service::logical;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// How many times an empty request is built over the same inputs.
type Input = usize;

type Value = usize;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_empty_request_holds_no_node_inputs_after_any_number_of_builds"),
    word: Word::Succeeds,
    input: || 2,
    expect: Expect::Value(0),
}];

fn subject(builds: Input) -> Result<Value, Failure> {
    let mut inputs = ProgramInputs::default();
    for _ in 0..builds {
        let _build = logical(&[], &mut inputs).map_err(|_| ())?;
    }
    Ok(inputs.lists.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
