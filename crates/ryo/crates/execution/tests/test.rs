//! The public protocol is the only execution entry used by these controls.
use ryo_execution::outbound::operators::interface::OperatorsPort as _;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_execution::outbound::query_language::errors::QueryLanguageError;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution::outbound::facts::types::ResidentStability;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::application::file_dag::program::request_execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::application::file_dag::program::file_session::interface::FileExecution;
use ryo_execution::application::file_dag::program::file_session::types::FileInput;
use ryo_execution::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution::application::rule_compilation::service::QueryCompilation;
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::EmitDemand;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleDemand;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    fixtures::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, Plan, PlanStep},
    query_tokens::types::QueryBundle,
    rows::types::{RowCount, Table},
    stored_facts::types::Digest,
};
use std::{cell::Cell, sync::Arc};
mod boundary_fixture;
use boundary_fixture::{
    authored_request, corpus, demand, engine, prepared, prepared_rules, source,
};

use ryo_execution::application::rule_compilation::service::ExecutionCompilation;

#[derive(Debug, Default)]
struct ReductionAudit {
    reductions: std::sync::atomic::AtomicUsize,
    emissions: std::sync::atomic::AtomicUsize,
}
impl ryo_execution::outbound::operators::interface::OperatorsPort for ReductionAudit {
    fn operators(
        &self,
        call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
        use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
        use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
        match &call {
            OperatorCall::ReduceCount { .. } => {
                self.reductions
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            OperatorCall::Aggregate { .. } => {
                self.emissions
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {}
        }
        OperatorsUseCase::operators(&OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())), call)
    }
}
#[test]
fn public_group_reduction_and_emission_use_the_admitted_semantic_path() {
    let semantics = Arc::new(ReductionAudit::default());
    let engine = PrepareUseCaseImpl::new(
        semantics.clone(),
        Arc::new(ryo_execution::outbound::derived::factory::semantics()),
    );
    let mut input = authored_request(1);
    crate::boundary_fixture::plans(&mut input)[0].steps.push(PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::GroupCount {
            from: Input::Step(StepId(0)),
            keys: vec![],
            distinct: None,
        },
    });
    crate::boundary_fixture::plans(&mut input)[0].emit.step = StepId(1);
    let program = engine.prepare(input).expect("prepare real count");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let table = source();
    let read = |relation| {
        if relation == Relation::Function {
            Ok(SourceRows::Computed(&table))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Run {
                    diagnostic: DiagnosticDelivery::Collected,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                }],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("open count file");
    let RootOutcome::Complete(table) = file
        .rule(RuleIndex(0), demand(), &mut symbols)
        .expect("count through owner")
        .emit
    else {
        panic!("complete count")
    };
    assert_eq!(table.count, RowCount(1));
    assert_eq!(table.cells, [Value::Number(Integer(1))]);
    assert_eq!(
        semantics
            .reductions
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    assert_eq!(
        semantics
            .emissions
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
    file.finish().expect("finish count");
    request.finish();
}

mod union_rows;

mod mode_order;
mod publication;

mod project_phase;

#[cfg(test)]
mod shared_walk;

#[cfg(test)]
mod file_intents;

#[cfg(test)]
mod scheduler;

mod source_boundary;

mod file_run;

mod compiled_rules;
