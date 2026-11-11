//! The memory budget stops a later population, and a refused frontier keeps its compiled source.
use super::fixtures::{GrowingMemory, ScriptedMemory};
use crate::application::cross_file::fixtures::ProjectCalls;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::evaluation::evaluate_rules;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::service;
use crate::application::cross_file::fixtures::world_with;
use crate::application::cross_file::fixtures::world_with_applicability;
use crate::cases::{Case, CaseName, Expect, Word, run};
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

fn project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase()
-> Result<String, RuleRequestsError> {
    let mut seen: Vec<String> = Vec::new();
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
            seen.push("the later cohort ran".to_owned());
            return Ok(seen.join("|"));
        };
        seen.push(format!("{:?}", completed.len()));
        seen.push(format!(
            "{:?}",
            completed
                .into_iter()
                .next()
                .unwrap()
                .unwrap()
                .crossed
                .rendered
                .is_some()
        ));
        seen.push(format!("{:?}", matches!(error, crate::application::cross_file::errors::CrossFileError::Budget(crate::domain::budget::errors::BudgetError::Exceeded { used: MemoryBytes(200), budget: MemoryBytes(100), ref plans }) if plans == &[rules.ids[order[1]].clone()])));
        seen.push(format!(
            "{:?}",
            memory.actual.reductions.load(Ordering::SeqCst)
        ));
        seen.push(format!(
            "{:?}",
            memory.actual.aggregates.load(Ordering::SeqCst)
        ));
        phase
            .finish()
            .expect("all scopes settle after budget refusal");
    }
    Ok(seen.join("|"))
}

fn refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort()
-> Result<String, RuleRequestsError> {
    let mut seen: Vec<String> = Vec::new();
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
        let results = crate::application::cross_file::fixtures::evaluation::evaluate_all(
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
        seen.push(format!("{:?}", results.len()));
        for result in results {
            seen.push(format!("{:?}", matches!(result.expect("the occurrence is evaluated").emit, RootOutcome::Unknown(OperatingError::ObservationIncomplete { ref reason }) if reason == "every source partition was explicitly skipped")));
        }
        seen.push(format!("{:?}", calls.reductions.load(Ordering::SeqCst)));
        seen.push(format!("{:?}", calls.aggregates.load(Ordering::SeqCst)));
        phase.finish().unwrap();
    }
    Ok(seen.join("|"))
}

