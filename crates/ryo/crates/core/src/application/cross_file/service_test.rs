// the cases share one injected dependency scenario to prove retained-data loading, resident lookups, ordered output, and each required port failure; keeping the fixtures and the complete case table beside the application operation makes the same boundary visible in every case.
//! The `cross_file` service's cases, on values alone.

use super::fixtures::{counting_plan, crossing, frontier_rows, prepared, todo, world};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;

#[cfg(test)]
mod batch_test;
#[cfg(test)]
mod budget_test;
#[cfg(test)]
mod generation_test;
#[cfg(test)]
mod partials_test;
#[cfg(test)]
mod population_test;
#[cfg(test)]
mod rendering_test;
#[cfg(test)]
mod root_set_test;
#[cfg(test)]
mod single_rule_test;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// Resource substitution changes rendered source spelling.
    Substitution,
    /// An unreadable source yields no invented text.
    Unreadable,
    /// Findings whose columns do and do not say where they are.
    Placeable,
    /// Files and the project that read a relation in part which a rule trusts.
    Partials,
    /// The nodes two files handed over, counted.
    CrossCounts,
    /// A rule the plans lack.
    CrossNoRule,
    /// A cross-file step that reads what nobody handed over.
    CrossNothingHanded,
    /// The roots of the current project keep the original two-group, four-cell result.
    CurrentRoots,
    /// Rejected syntax blocks the cross-file complements without inventing a partial diagnostic.
    RejectedSyntax,
    /// A project evaluation rejects frontiers admitted by another request.
    ForeignFrontiers,
    /// Ordinary project rules share one preparation and reject another generation.
    SharedPreparation,
    /// An ordinary phase keeps the populations and the partiality of its rules separate.
    SeparatePopulations,
    /// Equal policies do not alias distinct admitted frontier payloads.
    DistinctPayloads,
    /// Many distinct project populations allocate only the state of the region they reach.
    ReachedRegions,
    /// A project root set prepares shared publications once, before the ordered delivery.
    SharedPublications,
    /// A project root set keeps the distinct actual population policy in both orders.
    DistinctPolicies,
    /// The budget stops a later population after actual work and releases the phase.
    LaterPopulation,
    /// A refused frontier ordinal keeps its compiled source in each cohort.
    RefusedOrdinal,
    /// The ordinary phase keeps each refusal with its occurrence, in declaration order.
    OrdinaryOrder,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// Up to three numbers a row answers; a place a row has no number for holds zero.
