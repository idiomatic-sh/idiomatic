//! The public protocol is the only execution entry used by these controls.
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::EmittedRoot;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::FileStop;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleAfter;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::outbound::query_language::errors::QueryLanguageError;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution_runtime::outbound::operators::fixtures::suite::ReductionAudit;
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
use std::sync::Arc;
#[cfg(test)]
mod boundary_fixture;
use boundary_fixture::fixtures::{
    FileRun, authored_request, corpus, engine, prepared, prepared_rules, ran, root, run_file,
    run_file_into, source,
};

#[test]
fn public_group_reduction_and_emission_use_the_admitted_semantic_path() {
    let semantics = Arc::new(ReductionAudit::default());
    let engine = PrepareUseCaseImpl::new(
        semantics.clone(),
        Arc::new(ryo_execution::inbound::use_cases::execution::fixtures::derived()).into(),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    let mut input = authored_request(1);
    boundary_fixture::fixtures::plans(&mut input)[0]
        .steps
        .push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::GroupCount {
                from: Input::Step(StepId(0)),
                keys: vec![],
                distinct: None,
            },
        });
    boundary_fixture::fixtures::plans(&mut input)[0].emit.step = StepId(1);
    let program = engine.prepare(input).expect("prepare real count");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let table = source();
    let facts = TableFacts::new(&[(Relation::Function, &table)]);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let mut symbols = Symbols::default();
    let mut rules = run_file(
        &request,
        FileRun::of(
            &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }],
            &elements,
        ),
        &mut symbols,
        &facts,
    );
    let Ok(RootOutcome::Complete(table)) = root(ran(&mut rules, 0)) else {
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
    assert!(matches!(rules.end, FileEnd::Finished(_)), "finish count");
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

mod combined;
mod compiled_rules;
mod interpreter;
