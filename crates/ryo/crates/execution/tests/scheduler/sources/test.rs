//! Real relation, region, observation and member roots, including direct bound capture.
use super::fixtures::named;
use crate::boundary_fixture::fixtures::{corpus, engine, source};
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionRequest as _;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileExecution as _;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::PreparedExecution as _;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::{fixtures::node_id, types::*};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::runs::types::ApplicabilityRefusal;
use ryo_values::domain::{
    plan_parts::types::{Input, ObservationBindingId, StepId},
    rows::types::RowCount,
    stored_facts::types::Bucket,
    table_words::fixtures::decode_complete,
};
use std::cell::Cell;

#[test]
fn direct_bound_root_and_union_share_the_actual_member_before_producer_delivery() {
    for consumer_first in [false, true] {
        let engine = engine();
        let program = engine
            .prepare(named())
            .expect("named producer, alias and union");
        assert_eq!(
            program.frontiers(RuleIndex(1)).unwrap(),
            &[
                Input::BoundObservation(ObservationBindingId(0)),
                Input::Step(StepId(0))
            ]
        );
        assert_eq!(
            program
                .frontier_purpose(RuleIndex(1), InputNumber(0))
                .unwrap(),
            FrontierPurpose::Observation
        );
        assert_eq!(
            program
                .frontier_purpose(RuleIndex(1), InputNumber(1))
                .unwrap(),
            FrontierPurpose::Observation
        );
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
        let table = source();
        let reads = Cell::new(0);
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            reads.set(reads.get() + 1);
            Ok(SourceRows::Computed(&table))
        };
        let elements = Elements::default();
        let mut symbols = Symbols::default();
        let run = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::Required,
            capture: CaptureIntent::DiagnosticAndObservations,
        };
        let intents = [run, run, FileRuleIntent::Undemanded];
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
        assert_eq!(reads.get(), 0, "binding roots must remain lazy");
        let roots = RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::Required,
        };
        let order = if consumer_first { [1, 0] } else { [0, 1] };
        let expected = vec![Value::Node(node_id(FileId(0), Position(2)))];
        for index in order {
            let output = file.rule(RuleIndex(index), roots, &mut symbols).unwrap();
            let RootOutcome::Complete(table) = output.emit else {
                panic!("complete named root")
            };
            assert_eq!(table.count, RowCount(1));
            assert_eq!(table.cells, expected);
            assert_eq!(output.observations.len(), 1);
            let RootOutcome::Complete(observed) = &output.observations[0].outcome else {
                panic!("complete named union observation")
            };
            assert_eq!(observed.cells, expected);
        }
        let read_count = reads.get();
        assert!(read_count > 0);
        // The original direct bound root is captured after ordinary delivery.
        let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
        let captures = file
            .capture_frontiers(
                RuleIndex(1),
                Bucket(0),
                &mut symbols,
                CaptureContext {
                    worker: WorkerNumber(0),
                    arena: ryo_execution::inbound::use_cases::execution::interface::Arena(
                        &mut arena,
                    ),
                    places: &mut places,
                    words: &mut words,
                },
            )
            .unwrap();
        assert_eq!(captures.len(), 2);
        for (index, capture) in captures.iter().enumerate() {
            assert_eq!(
                capture.input,
                ryo_values::domain::plan_parts::types::InputNumber(index)
            );
            let captured = capture
                .outcome
                .as_ref()
                .expect("bound and exported publications");
            let decoded =
                decode_complete(&captured.words, &mut Vec::new()).expect("valid publication words");
            assert_eq!(decoded.count, RowCount(1));
            assert_eq!(decoded.cells, expected);
            file.commit_frontier(RuleIndex(1), capture.input).unwrap();
        }
        assert_eq!(
            reads.get(),
            read_count,
            "alias capture cannot rerun its provider"
        );
        file.finish().unwrap();
        request.finish();
    }
}

#[test]
fn a_refused_named_member_cannot_be_replaced_with_a_typed_empty_or_provider_retry() {
    let engine = engine();
    let program = engine.prepare(named()).unwrap();
    assert_eq!(
        program.frontiers(RuleIndex(1)).unwrap(),
        &[
            Input::BoundObservation(ObservationBindingId(0)),
            Input::Step(StepId(0))
        ]
    );
    assert_eq!(
        program
            .frontier_purpose(RuleIndex(1), InputNumber(0))
            .unwrap(),
        FrontierPurpose::Observation
    );
    assert_eq!(
        program
            .frontier_purpose(RuleIndex(1), InputNumber(1))
            .unwrap(),
        FrontierPurpose::Observation
    );
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let read = |_| panic!("refused member cannot acquire a relation");
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let intents = [
        FileRuleIntent::Refused(ApplicabilityRefusal::SyntaxUndetermined),
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::Required,
            capture: CaptureIntent::DiagnosticAndObservations,
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
    let roots = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::Required,
    };
    let first = file.rule(RuleIndex(1), roots, &mut symbols).unwrap();
    assert!(matches!(
        first.emit,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(first.observations.len(), 1);
    assert!(matches!(
        first.observations[0].outcome,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { .. })
    ));
    let second = file.rule(RuleIndex(1), roots, &mut symbols).unwrap();
    assert!(matches!(
        second.emit,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(second.observations.len(), 1);
    assert!(matches!(
        second.observations[0].outcome,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { .. })
    ));
    assert!(matches!(
        file.rule(RuleIndex(0), roots, &mut symbols),
        Err(OperatingError::NotRun)
    ));
    file.finish().unwrap();
    request.finish();
}
