//! File-start declarations are enforced through every public execution route.
use super::super::*;
use super::fixtures::input;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ProgramPreparedExecutionPort;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;
use ryo_relations::domain::relations::types::{Words, WorkerNumber};
use ryo_values::domain::runs::types::ApplicabilityRefusal;

#[test]
fn every_prepared_rule_requires_exactly_one_file_start_intent() {
    let engine = engine();
    let program = prepared_rules(&engine, 2);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| panic!("malformed intent vectors cannot read a provider");
    let mut symbols = Symbols::default();
    for intents in [
        &[][..],
        &[FileRuleIntent::Undemanded][..],
        &[FileRuleIntent::Undemanded; 3][..],
    ] {
        assert!(matches!(
            request.open_file(input(intents, &elements, &read), &mut symbols),
            Err(OperatingError::ShapeMismatch)
        ));
    }
    request.finish();
}

#[test]
fn absent_skipped_and_refused_rules_cannot_acquire_any_root() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| panic!("undeclared roots cannot read a provider");
    for intent in [
        FileRuleIntent::Undemanded,
        FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected),
        FileRuleIntent::Refused(ApplicabilityRefusal::SyntaxUndetermined),
        FileRuleIntent::Refused(ApplicabilityRefusal::MetadataUnavailable),
    ] {
        let intents = [intent];
        let mut symbols = Symbols::default();
        let mut file = request
            .open_file(input(&intents, &elements, &read), &mut symbols)
            .unwrap();
        assert!(matches!(
            file.rule(RuleIndex(0), demand(), &mut symbols),
            Err(OperatingError::NotRun)
        ));
        assert!(matches!(
            file.emit_rows(RuleIndex(0), &mut symbols, &mut |_| panic!("no delivery")),
            Err(OperatingError::NotRun)
        ));
        let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
        assert!(matches!(
            file.capture_frontiers(
                RuleIndex(0),
                ryo_values::domain::stored_facts::types::Bucket(0),
                &mut symbols,
                CaptureContext {
                    worker: WorkerNumber(0),
                    arena: ryo_execution::inbound::use_cases::execution::interface::Arena(
                        &mut arena
                    ),
                    places: &mut places,
                    words: &mut words
                },
            ),
            Err(OperatingError::NotRun)
        ));
        assert!(matches!(
            file.commit_frontier(
                RuleIndex(0),
                ryo_values::domain::plan_parts::types::InputNumber(0)
            ),
            Err(OperatingError::NotRun)
        ));
        file.finish().unwrap();
    }
    request.finish();
}

#[test]
fn rule_demand_is_immutable_and_capture_cannot_be_added_after_open() {
    let engine = engine();
    let program = prepared(&engine);
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&program, &corpus, None);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let reads = Cell::new(0);
    let table = source();
    let read = |relation| {
        assert_eq!(relation, Relation::Function);
        reads.set(reads.get() + 1);
        Ok(SourceRows::Computed(&table))
    };
    let intents = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }];
    let mut symbols = Symbols::default();
    let mut file = request
        .open_file(input(&intents, &elements, &read), &mut symbols)
        .unwrap();
    for changed in [
        RuleDemand {
            emit: EmitDemand::Disabled,
            observations: ObservationDemand::NotRequired,
        },
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::Required,
        },
    ] {
        assert!(matches!(
            file.rule(RuleIndex(0), changed, &mut symbols),
            Err(OperatingError::NotRun)
        ));
    }
    let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
    assert!(matches!(
        file.capture_frontiers(
            RuleIndex(0),
            ryo_values::domain::stored_facts::types::Bucket(0),
            &mut symbols,
            CaptureContext {
                worker: WorkerNumber(0),
                arena: ryo_execution::inbound::use_cases::execution::interface::Arena(&mut arena),
                places: &mut places,
                words: &mut words
            },
        ),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(reads.get(), 0, "refused root changes do not admit a source");
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols)
            .unwrap()
            .emit,
        RootOutcome::Complete(_)
    ));
    assert!(reads.get() > 0);
    file.finish().unwrap();
    request.finish();
}
