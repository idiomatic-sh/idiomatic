//! File-valued dispatch uses a real hosted schema and both declared corpus entries.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::{EmittedRoot, RuleAfter};
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_language::domain::{
    elements::types::Elements, files::types::FilePath, symbols::types::Symbols,
};
use ryo_relations::domain::relations::types::{CellKind, FileId, Value as Cell};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
        hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
        plan_parts::types::{ColumnIndex, HostedNumber, Input as Source, RepeatedRows, StepId},
        plans::types::Operation,
        row_texts::types::Cell as HostCell,
    },
};
/// What the projection published: its schema, row count and cells.
#[derive(Debug, PartialEq, Eq)]
struct Published {
    kinds: Cow<'static, [CellKind]>,
    rows: usize,
    cells: Cow<'static, [Cell]>,
}
use std::borrow::Cow;
type Input = ();
/// `None` when the root did not complete.
type Value = Option<Published>;
type Failure = OperatingError;
fn subject(_: Input) -> Result<Value, Failure> {
    let names = ["first.py", "second.py"];
    let host = HostRelation {
        name: HostName("files".into()),
        columns: vec![HostColumn {
            name: HostColumnName("file".into()),
            kind: HostKind::File,
        }],
        rows: names
            .map(|name| HostRow(vec![HostCell::File(name.into())]))
            .to_vec(),
    };
    let (_engine, prepared) = fixtures::requests::prepared(
        inputs::plan(vec![
            Operation::Hosted {
                relation: HostedNumber(0),
            },
            Operation::Project {
                from: Source::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            },
        ]),
        vec![],
        vec![host],
    )?;
    let corpus = names.map(|name| {
        CorpusInput::Available(CorpusEntry {
            path: FilePath(name.into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
            executable: ExecutablePermission::Unknown,
        })
    });
    let request = fixtures::requests::execution().execution(&prepared, &corpus, None.into());
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    // Hosted File data never invokes a catalog provider: were it called, the run would fail with the read refusal.
    let mut facts = fixtures::TableFacts::new(&[]);
    let ran = fixtures::requests::run_file(
        &request,
        fixtures::requests::RunFile {
            source: b"",
            elements: &elements,
            intents: &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::DiagnosticAndObservations,
            }],
            semantic: None,
        },
        &mut symbols,
        &mut facts,
    )
    .rules
    .into_iter()
    .next()
    .ok_or(OperatingError::NotRun)?;
    let emitted = match (ran.emit, ran.after) {
        (EmittedRoot::Delivered(outcome), _) => outcome,
        (EmittedRoot::Withheld, RuleAfter::Stopped(stop)) => {
            return Err(fixtures::facts::operating(stop));
        }
        (EmittedRoot::Withheld, RuleAfter::Ran { .. } | RuleAfter::Unreached) => {
            return Err(OperatingError::NotRun);
        }
    };
    let published = if let RootOutcome::Complete(table) = emitted {
        Some(Published {
            kinds: table.kinds.into(),
            rows: table.count.0,
            cells: table.cells.into(),
        })
    } else {
        None
    };
    request.finish();
    Ok(published)
}
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("dispatch_projects_both_distinct_declared_files"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Some(Published {
        kinds: Cow::Borrowed(&[CellKind::File]),
        rows: 2,
        cells: Cow::Borrowed(&[Cell::File(FileId(0)), Cell::File(FileId(1))]),
    })),
}];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