/// The ordinary phase checks the budget for the first rule before it opens, and for each later occurrence before its preparation:
/// a refusal stays with its own occurrence, a later occurrence is still prepared and fails on its own, the prepared one is
/// evaluated and completed, and the phase is released; a refusal of the first rule refuses the whole phase.
fn ordinary_phase_keeps_each_refusal_with_its_occurrence_in_declaration_order()
-> Result<String, RuleRequestsError> {
    let mut seen: Vec<String> = Vec::new();
    use crate::application::cross_file::errors::CrossFileError;
    use crate::application::cross_file::fixtures::kept_facts;
    use crate::application::cross_file::types::OrdinaryRun;
    use crate::domain::budget::errors::BudgetError;
    use ryo_values::domain::stored_facts::types::Bucket;
    let plan = counting_plan(1, vec![]);
    let rules = world_with(vec![plan.clone(), plan], Arc::new(ProjectCalls::default()));
    let (facts, project) = kept_facts(&rules, &[(RuleIndex(0), &frontier_rows())]);
    let project_symbols = Symbols::default();
    // Rule 0 twice, then rule 1, whose frontier has no first bucket.
    let todo = [
        Todo {
            rule: RuleIndex(0),
            id: rules.ids[0].clone(),
        },
        Todo {
            rule: RuleIndex(0),
            id: rules.ids[0].clone(),
        },
        Todo {
            rule: RuleIndex(1),
            id: rules.ids[1].clone(),
        },
    ];
    let first_bucket = [Bucket(0)];
    let run = || OrdinaryRun {
        crossing: crossing(&rules),
        services: &rules.request,
        todo: &todo,
        first_bucket: &first_bucket,
        facts: facts.as_ref(),
        project: project.as_ref(),
        project_symbols: &project_symbols,
        ids: &rules.ids,
        budget: MemoryBytes(100),
    };
    // The first rule's check passes, the second occurrence's is over the budget, the third's passes.
    let memory = ScriptedMemory {
        readings: &[0, 200, 0],
        read: std::sync::atomic::AtomicUsize::new(0),
    };
    let outcome = service()
        .ordinary(run(), &memory)
        .expect("the phase runs")
        .expect("three occurrences make a phase");
    seen.push(format!("{:?}", outcome.outcomes.len()));
    let mut outcomes = outcome.outcomes.into_iter();
    let first = outcomes.next().unwrap();
    let first = match first.result {
        Ok(first) => first,
        Err(stop) => {
            seen.push(format!("{stop:?}"));
            return Ok(seen.join("|"));
        }
    };
    seen.push(format!(
        "{:?}",
        ((
            first.crossed.unknowns.len(),
            first
                .crossed
                .rendered
                .map(|rendered| (rendered.order.len(), rendered.cells.len())),
            first.crossed.observations.is_none(),
            first.publications.len()
        )) == ((0, Some((2, 10)), true, 0))
    ));
    let second = outcomes.next().unwrap();
    seen.push(format!("{:?}", matches!(second.result, Err(CrossFileError::Budget(BudgetError::Exceeded { used: MemoryBytes(200), budget: MemoryBytes(100), ref plans })) if plans == &[rules.ids[0].clone()])));
    let third = outcomes.next().unwrap();
    seen.push(format!("{:?}", matches!(third.result, Err(CrossFileError::Rules(RuleRequestsError::Failed { ref id, ref reason })) if id == &rules.ids[1] && reason == "missing frontier bucket")));
    seen.push(format!("{:?}", outcome.refused.is_none()));
    seen.push(format!("{:?}", matches!(outcome.finished, Ok(()))));
    seen.push(format!("{:?}", memory.read.load(Ordering::SeqCst) >= 3));
    // Over the budget before the phase opens: the whole phase is refused for the first rule.
    let refused = service().ordinary(
        run(),
        &ScriptedMemory {
            readings: &[200],
            read: std::sync::atomic::AtomicUsize::new(0),
        },
    );
    seen.push(format!("{:?}", matches!(refused, Err(CrossFileError::Budget(BudgetError::Exceeded { ref plans, .. })) if plans == &[rules.ids[0].clone()])));
    Ok(seen.join("|"))
}

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase` scenario.
    ProjectBudgetStopsALaterPopulationAfterActualWorkAndReleasesThePhase,
    /// The `refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort` scenario.
    RefusedFrontierOrdinalKeepsItsActualCompiledSourceInEachCohort,
    /// The `ordinary_phase_keeps_each_refusal_with_its_occurrence_in_declaration_order` scenario.
    OrdinaryPhaseKeepsEachRefusalWithItsOccurrenceInDeclarationOrder,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// What a scenario's run refuses.
type Failure = RuleRequestsError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase",
        ),
        word: Word::Succeeds,
        input: || Given::ProjectBudgetStopsALaterPopulationAfterActualWorkAndReleasesThePhase,
        expect: Expect::Value("1|true|true|1|2|1|true|true|1|2"),
    },
    Case {
        name: CaseName("refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort"),
        word: Word::Succeeds,
        input: || Given::RefusedFrontierOrdinalKeepsItsActualCompiledSourceInEachCohort,
        expect: Expect::Value("2|true|true|0|0|2|true|true|0|0"),
    },
    Case {
        name: CaseName(
            "ordinary_phase_keeps_each_refusal_with_its_occurrence_in_declaration_order",
        ),
        word: Word::Succeeds,
        input: || Given::OrdinaryPhaseKeepsEachRefusalWithItsOccurrenceInDeclarationOrder,
        expect: Expect::Value("3|true|true|true|true|true|true|true"),
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
        Given::OrdinaryPhaseKeepsEachRefusalWithItsOccurrenceInDeclarationOrder => {
            ordinary_phase_keeps_each_refusal_with_its_occurrence_in_declaration_order()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
