//! The public protocol is the only execution entry used by these controls.
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::outbound::query_language::errors::QueryLanguageError;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::outbound::operators::fixtures::boundary::suite::ReductionAudit;
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
#[cfg(test)]
mod boundary_fixture;
use boundary_fixture::fixtures::{
    authored_request, corpus, demand, engine, prepared, prepared_rules, source,
};

#[test]
fn public_group_reduction_and_emission_use_the_admitted_semantic_path() {
    let semantics = Arc::new(ReductionAudit::default());
    let engine = PrepareUseCaseImpl::new(
        semantics.clone(),
        Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    let mut input = authored_request(1);
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .steps
        .push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::GroupCount {
                from: Input::Step(StepId(0)),
                keys: vec![],
                distinct: None,
            },
        });
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .emit
        .step = StepId(1);
    let program = engine.prepare(input).expect("prepare real count");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
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
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
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

/// S14: a forwarded root is evaluated once: a sink that refuses its rows leaves the committed publication, and the retry delivers
/// it again without another reduction.
#[test]
fn a_refused_sink_is_retried_from_the_publication_without_another_evaluation() {
    let semantics = Arc::new(ReductionAudit::default());
    let engine = PrepareUseCaseImpl::new(
        semantics.clone(),
        Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    let mut input = authored_request(1);
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .steps
        .push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::GroupCount {
                from: Input::Step(StepId(0)),
                keys: vec![],
                distinct: None,
            },
        });
    crate::boundary_fixture::fixtures::plans(&mut input)[0]
        .emit
        .step = StepId(1);
    let program = engine.prepare(input).expect("prepare a count");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
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
                    diagnostic: DiagnosticDelivery::Forwarded,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::Disabled,
                }],
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("open the count file");
    let refused = file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| {
        Err(OperatingError::NotRead)
    });
    assert!(
        matches!(refused, Err(OperatingError::NotRead)),
        "the sink refused"
    );
    let mut rows = vec![];
    let _emission = file
        .emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
            rows.push(row.to_vec());
            Ok(())
        })
        .expect("the retry delivers");
    assert_eq!(rows, [vec![Value::Number(Integer(1))]]);
    assert_eq!(
        semantics
            .reductions
            .load(std::sync::atomic::Ordering::Relaxed),
        1,
        "the count was reduced once; the retry replayed its publication"
    );
    let _finished = file.finish();
    request.finish();
}

#[cfg(test)]
mod union_rows;

#[cfg(test)]
mod mode_order;
#[cfg(test)]
mod publication;

#[cfg(test)]
mod project_phase;

#[cfg(test)]
mod shared_walk;

#[cfg(test)]
mod file_intents;

#[cfg(test)]
pub(crate) mod scheduler;

#[cfg(test)]
mod submission;

mod completion;

mod preflight;

mod resources;

#[cfg(test)]
mod file_run;

mod compiled_rules;
mod interpreter;