type Value = [usize; 3];
type Failure = RuleRequestsError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Substitution => Ok([
            usize::from(rendering_test::source_text(b"one", false) == "one"),
            usize::from(rendering_test::source_text(b"two", false) == "two"),
            0,
        ]),
        Scenario::Unreadable => Ok([
            usize::from(rendering_test::source_text(b"one", true).is_empty()),
            0,
            0,
        ]),
        Scenario::Placeable => Ok(partials_test::placing_outcomes()),
        Scenario::Partials => Ok(partials_test::partials_found()),
        Scenario::CrossCounts => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let (_phase, scope, frontier) = prepared(&rules, &frontier_rows(), &mut cross)?;
            let done = batch_test::evaluate_fixture(
                &crossing(&rules),
                &todo(0),
                frontier,
                &mut cross,
                scope,
                &_phase,
            )?;
            Ok([
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.order.len()),
                done.fed.len(),
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.cells.len()),
            ])
        }
        Scenario::CrossNoRule => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let (_phase, scope, frontier) = prepared(&rules, &frontier_rows(), &mut cross)?;
            let done = batch_test::evaluate_fixture(
                &crossing(&rules),
                &todo(3),
                frontier,
                &mut cross,
                scope,
                &_phase,
            )?;
            Ok([
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.order.len()),
                done.fed.len(),
                0,
            ])
        }
        Scenario::CrossNothingHanded => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let (_phase, scope, frontier) = prepared(&rules, &[], &mut cross)?;
            let done = batch_test::evaluate_fixture(
                &crossing(&rules),
                &todo(0),
                frontier,
                &mut cross,
                scope,
                &_phase,
            )?;
            Ok([
                done.rendered
                    .as_ref()
                    .map_or(0, |rendered| rendered.order.len()),
                0,
                0,
            ])
        }
        Scenario::CurrentRoots => single_rule_test::current_project_roots_keep_the_original_two_group_four_cell_result().map(|()| [0; 3]),
        Scenario::RejectedSyntax => single_rule_test::rejected_syntax_blocks_cross_file_complements_without_inventing_partial_diagnostics().map(|()| [0; 3]),
        Scenario::ForeignFrontiers => single_rule_test::project_evaluation_rejects_frontiers_from_another_request().map(|()| [0; 3]),
        Scenario::SharedPreparation => generation_test::ordinary_project_rules_share_one_preparation_and_reject_another_generation().map(|()| [0; 3]),
        Scenario::SeparatePopulations => population_test::ordinary_phase_keeps_valid_rule_populations_and_partiality_separate().map(|()| [0; 3]),
        Scenario::DistinctPayloads => population_test::equal_policies_do_not_alias_distinct_admitted_frontier_payloads().map(|()| [0; 3]),
        Scenario::ReachedRegions => population_test::many_distinct_project_populations_allocate_only_reached_region_state().map(|()| [0; 3]),
        Scenario::SharedPublications => root_set_test::project_root_set_prepares_shared_publications_once_before_ordered_delivery().map(|()| [0; 3]),
        Scenario::DistinctPolicies => root_set_test::project_root_set_preserves_distinct_actual_population_policy_in_both_orders().map(|()| [0; 3]),
        Scenario::LaterPopulation => budget_test::project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase().map(|()| [0; 3]),
        Scenario::RefusedOrdinal => budget_test::refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort().map(|()| [0; 3]),
        Scenario::OrdinaryOrder => budget_test::ordinary_phase_keeps_each_refusal_with_its_occurrence_in_declaration_order().map(|()| [0; 3]),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("substituting_the_injected_corpus_changes_source_spelling"),
        word: Word::Succeeds,
        input: || Scenario::Substitution,
        expect: Expect::Value([1, 1, 0]),
    },
    Case {
        name: CaseName("failed_source_reads_never_invent_a_spelling"),
        word: Word::Succeeds,
        input: || Scenario::Unreadable,
        expect: Expect::Value([1, 0, 0]),
    },
    Case {
        name: CaseName("a_finding_is_placeable_by_a_name_or_a_key_that_says_where_it_is"),
        word: Word::Succeeds,
        input: || Scenario::Placeable,
        expect: Expect::Value([1, 0, 1]),
    },
    Case {
        name: CaseName(
            "the_partial_reads_of_the_project_and_the_files_that_govern_a_rule_bear_on_it",
        ),
        word: Word::Succeeds,
        input: || Scenario::Partials,
        expect: Expect::Value([2, 4_294_967_295, 2]),
    },
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
    Case {
        name: CaseName("current_project_roots_keep_the_original_two_group_four_cell_result"),
        word: Word::Succeeds,
        input: || Scenario::CurrentRoots,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName(
            "rejected_syntax_blocks_cross_file_complements_without_inventing_partial_diagnostics",
        ),
        word: Word::Succeeds,
        input: || Scenario::RejectedSyntax,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("project_evaluation_rejects_frontiers_from_another_request"),
        word: Word::Succeeds,
        input: || Scenario::ForeignFrontiers,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName(
            "ordinary_project_rules_share_one_preparation_and_reject_another_generation",
        ),
        word: Word::Succeeds,
        input: || Scenario::SharedPreparation,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("ordinary_phase_keeps_valid_rule_populations_and_partiality_separate"),
        word: Word::Succeeds,
        input: || Scenario::SeparatePopulations,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("equal_policies_do_not_alias_distinct_admitted_frontier_payloads"),
        word: Word::Succeeds,
        input: || Scenario::DistinctPayloads,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("many_distinct_project_populations_allocate_only_reached_region_state"),
        word: Word::Succeeds,
        input: || Scenario::ReachedRegions,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName(
            "project_root_set_prepares_shared_publications_once_before_ordered_delivery",
        ),
        word: Word::Succeeds,
        input: || Scenario::SharedPublications,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName(
            "project_root_set_preserves_distinct_actual_population_policy_in_both_orders",
        ),
        word: Word::Succeeds,
        input: || Scenario::DistinctPolicies,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName(
            "project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase",
        ),
        word: Word::Succeeds,
        input: || Scenario::LaterPopulation,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort"),
        word: Word::Succeeds,
        input: || Scenario::RefusedOrdinal,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("ordinary_phase_keeps_each_refusal_with_its_occurrence_in_declaration_order"),
        word: Word::Succeeds,
        input: || Scenario::OrdinaryOrder,
        expect: Expect::Value([0, 0, 0]),
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
