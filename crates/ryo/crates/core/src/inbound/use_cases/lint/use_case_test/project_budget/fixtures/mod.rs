//! Actual lint dependencies; memory changes only after successful semantic reduction.
use super::super::*;
use crate::outbound::memory::{errors::MemoryError, interface::MemoryPort};
use ryo_execution::inbound::use_cases::execution::types::query_language::{
    QueryLanguageError, QueryLanguagePort,
};
use ryo_operators::domain::contract::types::OperatorAnswer;
use ryo_operators::domain::contract::types::OperatorCall;
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::{
    descriptions::types::ValidateReport,
    hosted::types::HostedRelation,
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
    },
    query_tokens::types::QueryBundle,
    runs::types::MemoryBytes,
};
use std::sync::atomic::AtomicUsize;

#[derive(Debug)]
struct ProjectPressure {
    reductions: AtomicUsize,
    pressure: MemoryPressure,
}
impl OperatorsUseCase for ProjectPressure {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        let reduction = matches!(&call, OperatorCall::ReduceCount { .. });
        let answer =
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()))
                .operators(call)?;
        if reduction {
            self.reductions.fetch_add(1, Ordering::SeqCst);
        }
        Ok(answer)
    }
}
impl MemoryPort for ProjectPressure {
    fn budget(&self) -> Result<MemoryBytes, MemoryError> {
        Ok(MemoryBytes(match self.pressure {
            MemoryPressure::Allowed => 4_000_000,
            _ => 1_000_000,
        }))
    }
    fn resident_bytes(&self) -> Result<MemoryBytes, MemoryError> {
        if self.reductions.load(Ordering::SeqCst) == 0 {
            return Ok(MemoryBytes(0));
        }
        match self.pressure {
            MemoryPressure::Unreadable => Err(MemoryError::Unreadable {
                reason: "after first project reduction".into(),
            }),
            _ => Ok(MemoryBytes(2_000_000)),
        }
    }
}
struct ProjectQueries {
    route: BudgetRoute,
}
impl QueryLanguagePort for ProjectQueries {
    fn validate(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[BoundObservationInput],
    ) -> Result<ValidateReport, QueryLanguageError> {
        QueryLanguageFake::selecting().validate(bundle, hosted, observations)
    }
    fn compile(
        &self,
        bundle: &QueryBundle,
        _hosted: &[HostedRelation],
        observations: &[BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError> {
        let first = &*bundle.root.text == "first";
        let mut plan = QueryLanguageFake::selecting().plan;
        plan.observation_inputs = observations.to_vec();
        let from = if self.route == BudgetRoute::Named && !first {
            PlanInput::BoundObservation(
                ryo_values::domain::plan_parts::types::ObservationBindingId(0),
            )
        } else {
            PlanInput::Step(StepId(0))
        };
        plan.frontier = vec![from];
        plan.steps = if self.route == BudgetRoute::Named && !first {
            Vec::new()
        } else {
            let file = ryo_relations::domain::relations::service::kinds(Relation::Function)
                .iter()
                .position(|kind| *kind == CellKind::File)
                .expect("canonical Function file column");
            let file = ColumnIndex(u32::try_from(file).unwrap());
            // These are actual captured file populations, not two request-invariant
            // Hosted operations with the same empty invocation source map.
            let operation = if first {
                Operation::Select {
                    from: PlanInput::Relation(Relation::Function),
                    comparisons: vec![],
                    kept: vec![file],
                }
            } else {
                Operation::Project {
                    from: PlanInput::Relation(Relation::Function),
                    columns: vec![file],
                    distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
                }
            };
            vec![PlanStep {
                scope: StepScope::PerFile,
                operation,
            }]
        };
        plan.steps.push(PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::GroupCount {
                from,
                keys: vec![ColumnIndex(0)],
                distinct: None,
            },
        });
        plan.emit = Emit {
            step: StepId((plan.steps.len() - 1) as u32),
            subject: ColumnIndex(0),
            key: vec![ColumnIndex(1)],
            headings: vec![],
        };
        if self.route == BudgetRoute::Named && first {
            plan.observations = vec![PlanObservation {
                name: ObservationName("counts".into()),
                source: PlanInput::Step(plan.emit.step),
                columns: count_columns(),
                scope: StepScope::CrossFile,
            }];
        }
        Ok(plan)
    }
}
fn count_columns() -> Vec<ObservationColumn> {
    vec![
        ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        },
        ObservationColumn {
            name: ObservationColumnName("count".into()),
            kind: CellKind::Number,
        },
    ]
}

pub(super) fn observe(scenario: BudgetScenario) -> BudgetObservation {
    let effect = Arc::new(ProjectPressure {
        reductions: AtomicUsize::new(0),
        pressure: scenario.pressure,
    });
    let given = Given {
        files: vec![("a.rs", "fn a() {}"), ("b.rs", "fn b() {}")],
        rules: vec!["first", "second"],
        ids: vec!["first", "second"],
        ..violations()
    };
    let mut asked = request(&given);
    if scenario.route == BudgetRoute::Named {
        asked.rules[1].observations = vec![ObservationBinding {
            name: ObservationBindingName("counts".into()),
            columns: count_columns(),
            producers: vec![ObservationProducer::Output {
                rule: RuleId("first".into()),
                output: ObservationName("counts".into()),
            }],
        }];
    }
    let world = scheduled_world_with_query_resources(
        given,
        Arc::new(crate::outbound::scheduler::factory::sequential()),
        WorkerCount(1),
        Arc::new(ProjectQueries {
            route: scenario.route,
        }),
        effect.clone(),
        effect.clone(),
    );
    let report = world.use_case.lint(asked);
    BudgetObservation {
        report,
        reductions: ReductionCount(effect.reductions.load(Ordering::SeqCst)),
    }
}

use crate::inbound::use_cases::lint::{errors::LintError, types::LintReport};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BudgetRoute {
    Ordinary,
    Named,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MemoryPressure {
    Budget,
    Unreadable,
    Allowed,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct BudgetScenario {
    pub route: BudgetRoute,
    pub pressure: MemoryPressure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ReductionCount(pub usize);
pub(super) struct BudgetObservation {
    pub report: Result<LintReport, LintError>,
    pub reductions: ReductionCount,
}
