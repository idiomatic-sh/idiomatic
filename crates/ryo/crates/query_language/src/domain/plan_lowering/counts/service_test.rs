//! The `counts` pass's cases, on the passes' small plan (`fixtures.rs`) after the semijoins pass: the marks are those
//! `pass_count.py` wrote on the same plan.

use super::service::counted;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::finish_plans::types::FinishPlan;
use crate::domain::plan_lowering::fixtures::{select_marks, small_plan};
use crate::domain::plan_lowering::semijoins::service::semijoined;

type Input = FinishPlan;
/// How many selects the pass marked, and each select's marks: (step, probed source, counted).
type Value = (usize, [(usize, Option<usize>, bool); 8]);
type Failure = ();

fn semijoined_plan() -> FinishPlan {
    let mut plan = small_plan();
    let uses = crate::domain::plan_lowering::readers::service::step_uses(&plan);
    let _marked = semijoined(&mut plan, &uses);
    plan
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_select_whose_one_reader_counts_every_row_is_answered_by_counts"),
    word: Word::Succeeds,
    input: semijoined_plan,
    expect: Expect::Value((
        1,
        [
            (2, Some(1), false),
            (4, Some(0), false),
            (5, None, false),
            (6, None, false),
            (7, Some(1), true),
            (9, Some(1), false),
            (12, None, false),
            (14, None, false),
        ],
    )),
}];

fn subject(mut plan: Input) -> Result<Value, Failure> {
    let uses = crate::domain::plan_lowering::readers::service::step_uses(&plan);
    let marked = counted(&mut plan, &uses);
    Ok((marked, select_marks(&plan).try_into().map_err(|_| ())?))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
