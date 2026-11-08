//! Sink refusal retries terminal delivery, while a new owner must admit a new population.
use ryo_execution::inbound::use_cases::execution::interface::FileExecution as _;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest as _;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase as _;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use crate::boundary_fixture::{authored_request, corpus, demand, engine, source};
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::*;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
};
use std::cell::Cell;

#[test]
fn sink_refusal_replays_delivery_without_restarting_the_shared_dependency_graph() {
    let engine = engine();
    let mut authored = authored_request(2);
    for plan in crate::boundary_fixture::plans(&mut authored) {
        plan.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: Input::Step(StepId(0)),
                right: Input::Step(StepId(0)),
            },
        });
        plan.emit.step = StepId(1);
    }
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let table = source();
    let reads = Cell::new(0);
    let refuse = Cell::new(false);
    let read = |relation| {
        assert_eq!(relation, Relation::Function);
        reads.set(reads.get() + 1);
        if refuse.get() {
            Err(OperatingError::NotRead)
        } else {
            Ok(SourceRows::Computed(&table))
        }
    };
    let elements = Elements::default();
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Forwarded,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }; 2];
    let mut symbols = Symbols::default();
    // An engine-issued generation cannot be reused by opening the same FileId again.
    for generation in 0..2 {
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &intents,
                    source: ryo_values::domain::files::types::SourceView(b"abc"),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )
            .unwrap();
        assert_eq!(file.readiness_counts().evaluations.0, 0);
        if generation == 0 {
            let callbacks = Cell::new(0);
            assert!(matches!(
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| {
                    callbacks.set(callbacks.get() + 1);
                    Err(OperatingError::WrongBinding)
                }),
                Err(OperatingError::WrongBinding)
            ));
            assert_eq!(callbacks.get(), 1);
            let work = file.readiness_counts();
            let admitted = reads.get();
            assert!(admitted > 0);
            assert!(work.evaluations.0 > 0);
            refuse.set(true);
            for index in [0, 1, 0] {
                let mut rows = vec![];
                file.emit_rows(RuleIndex(index), &mut symbols, &mut |row| {
                    rows.push(row.to_vec());
                    Ok(())
                })
                .expect("retry terminal publication through the public owner");
                assert_eq!(rows, [vec![table.cells[0]]]);
                assert_eq!(file.readiness_counts(), work);
                assert_eq!(reads.get(), admitted);
            }
        } else {
            let before = reads.get();
            assert!(matches!(
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| {
                    panic!("a retired generation must not supply a cached row")
                }),
                Err(OperatingError::NotRead)
            ));
            assert!(
                reads.get() > before,
                "the fresh generation admitted the current provider"
            );
            let failed = file.readiness_counts();
            let after = reads.get();
            assert!(matches!(
                file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| {
                    panic!("failed publication must remain unavailable")
                }),
                Err(OperatingError::NotRead)
            ));
            assert_eq!(
                file.readiness_counts(),
                failed,
                "terminal failure is not a retry permit"
            );
            assert_eq!(reads.get(), after);
        }
        file.finish().unwrap();
    }
    request.finish();
}

#[test]
fn an_unbound_rule_cannot_schedule_an_independent_provider_after_a_failed_sibling() {
    let engine = engine();
    let mut authored = authored_request(2);
    crate::boundary_fixture::plans(&mut authored)[1].steps[0].operation = Operation::Select {
        from: Input::Relation(Relation::Identifier),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    };
    let program = engine.prepare(authored).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
    let reads = Cell::new(0);
    let read = |relation| {
        assert_eq!(
            relation,
            Relation::Function,
            "undeclared sibling cannot be prepared"
        );
        reads.set(reads.get() + 1);
        Err::<SourceRows<'_>, _>(OperatingError::NotRead)
    };
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let intents = [
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        },
        FileRuleIntent::Undemanded,
    ];
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &intents,
                source: ryo_values::domain::files::types::SourceView(b"abc"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .unwrap();
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols)
            .unwrap()
            .emit,
        RootOutcome::Failed(OperatingError::NotRead)
    ));
    let failed = file.readiness_counts();
    let after = reads.get();
    assert!(matches!(
        file.rule(RuleIndex(1), demand(), &mut symbols),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(file.readiness_counts(), failed);
    assert_eq!(reads.get(), after);
    file.finish().unwrap();
    request.finish();
}
