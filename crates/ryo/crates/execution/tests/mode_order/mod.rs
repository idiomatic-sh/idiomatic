//! Declared schema/resource and transport ordering through the real public owner.
use ryo_execution::inbound::use_cases::execution::interface::RowEmission;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::types::{PlanSource, PrepareRequest};
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase;
use super::*;
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{Direction, HostedNumber},
    row_texts::types::Cell as HostCell,
    rows::errors::RowsError,
};

fn evaluate<'facts>(
    input: PrepareRequest,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> RootOutcome {
    let engine = engine();
    let prepared = engine
        .prepare(input)
        .expect("prepare actual owner policy case");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
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
                read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("open actual owner policy file");
    let outcome = file
        .rule(RuleIndex(0), demand(), &mut symbols)
        .expect("obtain typed root outcome")
        .emit;
    file.finish().expect("release actual policy file");
    request.finish();
    outcome
}
fn step(operation: Operation) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation,
    }
}
fn select(relation: Relation) -> Operation {
    Operation::Select {
        from: Input::Relation(relation),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    }
}
fn hosted(values: &[i64]) -> HostRelation {
    HostRelation {
        name: HostName("numbers".into()),
        columns: vec![HostColumn {
            name: HostColumnName("number".into()),
            kind: HostKind::Number,
        }],
        rows: values
            .iter()
            .map(|value| HostRow(vec![HostCell::Number(Integer(*value))]))
            .collect(),
    }
}
fn axis(from: Input, domain: Input, parent: u32) -> Operation {
    Operation::Axis {
        from,
        edges: Input::Relation(Relation::SemanticChild),
        domain,
        direction: Direction::Child,
        id: ColumnIndex(0),
        parent: ColumnIndex(parent),
        child: ColumnIndex(1),
        domain_id: ColumnIndex(99),
        domain_filter: vec![],
        domain_view: vec![],
        output: vec![],
    }
}
fn assert_column(outcome: RootOutcome, index: u32) {
    assert!(
        matches!(outcome, RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn { index: actual })) if actual == index)
    );
}

struct Forwarded {
    result: Result<RowEmission, OperatingError>,
    rows: Vec<Vec<Value>>,
}
fn deliver<'facts>(
    input: PrepareRequest,
    read: &dyn Fn(Relation) -> Result<SourceRows<'facts>, OperatingError>,
) -> Forwarded {
    let engine = engine();
    let prepared = engine
        .prepare(input)
        .expect("prepare actual forward policy case");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
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
                read,
                semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
            },
            &mut symbols,
        )
        .expect("open actual forward policy file");
    let mut rows = vec![];
    let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
        rows.push(row.to_vec());
        Ok(())
    });
    file.finish().expect("release actual forward file");
    request.finish();
    Forwarded { result, rows }
}

mod axis;
mod collected;
