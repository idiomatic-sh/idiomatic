//! A skipped rule's file never starts the shared walk.
use crate::boundary_fixture::fixtures::{authored_request, corpus, demand, engine};
use ryo_execution::inbound::use_cases::execution::interface::ApplicabilityReason;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use ryo_execution::inbound::use_cases::execution::interface::FileInput;
use ryo_execution::inbound::use_cases::execution::interface::FileRuleIntent;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::ProgramFileSessionPort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramProjectPhasePort;
use ryo_execution::inbound::use_cases::execution::interface::ProgramRequestExecutionPort;
use ryo_execution::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution::inbound::use_cases::execution::interface::SourceLanguage;
use ryo_execution::inbound::use_cases::execution::interface::SourceRows;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;

#[test]
fn a_skipped_rule_is_refused_without_a_walk_pass() {
    let engine = engine();
    let prepared = engine.prepare(authored_request(1)).unwrap();
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
    let mut symbols = Symbols::default();
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| -> Result<SourceRows<'_>, OperatingError> {
        panic!("rejected syntax has no semantic rows")
    };
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &[FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected)],
                source: ryo_values::domain::files::types::SourceView(b"def broken(:"),
                elements: &elements,
                read: &read,
                semantic: ryo_execution::inbound::use_cases::execution::interface::SemanticSupport::Absent,
                kinds: ryo_execution::inbound::use_cases::execution::interface::KindSupport::Absent,
            },
            &mut symbols,
        )
        .unwrap();
    assert!(matches!(
        file.rule(RuleIndex(0), demand(), &mut symbols),
        Err(OperatingError::NotRun)
    ));
    assert_eq!(file.fixture_walk_counts().passes.0, 0);
    file.finish().unwrap();
    request.finish();
}
