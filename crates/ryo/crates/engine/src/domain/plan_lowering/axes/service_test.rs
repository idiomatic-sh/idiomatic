//! The `axes` pass's cases, on the passes' small plan (`fixtures.rs`) after the semijoins and counts passes: the marks
//! are those `pass_axis.py` wrote on the same plan.

use super::service::existence_marked;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::finish_plans::types::FinishPlan;
use crate::domain::plan_lowering::counts::service::counted;
use crate::domain::plan_lowering::fixtures::{existence_marks, small_plan};
use crate::domain::plan_lowering::semijoins::service::semijoined;

type Input = FinishPlan;
/// How many axes the pass marked, and which.
type Value = (usize, [usize; 1]);
type Failure = ();

fn counted_plan() -> FinishPlan {
    let mut plan = small_plan();
    let _semijoins = semijoined(&mut plan);
    let _counts = counted(&mut plan);
    plan
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_ancestor_axis_whose_related_column_no_reader_keeps_asks_only_existence"),
    word: Word::Succeeds,
    input: counted_plan,
    expect: Expect::Value((1, [11])),
}];

fn subject(mut plan: Input) -> Result<Value, Failure> {
    let marked = existence_marked(&mut plan);
    Ok((marked, existence_marks(&plan).try_into().map_err(|_| ())?))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
