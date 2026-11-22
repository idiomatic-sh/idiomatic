//! The `semijoins` pass's cases, on the passes' small plan (`fixtures.rs`): the marks are those `pass_semi.py` wrote
//! on the same plan.

use super::service::semijoined;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::finish_plans::types::FinishPlan;
use crate::domain::plan_lowering::fixtures::{select_marks, small_plan};

type Input = FinishPlan;
/// How many selects the pass marked, and each select's marks: (step, probed source, counted).
type Value = (usize, [(usize, Option<usize>, bool); 8]);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_select_keeping_one_source_whose_readers_cannot_count_its_rows_probes_the_other"),
    word: Word::Succeeds,
    input: small_plan,
    expect: Expect::Value((
        3,
        [
            (2, Some(1), false),
            (4, Some(0), false),
            (5, None, false),
            (6, None, false),
            (7, None, false),
            (9, Some(1), false),
            (12, None, false),
            (14, None, false),
        ],
    )),
}];

fn subject(mut plan: Input) -> Result<Value, Failure> {
    let marked = semijoined(&mut plan);
    Ok((marked, select_marks(&plan).try_into().map_err(|_| ())?))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
