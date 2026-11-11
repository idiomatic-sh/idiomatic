//! The route controls' harness: the operators that count the meanings entered, and one rule run through the real compiled owner.
use super::super::super::*;
use super::super::fixtures::*;
use crate::application::file_dag::execution::CommonOperatorEngine;
use ryo_execution_domain::domain::coverage::types::Partial;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{FileId, Relation},
};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, Test};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plans::types::Plan,
    rows::types::Table,
    stored_facts::types::Digest,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId, StepScope},
    plans::types::{Emit, Operation, PlanStep},
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
#[derive(Debug, Default)]
pub(crate) struct MeaningCalls {
    pub(crate) ordered: AtomicUsize,
    pub(crate) joins: AtomicUsize,
    pub(crate) sets: AtomicUsize,
}
impl crate::outbound::operators::interface::OperatorsPort for MeaningCalls {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        match &call {
            OperatorCall::OrderedExistence { .. } => {
                self.ordered.fetch_add(1, Ordering::Relaxed);
            }
            OperatorCall::Evaluate {
                operation:
                    ryo_values::domain::plans::types::Operation::Union { .. }
                    | ryo_values::domain::plans::types::Operation::Intersection { .. }
                    | ryo_values::domain::plans::types::Operation::Difference { .. }
                    | ryo_values::domain::plans::types::Operation::Antijoin { .. },
                ..
            } => {
                self.sets.fetch_add(1, Ordering::Relaxed);
            }
            OperatorCall::Evaluate {
                operation: ryo_values::domain::plans::types::Operation::Join { .. },
                ..
            } => {
                self.joins.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
        crate::application::file_dag::fixtures::operators().operators(call)
    }
}
pub(crate) fn run(
    plan: Plan,
    providers: &[(Relation, Table)],
    partials: &[Partial],
    calls: Arc<MeaningCalls>,
) -> RootOutcome {
    run_read(
        plan,
        providers,
        (partials, partials),
        Supplied::Stored,
        calls,
    )
}
/// How a provider hands its rows over: stored columns, or a computed table.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Supplied {
    Stored,
    Computed,
}
/// What the rule of `plan` answers when its evaluation sees the partial reads `evaluation` and its publication `publication`.
pub(crate) fn run_read(
    plan: Plan,
    providers: &[(Relation, Table)],
    (evaluation, publication): (&[Partial], &[Partial]),
    supplied: Supplied,
    calls: Arc<MeaningCalls>,
) -> RootOutcome {
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            plans: vec![plan],
            bindings: vec![vec![]],
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(calls)),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("route-control.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation,
            publication,
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let stored = providers
        .iter()
        .map(|(_, table)| {
            (0..table.kinds.len())
                .map(|column| {
                    FactColumn(
                        table
                            .cells
                            .chunks_exact(table.kinds.len())
                            .map(|row| to_bits(row[column]).0)
                            .collect(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let read = |relation| {
        providers
            .iter()
            .position(|(kind, _)| *kind == relation)
            .map(|at| {
                if supplied == Supplied::Computed {
                    return Rows::Computed(&providers[at].1);
                }
                Rows::Stored(ColumnsView {
                    kinds: kinds(relation),
                    columns: &stored[at],
                })
            })
            .ok_or(OperatingError::NotRead)
    };
    let outcome = file
        .rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            &mut symbols,
        )
        .unwrap();
    file.finish().unwrap();
    outcome.emit
}

pub(crate) fn ordered_strategy_fixture() -> (Plan, Vec<CellKind>) {
    use ryo_values::domain::plan_parts::types::{JoinColumn, Side};
    let schema = kinds(ryo_relations::domain::relations::types::Relation::SourceLocation).to_vec();
    let number = ColumnIndex(
        schema
            .iter()
            .position(|kind| *kind == CellKind::Number)
            .expect("typed numeric source") as u32,
    );
    let source = Operation::Select {
        from: PlanInput::Relation(
            ryo_relations::domain::relations::types::Relation::SourceLocation,
        ),
        comparisons: vec![],
        kept: vec![],
    };
    let join = Operation::Join {
        left: PlanInput::Step(StepId(0)),
        right: PlanInput::Step(StepId(0)),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: number,
            },
            JoinColumn {
                side: Side::Right,
                column: number,
            },
        ],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    let select = Operation::Select {
        from: PlanInput::Step(StepId(1)),
        comparisons: vec![PlanComparison {
            test: Test::LessThan,
            left: Operand::Column(ColumnIndex(0)),
            right: Operand::Column(ColumnIndex(1)),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        }],
        kept: vec![],
    };
    let project = Operation::Project {
        from: PlanInput::Step(StepId(2)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let anti = Operation::Antijoin {
        left: PlanInput::Step(StepId(0)),
        right: PlanInput::Step(StepId(3)),
        left_on: vec![number],
        right_on: vec![ColumnIndex(0)],
    };
    (
        Plan {
            observation_inputs: Vec::new(),
            observations: Vec::new(),
            parameters: vec![],
            steps: [source, join, select, project, anti]
                .into_iter()
                .map(|operation| PlanStep {
                    operation,
                    scope: StepScope::PerFile,
                })
                .collect(),
            comparison_origins: vec![],
            emit: Emit {
                step: StepId(4),
                subject: ColumnIndex(0),
                key: vec![],
                headings: vec![],
            },
            frontier: vec![],
        },
        schema,
    )
}
