//! The memory budget stops a later population, and a refused frontier keeps its compiled source.

use super::fixtures::{
    project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase,
    refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rule_requests::errors::RuleRequestsError;

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase` scenario.
    ProjectBudgetStopsALaterPopulationAfterActualWorkAndReleasesThePhase,
    /// The `refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort` scenario.
    RefusedFrontierOrdinalKeepsItsActualCompiledSourceInEachCohort,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// The rule requests a scenario's run refuses.
type Failure = RuleRequestsError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase",
        ),
        word: Word::Succeeds,
        input: || Given::ProjectBudgetStopsALaterPopulationAfterActualWorkAndReleasesThePhase,
        expect: Expect::Value("1|true|true|1|2|1|1|true|true|1|2|1"),
    },
    Case {
        name: CaseName("refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort"),
        word: Word::Succeeds,
        input: || Given::RefusedFrontierOrdinalKeepsItsActualCompiledSourceInEachCohort,
        expect: Expect::Value("2|true|true|0|0|2|2|true|true|0|0|2"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::ProjectBudgetStopsALaterPopulationAfterActualWorkAndReleasesThePhase => {
            project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase()?
        }
        Given::RefusedFrontierOrdinalKeepsItsActualCompiledSourceInEachCohort => {
            refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
