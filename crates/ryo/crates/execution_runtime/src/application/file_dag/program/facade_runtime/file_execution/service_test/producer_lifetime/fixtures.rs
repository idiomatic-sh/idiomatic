//! Actual compiled producer plans shared by the internal and external reader controls.
pub(super) mod scenario;
use super::super::super::service::*;
use super::super::fixtures::*;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::runtime::types::AttemptCheckpoint;
use crate::application::file_dag::program::runtime::types::IndexLiveness;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::derived::owner::witness::service::{
    BackingIdentity, BackingLiveness, BackingWitness,
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{Integer, Relation, Value as Cell};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId},
};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    rows::types::RowCount,
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, Input as PlanInput, JoinColumn, RepeatedRows, Side, StepId, StepScope,
    },
    plans::types::{Emit, Operation, Plan, PlanStep},
    rows::types::Table,
};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(super) enum ProjectionOrder {
    Forward,
    Reversed,
}
pub(super) fn shared_plan(order: ProjectionOrder, outcome: FinalConsumer) -> Plan {
    let swapped = matches!(order, ProjectionOrder::Reversed);
    let invalid_terminal = matches!(outcome, FinalConsumer::InvalidTerminal);
    let refuse_provider = matches!(outcome, FinalConsumer::RefusingProvider);
    let mut steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: PlanInput::Relation(Relation::LanguageTarget),
                columns: vec![ColumnIndex(0), ColumnIndex(1), ColumnIndex(2)],
                distinct: RepeatedRows::Kept,
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(0)),
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(1)],
                right_on: vec![ColumnIndex(1)],
                output: vec![
                    JoinColumn {
                        side: Side::Left,
                        column: ColumnIndex(1 + u32::from(swapped)),
                    },
                    JoinColumn {
                        side: Side::Right,
                        column: ColumnIndex(1 + u32::from(!swapped)),
                    },
                ],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    if refuse_provider {
        // This actual reader first demands the first Join result, then the
        // shared producer, and finally a schema-valid unavailable population.
        // The cancellation therefore occurs after the first consumer released.
        if let Operation::Join { output, .. } = &mut steps[2].operation {
            *output = vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(1),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(2),
                },
            ];
        }
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Axis {
                from: PlanInput::Step(StepId(2)),
                edges: PlanInput::Step(StepId(1)),
                domain: PlanInput::Relation(Relation::SemanticChild),
                direction: ryo_values::domain::plan_parts::types::Direction::Descendant,
                id: ColumnIndex(0),
                parent: ColumnIndex(1),
                child: ColumnIndex(2),
                domain_id: ColumnIndex(2),
                domain_filter: vec![],
                domain_view: vec![],
                output: vec![],
            },
        });
    }
    if invalid_terminal {
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: PlanInput::Step(StepId(2)),
                columns: vec![ColumnIndex(99)],
                distinct: RepeatedRows::Kept,
            },
        });
    }
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        emit: Emit {
            step: StepId((steps.len() - 1) as u32),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
        frontier: vec![],
        comparison_origins: vec![],
    }
}
pub(super) fn numbers(table: &Table) -> Vec<Integer> {
    table
        .cells
        .iter()
        .map(|value| match value {
            Cell::Number(number) => *number,
            other => panic!("number output required, got {other:?}"),
        })
        .collect()
}
#[derive(Clone, Copy)]
pub(super) enum FinalConsumer {
    Completes,
    SkippedAtStart,
    InvalidTerminal,
    RefusingProvider,
}
