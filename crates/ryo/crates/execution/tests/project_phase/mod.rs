//! A compiled project emit defers at file scope while real file frontiers remain available.
use ryo_execution_runtime::application::file_dag::observations::types::AdmittedOutput;
use ryo_execution_runtime::application::file_dag::frontiers::types::ReadinessProof;
use ryo_execution_runtime::application::file_dag::frontiers::types::InputReadiness;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort as _;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use crate::boundary_fixture::{authored_request, corpus, demand, engine, source};
use ryo_execution::inbound::use_cases::execution::interface::ProjectStage;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution_runtime::application::file_dag::frontiers::types::PublicationSlot;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::ProjectInput;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::FrontierDemand;
use ryo_execution_runtime::application::file_dag::program::project_phase::types::FrontierRequest;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectObservation;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::ProjectOutputOutcome;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::Covered;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::EmitDemand;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::Partials;
use ryo_execution_domain::domain::row_emission::types::RowEmission;
use ryo_execution::inbound::use_cases::execution::interface::RuleDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_relations::domain::relations::{
    fixtures::kinds,
    types::{CellKind, FileId, Relation, Value},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
};
use ryo_values::domain::{
    plan_parts::types::{InputNumber, JoinColumn, Side},
    stored_facts::types::Bucket,
};
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
mod applicability;
#[cfg(test)]
mod consumers;
#[cfg(test)]
mod declared_empty;
mod restoration;
mod source_facts;
use self::source_facts::fixtures::{CapturedFacts, ProjectFacts};

fn mixed_program(engine: &PrepareUseCaseImpl, relation: Relation) -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    let file_column = kinds(Relation::Function)
        .iter()
        .position(|kind| *kind == CellKind::File)
        .unwrap();
    crate::boundary_fixture::plans(&mut input)[0].steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: vec![],
                kept: vec![ColumnIndex(u32::try_from(file_column).unwrap())],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Select {
                from: Input::Relation(relation),
                comparisons: vec![],
                kept: vec![ColumnIndex(0)],
            },
        },
        PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::Join {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    crate::boundary_fixture::plans(&mut input)[0].frontier = vec![Input::Step(StepId(0)), Input::Relation(relation)];
    crate::boundary_fixture::plans(&mut input)[0].emit.step = StepId(2);
    engine
        .prepare(input)
        .expect("compile actual mixed-phase request")
}

pub(crate) mod negative;

mod population_reads;

mod grouping;

mod named_selection;

mod text_identity;

mod union;
