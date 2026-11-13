//! Every invocation workspace of a file run holds the file's relation owner, and finishing the file drops every one of them.
use super::super::super::super::fixtures;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::execution::interface::{
    CaptureIntent, DiagnosticDelivery, ExecutionUseCase as _, FileRuleIntent, ObservationDemand,
    OperatingError, RuleIndex,
};
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::ProgramFileSessionPort as _;
use ryo_execution_runtime::application::file_dag::program::file_session::types::{
    FileInput, SourceLanguage, SourceRows,
};
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput};
use ryo_values::domain::plans::types::Operation;

/// Opens one file of a one-select program, runs its rule `runs` times, and finishes it.
fn finish_after(runs: Runs) -> Result<Value, Failure> {
    let plan = inputs::plan(vec![Operation::Select {
        from: PlanInput::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    }]);
    let (_engine, program) =
        fixtures::requests::prepared(plan, vec![], vec![]).map_err(|error| format!("{error:?}"))?;
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("operator-contract.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = fixtures::requests::execution().execution(&program, &corpus, None.into());
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
        .map_err(|error| format!("{error:?}"))?;
    for _run in 0..runs.0 {
        let _outcome = file.rule(RuleIndex(0), fixtures::requests::emitted(), &mut symbols);
    }
    // Every actual invocation workspace retains this required owner Rc. A surviving workspace would therefore keep this weak owner
    // live.
    let owner = file.owner_lifetime();
    let live_while_open = owner.upgrade().is_some();
    let _finished = file.finish();
    let live_after_finish = owner.upgrade().is_some();
    request.finish();
    Ok(Lifetime {
        live_while_open,
        live_after_finish,
    })
}

/// How many times the file's one rule runs before the file finishes.
#[derive(Clone, Copy, Debug)]
struct Runs(usize);

/// Whether the file's relation owner is live while the file is open, and after it finishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Lifetime {
    live_while_open: bool,
    live_after_finish: bool,
}

type Input = Runs;
type Value = Lifetime;
type Failure = String;

/// The owner lives while the file is open and is dropped with it.
const DROPPED_AT_FINISH: Lifetime = Lifetime {
    live_while_open: true,
    live_after_finish: false,
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        // Every invocation workspace retains the owner, so a surviving workspace would keep it live.
        name: CaseName("finishing_a_file_drops_its_relation_owner"),
        word: Word::Succeeds,
        input: || Runs(1),
        expect: Expect::Value(DROPPED_AT_FINISH),
    },
    Case {
        name: CaseName("finishing_a_file_that_ran_no_rule_drops_its_relation_owner"),
        word: Word::Succeeds,
        input: || Runs(0),
        expect: Expect::Value(DROPPED_AT_FINISH),
    },
];

/// Finishes the file after its rule runs.
fn subject(runs: Input) -> Result<Value, Failure> {
    finish_after(runs)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
