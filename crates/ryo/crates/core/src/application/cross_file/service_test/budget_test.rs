//! The memory budget stops a later population, and a refused frontier keeps its compiled source.
use super::batch_test::evaluate_rules;
use crate::application::cross_file::fixtures::ProjectCalls;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::service;
use crate::application::cross_file::fixtures::world_with;
use crate::application::cross_file::fixtures::world_with_applicability;
use crate::domain::answering::types::Todo;
use crate::domain::crossing::service::partials_for;
use crate::outbound::memory::errors::MemoryError;
use crate::outbound::memory::interface::MemoryPort;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Position;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use ryo_values::domain::runs::types::MemoryBytes;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
struct GrowingMemory {
    actual: ProjectCalls,
    resident: AtomicU64,
}
impl ryo_execution_runtime::outbound::operators::interface::OperatorsPort for GrowingMemory {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        let answer = self.actual.operators(call);
        self.resident.store(200, Ordering::SeqCst);
        answer
    }
}
impl MemoryPort for GrowingMemory {
    fn budget(&self) -> Result<MemoryBytes, MemoryError> {
        Ok(MemoryBytes(100))
    }
    fn resident_bytes(&self) -> Result<MemoryBytes, MemoryError> {
        Ok(MemoryBytes(self.resident.load(Ordering::SeqCst)))
    }
}

pub(super) fn project_budget_stops_a_later_population_after_actual_work_and_releases_the_phase()
-> Result<(), RuleRequestsError> {
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
            panic!("later cohort must be refused")
        };
        assert_eq!(completed.len(), 1);
        assert!(
            completed
                .into_iter()
                .next()
                .unwrap()
                .unwrap()
                .crossed
                .rendered
                .is_some()
        );
        assert!(
            matches!(error, crate::application::cross_file::errors::CrossFileError::Budget(crate::domain::budget::errors::BudgetError::Exceeded { used: MemoryBytes(200), budget: MemoryBytes(100), ref plans }) if plans == &[rules.ids[order[1]].clone()])
        );
        assert_eq!(memory.actual.reductions.load(Ordering::SeqCst), 1);
        assert_eq!(memory.actual.aggregates.load(Ordering::SeqCst), 2);
        assert_eq!(
            phase.state_counts().slots.0,
            1,
            "refused population never starts source admission or semantic work"
        );
        phase
            .finish()
            .expect("all scopes settle after budget refusal");
    }
    Ok(())
}

pub(super) fn refused_frontier_ordinal_keeps_its_actual_compiled_source_in_each_cohort()
-> Result<(), RuleRequestsError> {
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
        let results = super::batch_test::evaluate_all(
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
            |_, outcome, _, _| Ok(outcome),
        );
        assert_eq!(results.len(), 2);
        for result in results {
            assert!(
                matches!(result.expect("the occurrence is evaluated").emit, RootOutcome::Unknown(OperatingError::ObservationIncomplete { ref reason }) if reason == "every source partition was explicitly skipped")
            );
        }
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 0);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 0);
        assert_eq!(
            phase.state_counts().populations.0,
            2,
            "same local #1 refusal belongs to different compiled frontier roots"
        );
        phase.finish().unwrap();
    }
    Ok(())
}

/// Resident bytes read in turn from a script, then none: each read is the next reading.
#[derive(Debug)]
struct ScriptedMemory {
    readings: &'static [u64],
    read: std::sync::atomic::AtomicUsize,
}
impl MemoryPort for ScriptedMemory {
    fn budget(&self) -> Result<MemoryBytes, MemoryError> {
        Ok(MemoryBytes(100))
    }
    fn resident_bytes(&self) -> Result<MemoryBytes, MemoryError> {
        let at = self.read.fetch_add(1, Ordering::SeqCst);
        Ok(MemoryBytes(self.readings.get(at).copied().unwrap_or(0)))
    }
}

/// The ordinary phase checks the budget for the first rule before it opens, and for each later occurrence before its preparation:
/// a refusal stays with its own occurrence, a later occurrence is still prepared and fails on its own, the prepared one is
/// evaluated and completed, and the phase is released; a refusal of the first rule refuses the whole phase.
pub(super) fn ordinary_phase_keeps_each_refusal_with_its_occurrence_in_declaration_order()
-> Result<(), RuleRequestsError> {
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
    assert_eq!(
        outcome.outcomes.len(),
        3,
        "every occurrence answers, in declaration order"
    );
    let mut outcomes = outcome.outcomes.into_iter();
    let first = outcomes.next().unwrap();
    assert_eq!(first.todo.rule, RuleIndex(0));
    assert!(
        first.result.is_ok(),
        "the prepared occurrence is evaluated and completed"
    );
    let second = outcomes.next().unwrap();
    assert!(
        matches!(second.result, Err(CrossFileError::Budget(BudgetError::Exceeded { used: MemoryBytes(200), budget: MemoryBytes(100), ref plans })) if plans == &[rules.ids[0].clone()]),
        "the budget refusal stays with the occurrence it was checked for"
    );
    let third = outcomes.next().unwrap();
    assert!(
        matches!(third.result, Err(CrossFileError::Rules(RuleRequestsError::Failed { ref id, ref reason })) if id == &rules.ids[1] && reason == "missing frontier bucket"),
        "a later occurrence is still prepared after a refusal, and fails on its own"
    );
    assert!(outcome.refused.is_none(), "no cohort was refused");
    assert!(outcome.finished.is_ok(), "the phase is released");
    assert!(memory.read.load(Ordering::SeqCst) >= 3);
    // Over the budget before the phase opens: the whole phase is refused for the first rule.
    let refused = service().ordinary(
        run(),
        &ScriptedMemory {
            readings: &[200],
            read: std::sync::atomic::AtomicUsize::new(0),
        },
    );
    assert!(
        matches!(refused, Err(CrossFileError::Budget(BudgetError::Exceeded { ref plans, .. })) if plans == &[rules.ids[0].clone()])
    );
    Ok(())
}
