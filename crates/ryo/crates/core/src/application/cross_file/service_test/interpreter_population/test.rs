//! Project populations stay separate by rule, payload and reached region.

use super::fixtures::many_distinct_project_populations_allocate_only_reached_region_state;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rule_requests::errors::RuleRequestsError;

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `many_distinct_project_populations_allocate_only_reached_region_state` scenario.
    ManyDistinctProjectPopulationsAllocateOnlyReachedRegionState,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// The rule requests a scenario's run refuses.
type Failure = RuleRequestsError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("many_distinct_project_populations_allocate_only_reached_region_state"),
    word: Word::Succeeds,
    input: || Given::ManyDistinctProjectPopulationsAllocateOnlyReachedRegionState,
    expect: Expect::Value(
        "InvocationStateCounts { populations: InvocationStateCount(0), slots: InvocationStateCount(0), instruction_counters: InvocationStateCount(0), dormant_obligations: InvocationStateCount(0) }|2|1|1|1|true|2|2|2|2|true|2|3|3|3|true|2|4|4|4|true|2|5|5|5|true|2|6|6|6|true|2|7|7|7|true|2|8|8|8|true|8|16",
    ),
}];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::ManyDistinctProjectPopulationsAllocateOnlyReachedRegionState => {
            many_distinct_project_populations_allocate_only_reached_region_state()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
