//! Every invocation workspace of a file run holds the file's relation owner, and finishing the file drops every one of them.
use super::super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::{
    CaptureIntent, DiagnosticDelivery, ExecutionUseCase as _, FileRuleIntent, ObservationDemand,
    OperatingError, RuleIndex,
};
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution as _;
use ryo_execution_runtime::application::file_dag::program::file_session::types::{
    FileInput, SourceLanguage, SourceRows,
};
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use ryo_values::domain::plans::types::Operation;

#[test]
fn finishing_a_file_drops_its_relation_owner() {
    let plan = inputs::plan(vec![Operation::Select {
        from: Input::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    }]);
    let (_engine, program) =
        fixtures::requests::prepared(plan, vec![], vec![]).expect("prepare one select");
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("operator-contract.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = fixtures::requests::execution().execution(&program, &corpus, None);
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let read = |_| -> Result<SourceRows<'_>, OperatingError> { Err(OperatingError::NotRead) };
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
                source: ryo_values::domain::files::types::SourceView(b""),
                elements: &elements,
                read: &read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("open the file");
    let _outcome = file.rule(
        RuleIndex(0),
        crate::inbound::use_cases::execution::fixtures::requests::emitted(),
        &mut symbols,
    );
    // Every actual invocation workspace retains this required owner Rc. A surviving workspace would therefore keep this weak owner
    // live.
    let owner = file.owner_lifetime();
    assert!(owner.upgrade().is_some());
    let _finished = file.finish();
    assert!(
        owner.upgrade().is_none(),
        "file finish drops the actual owner and all mandatory workspace references"
    );
    request.finish();
}
