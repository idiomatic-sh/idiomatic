//! Real relation, region, observation and member roots, including direct bound capture.
use super::fixtures::named;
use crate::boundary_fixture::fixtures::{FileRun, corpus, engine, ran, root, run_file, source};
use ryo_execution::inbound::use_cases::execution::fixtures::TableFacts;
use ryo_execution::inbound::use_cases::execution::interface::CaptureIntent;
use ryo_execution::inbound::use_cases::execution::interface::DiagnosticDelivery;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileEnd;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::FrontierPurpose;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramPreparedExecutionPort as _;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution::inbound::use_cases::execution::interface::RootOutcome;
use ryo_execution::inbound::use_cases::execution::interface::RuleAfter;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::{fixtures::node_id, types::*};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::runs::types::ApplicabilityRefusal;
use ryo_values::domain::{
    plan_parts::types::{Input, ObservationBindingId, StepId},
    rows::types::RowCount,
    table_words::fixtures::decode_complete,
};

#[test]
fn direct_bound_root_and_union_share_the_actual_member_before_producer_delivery() {
    {
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
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
        let table = source();
        let mut facts = TableFacts::new(&[(Relation::Function, &table)]);
        let elements = Elements::default();
        let mut symbols = Symbols::default();
        let run = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::Required,
            capture: CaptureIntent::DiagnosticAndObservations,
        };
        let intents = [run, run, FileRuleIntent::Undemanded];
        let mut rules = run_file(
            &request,
            FileRun::of(&intents, &elements),
            &mut symbols,
            &mut facts,
        );
        let expected = vec![Value::Node(node_id(FileId(0), Position(2)))];
        let mut captured = None;
        for index in [0, 1] {
            let rule = ran(&mut rules, index);
            let ryo_execution::inbound::use_cases::execution::interface::EmittedRoot::Delivered(
                RootOutcome::Complete(table),
            ) = rule.emit
            else {
                panic!("complete named root")
            };
            assert_eq!(table.count, RowCount(1));
            assert_eq!(table.cells, expected);
            if index == 1 {
                captured = Some(rule.after);
            }
        }
        assert!(facts.reads() > 0);
        assert!(
            facts
                .history()
                .iter()
                .all(|relation| *relation == Relation::Function),
            "the named roots read Function alone"
        );
        // The original direct bound root is captured after ordinary delivery.
        let Some(RuleAfter::Ran { captures }) = captured else {
            panic!("the consumer runs and captures")
        };
        assert_eq!(captures.len(), 2);
        for (index, capture) in captures.iter().enumerate() {
            assert_eq!(capture.input, InputNumber(index));
            let captured = capture
                .outcome
                .as_ref()
                .expect("bound and exported publications");
            let decoded =
                decode_complete(&captured.words, &mut Vec::new()).expect("valid publication words");
            assert_eq!(decoded.count, RowCount(1));
            assert_eq!(decoded.cells, expected);
        }
        assert!(matches!(rules.end, FileEnd::Finished(_)));
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
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None.into());
    let mut facts = TableFacts::new(&[]);
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
    let mut rules = run_file(
        &request,
        FileRun::of(&intents, &elements),
        &mut symbols,
        &mut facts,
    );
    assert!(
        rules.rules.iter().all(|each| each.rule != RuleIndex(0)),
        "the refused member does not run"
    );
    assert!(matches!(
        root(ran(&mut rules, 1)),
        Ok(RootOutcome::Unknown(
            OperatingError::ObservationIncomplete { .. }
        ))
    ));
    assert_eq!(facts.reads(), 0, "refused member cannot acquire a relation");
    request.finish();
}
