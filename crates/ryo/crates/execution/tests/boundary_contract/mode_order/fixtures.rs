//! The support of the `mode_order` cases and of the groups nested in it.
use super::super::*;
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::Direction,
    row_texts::types::Cell as HostCell,
    rows::errors::RowsError,
};

/// The root rule 0 of `input` delivered in file 0 run through `run_file` with its diagnostic collected over `facts`, or what
/// stopped it.
pub(super) fn evaluate(
    input: PrepareRequest,
    facts: &mut TableFacts,
) -> Result<RootOutcome, FileStop> {
    let engine = engine();
    let prepared = engine
        .prepare(input)
        .expect("prepare actual owner policy case");
    let corpus = corpus();
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus, None.into());
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
        facts,
    );
    let outcome = root(ran(&mut rules, 0));
    request.finish();
    outcome
}

pub(super) fn step(operation: Operation) -> PlanStep {
    PlanStep {
        scope: StepScope::PerFile,
        operation,
    }
}

pub(super) fn select(relation: Relation) -> Operation {
    Operation::Select {
        from: Input::Relation(relation),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    }
}

pub(super) fn hosted(values: &[i64]) -> HostRelation {
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

pub(super) fn axis(from: Input, domain: Input, parent: u32) -> Operation {
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

/// The column a root failed for want of; `None` when it did not fail on a missing column.
pub(super) fn failed_column(outcome: Result<RootOutcome, FileStop>) -> Option<u32> {
    if let Ok(RootOutcome::Failed(OperatingError::Table(RowsError::NoSuchColumn { index }))) =
        outcome
    {
        Some(index)
    } else {
        None
    }
}
