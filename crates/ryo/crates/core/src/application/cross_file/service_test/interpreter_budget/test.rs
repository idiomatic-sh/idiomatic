//! The memory budget stops a later population, and a refused frontier keeps its compiled source.

use super::super::super::fixtures::evaluation::evaluate_rules;
use super::fixtures::GrowingMemory;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::project_calls::ProjectCalls;
use crate::application::cross_file::fixtures::world_with;
use crate::application::cross_file::fixtures::world_with_applicability;
use crate::domain::answering::types::Todo;
use crate::domain::crossing::service::partials_for;
use crate::outbound::memory::interface::MemoryPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Position;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use ryo_values::domain::runs::types::MemoryBytes;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::cases::{Case, CaseName, Expect, Word, run};

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

fn project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase()
-> Result<String, RuleRequestsError> {
    let mut observed: Vec<String> = Vec::new();
    for order in [[0, 1], [1, 0]] {
        let memory = Arc::new(GrowingMemory::default());
        let plan = counting_plan(1, vec![]);
        let rules = world_with(vec![plan.clone(), plan], memory.clone());
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let original = frontier_rows();
        let mut different = original.clone();
        for cell in &mut different[0].1.cells {
            *cell = Cell::Node(node_id(FileId(0), Position(9)));
        }
        let todos = order
            .iter()
            .map(|&rule| Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            })
            .collect::<Vec<_>>();
        // The budget's own check is the phase's guard: the production refusal path of the phase call.
        let guarded = evaluate_rules(
            &rules,
            &phase,
            &todos,
            &[(RuleIndex(0), &original), (RuleIndex(1), &different)],
            &mut symbols,
            |rule| {
                Ok(crate::domain::budget::service::within(
                    memory.resident_bytes()?,
                    memory.budget()?,
                    || vec![rules.ids[rule.0].clone()],
                )?)
            },
        );
        let ProjectBatch::Refused { completed, error } = guarded else {
            observed.push("the later cohort ran".to_owned());
            return Ok(observed.join("|"));
        };
        observed.push(format!("{:?}", completed.len()));
        observed.push(format!(
            "{}",
            completed
                .into_iter()
                .next()
                .unwrap()
                .unwrap()
                .crossed
                .rendered
                .is_some()
        ));
        observed.push(format!("{}", matches!(error, crate::application::cross_file::errors::CrossFileError::Budget(crate::domain::budget::errors::BudgetError::Exceeded { used: MemoryBytes(200), budget: MemoryBytes(100), ref plans }) if plans == &[rules.ids[order[1]].clone()])));
        observed.push(format!(
            "{:?}",
            memory.actual.reductions.load(Ordering::SeqCst)
        ));
        observed.push(format!(
            "{:?}",
            memory.actual.aggregates.load(Ordering::SeqCst)
        ));
        observed.push(format!("{:?}", phase.fixture_state_counts().slots.0));
        phase
            .finish()
            .expect("all scopes settle after budget refusal");
    }
    Ok(observed.join("|"))
}

fn refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort()
-> Result<String, RuleRequestsError> {
    let mut observed: Vec<String> = Vec::new();
    use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
    use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
    use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
    use ryo_values::domain::runs::types::{RuleApplicability, SkipReason, Skipped};
    for order in [[0, 1], [1, 0]] {
        let calls = Arc::new(ProjectCalls::default());
        let first = counting_plan(1, vec![]);
        let mut second = first.clone();
        let Operation::Project { distinct, .. } = &mut second.steps[0].operation else {
            unreachable!()
        };
        *distinct = RepeatedRows::Kept;
        let mut rules = world_with_applicability(
            vec![first, second],
            calls.clone(),
            &[RuleApplicability::AcceptedSyntax; 2],
        );
        for rule in [0, 1] {
            for path in &rules.paths {
                rules.skipped.push(Skipped {
                    rule: rules.ids[rule].clone(),
                    path: path.clone(),
                    reason: SkipReason::SyntaxRejected,
                });
            }
        }
        rules.covered.clear();
        let phase = rules.request.project_phase().unwrap();
        let mut symbols = Symbols::default();
        let declared = order.map(RuleIndex);
        let policies =
            declared.map(|rule| partials_for(&MetadataUseCaseImpl, &crossing(&rules).domain, rule));
        let results = super::super::super::fixtures::evaluation::evaluate_all(
            &rules,
            &phase,
            &declared,
            &[],
            &rules.covered,
            &mut symbols,
            |occurrence| {
                Ok(PhasePartials {
                    evaluation: policies[occurrence.0].clone(),
                    publication: policies[occurrence.0].clone(),
                })
            },
            |_, outcome, _, _, _| Ok(outcome),
        );
        observed.push(format!("{:?}", results.len()));
        for result in results {
            observed.push(format!("{}", matches!(result.expect("the occurrence is evaluated").emit, RootOutcome::Unknown(OperatingError::ObservationIncomplete { ref reason }) if reason == "every source partition was explicitly skipped")));
        }
        observed.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
        observed.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
        observed.push(format!("{:?}", phase.fixture_state_counts().populations.0));
        phase.finish().unwrap();
    }
    Ok(observed.join("|"))
}
