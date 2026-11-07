//! The memory budget stops a later population, and a refused frontier keeps its compiled source.
use super::batch_test::sealed_fixture_batch;
use crate::application::cross_file::fixtures::ProjectCalls;
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::frontier_rows;
use crate::application::cross_file::fixtures::prepared_in;
use crate::application::cross_file::fixtures::service;
use crate::application::cross_file::fixtures::world_with;
use crate::application::cross_file::fixtures::world_with_applicability;
use crate::domain::answering::types::Todo;
use crate::domain::crossing::service::partials_for;
use crate::outbound::memory::errors::MemoryError;
use crate::outbound::memory::interface::MemoryPort;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::types::ProjectBatch;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::contract::types::OperatorAnswer;
use ryo_operators::domain::contract::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::domain::contract::types::PortableOperatorFault;
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
impl OperatorsUseCase for GrowingMemory {
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
        let mut scopes = Vec::new();
        let mut frontiers = Vec::new();
        let mut todos = Vec::new();
        for rule in order {
            let source = if rule == 0 { &original } else { &different };
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                RuleIndex(rule),
                &rules.covered,
                source,
                &mut symbols,
            )
            .unwrap();
            scopes.push(scope);
            frontiers.push(frontier);
            todos.push(Todo {
                rule: RuleIndex(rule),
                id: rules.ids[rule].clone(),
            });
        }
        let tasks = sealed_fixture_batch(&todos, scopes, frontiers);
        let guarded =
            service().evaluate_batch(&crossing(&rules), tasks, &phase, &mut symbols, |rule| {
                Ok(crate::domain::budget::service::within(
                    memory.resident_bytes()?,
                    memory.budget()?,
                    || vec![rules.ids[rule.0].clone()],
                )?)
            });
        let ProjectBatch::Refused { completed, error } = guarded else {
            panic!("later cohort must be refused")
        };
        assert_eq!(completed.len(), 1);
        assert!(
            completed
                .into_iter()
                .next()
                .unwrap()
                .result
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
    use ryo_execution::inbound::use_cases::execution::{
        interface::ProjectRootExecution,
        types::{
            EmitDemand, ObservationDemand, OperatingError, ProjectInput, ProjectTask, RootOutcome,
            RuleDemand,
        },
    };
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
        let mut scopes = vec![];
        let mut frontiers = vec![];
        let mut policies = vec![];
        for rule in order {
            let (scope, frontier) = prepared_in(
                &rules,
                &phase,
                RuleIndex(rule),
                &rules.covered,
                &[],
                &mut symbols,
            )
            .unwrap();
            scopes.push(scope);
            frontiers.push(frontier);
            policies.push(partials_for(&crossing(&rules).domain, RuleIndex(rule)));
        }
        let tasks = order
            .into_iter()
            .zip(&scopes)
            .zip(frontiers)
            .zip(&policies)
            .map(|(((rule, scope), ordinary), policy)| ProjectTask {
                rule: RuleIndex(rule),
                demand: RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::NotRequired,
                },
                input: ProjectInput {
                    scope,
                    evaluation: policy,
                    publication: policy,
                    ordinary,
                    bound: &[],
                    arenas: &rules.arenas,
                },
            })
            .collect();
        let ProjectBatch::Complete(results) = phase.evaluate(tasks, &mut symbols, |_| {
            Ok::<(), std::convert::Infallible>(())
        }) else {
            unreachable!()
        };
        assert_eq!(results.len(), 2);
        for result in results {
            assert!(
                matches!(result.unwrap().emit, RootOutcome::Unknown(OperatingError::ObservationIncomplete { ref reason }) if reason == "every source partition was explicitly skipped")
            );
        }
        assert_eq!(calls.reductions.load(Ordering::SeqCst), 0);
        assert_eq!(calls.aggregates.load(Ordering::SeqCst), 0);
        assert_eq!(
            phase.state_counts().populations.0,
            2,
            "same local #1 refusal belongs to different compiled frontier roots"
        );
        drop(scopes);
        phase.finish().unwrap();
    }
    Ok(())
}
