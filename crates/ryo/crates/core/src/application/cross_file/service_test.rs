// the cases share one injected dependency scenario to prove retained-data loading, resident lookups, ordered output, and each required port failure; keeping the fixtures and the complete case table beside the application operation makes the same boundary visible in every case.
//! The `cross_file` service's cases, on values alone.

use super::fixtures::{counting_plan, fed_steps, frontier_rows, todo, world};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest as _;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;

#[cfg(test)]
mod budget;
#[cfg(test)]
mod generation;
#[cfg(test)]
mod interpreter_budget;
#[cfg(test)]
mod interpreter_population;
#[cfg(test)]
mod interpreter_root_set;
#[cfg(test)]
mod partials;
#[cfg(test)]
mod population;
#[cfg(test)]
mod rendering;
#[cfg(test)]
mod root_set;
#[cfg(test)]
mod single_rule;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The nodes two files handed over, counted.
    CrossCounts,
    /// A rule the plans lack.
    CrossNoRule,
    /// A cross-file step that reads what nobody handed over.
    CrossNothingHanded,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// Up to three numbers a row answers; a place a row has no number for holds zero.
type Value = [usize; 3];
type Failure = RuleRequestsError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::CrossCounts => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let phase = rules.request.project_phase().expect("one project phase");
            let done = super::fixtures::evaluation::evaluate_fixture(
                &rules,
                &todo(0),
                &frontier_rows(),
                &rules.covered,
                &mut cross,
                &phase,
            )?;
            Ok([
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.order.len()),
                fed_steps(&rules, 0),
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.cells.len()),
            ])
        }
        Scenario::CrossNoRule => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let phase = rules.request.project_phase().expect("one project phase");
            let done = super::fixtures::evaluation::evaluate_fixture(
                &rules,
                &todo(3),
                &frontier_rows(),
                &rules.covered,
                &mut cross,
                &phase,
            )?;
            Ok([
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.order.len()),
                fed_steps(&rules, 3),
                0,
            ])
        }
        Scenario::CrossNothingHanded => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let phase = rules.request.project_phase().expect("one project phase");
            let done = super::fixtures::evaluation::evaluate_fixture(
                &rules,
                &todo(0),
                &[],
                &rules.covered,
                &mut cross,
                &phase,
            )?;
            Ok([
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.order.len()),
                0,
                0,
            ])
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "the_nodes_two_files_handed_over_are_counted_into_two_findings_of_five_cells",
        ),
        word: Word::Succeeds,
        input: || Scenario::CrossCounts,
        expect: Expect::Value([2, 2, 10]),
    },
    Case {
        name: CaseName("an_unprepared_rule_refuses_the_project_scope"),
        word: Word::Refuses,
        input: || Scenario::CrossNoRule,
        expect: Expect::Refusal(|error| {
            if matches!(error, RuleRequestsError::Failed { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_step_that_reads_what_nobody_handed_over_fails_the_rule"),
        word: Word::Refuses,
        input: || Scenario::CrossNothingHanded,
        expect: Expect::Refusal(|error| {
            if matches!(error, RuleRequestsError::Failed { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
