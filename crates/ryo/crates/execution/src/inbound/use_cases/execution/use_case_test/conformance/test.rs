//! Every operation executes through the real prepare and the one per-file route, collected or forwarded.
use super::super::super::fixtures as owner_fixtures;
use super::fixtures::{ObservedDelivery, Settlement, deliver};
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::SemanticLookup;
use ryo_execution_domain::domain::operator::service::prepare_operator;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::*;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::rows::types::Table;
use std::borrow::Cow;
use std::sync::Arc;

pub(crate) fn same_rows(table: &Table, rows: &[Vec<Cell>]) -> bool {
    table.count.0 == rows.len()
        && rows.iter().all(|row| row.len() == table.kinds.len())
        && table.cells == rows.iter().flatten().copied().collect::<Vec<_>>()
}
pub(crate) fn compare_delivery(
    operation: &Operation,
    inputs: &[Table],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
) -> Result<ObservedDelivery, OperatingError> {
    compare_delivery_with_semantic(operation, inputs, bindings, source, symbols, None)
}
pub(crate) fn compare_delivery_with_semantic(
    operation: &Operation,
    inputs: &[Table],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    semantic: Option<&dyn SemanticLookup>,
) -> Result<ObservedDelivery, OperatingError> {
    compare_delivery_with_port(
        operation,
        inputs,
        bindings,
        source,
        symbols,
        semantic,
        Arc::new(owner_fixtures::operators()),
    )
}
/// This helper only drives supported owner APIs. It cannot invoke a primitive,
/// construct a permit, choose an execution strategy or supply an alternate executor.
pub(crate) fn compare_delivery_with_port(
    operation: &Operation,
    inputs: &[Table],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    semantic: Option<&dyn SemanticLookup>,
    port: Arc<dyn OperatorsPort>,
) -> Result<ObservedDelivery, OperatingError> {
    let schema = schema(operation, inputs, bindings);
    let authored = author(operation, inputs, bindings, symbols);
    deliver(authored, schema, bindings, source, symbols, semantic, port)
}

/// A delivery whose root completed or failed; an unknown or unsettled root is refused with its error.
pub(crate) fn settled(observed: ObservedDelivery) -> Result<ObservedDelivery, OperatingError> {
    match observed.settlement {
        Settlement::Complete | Settlement::Failed => Ok(observed),
        Settlement::Unknown | Settlement::Unsettled => {
            Err(observed.collected.err().unwrap_or(OperatingError::NotRun))
        }
    }
}

/// The operation's output schema over these input tables.
pub(crate) fn schema(
    operation: &Operation,
    inputs: &[Table],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
) -> Result<Vec<CellKind>, OperatingError> {
    ryo_execution_domain::domain::operation_schema::service::schema_of(
        operation,
        &inputs
            .iter()
            .map(
                |table| ryo_execution_domain::domain::operation_schema::types::InputShape {
                    kinds: &table.kinds,
                    tree: None,
                },
            )
            .collect::<Vec<_>>(),
        bindings,
    )
}

/// The authored plan of an operation over these input tables.
pub(crate) fn author(
    operation: &Operation,
    inputs: &[Table],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    symbols: &mut Symbols,
) -> inputs::AuthoredRows {
    let refs = inputs.iter().collect::<Vec<_>>();
    if matches!(operation, Operation::SpanBetween { .. }) {
        use ryo_values::domain::plan_parts::types::{Input, JoinColumn, Side, StepId};
        let left = Table {
            kinds: vec![CellKind::Span],
            cells: vec![inputs[0].cells[0]],
            count: ryo_values::domain::rows::types::RowCount(1),
        };
        let right = Table {
            kinds: vec![CellKind::Span],
            cells: vec![inputs[0].cells[1]],
            count: ryo_values::domain::rows::types::RowCount(1),
        };
        let join = Operation::Join {
            left: Input::Relation(
                ryo_relations::domain::relations::types::Relation::SourceLocation,
            ),
            right: Input::Relation(
                ryo_relations::domain::relations::types::Relation::StringLiteral,
            ),
            left_on: vec![],
            right_on: vec![],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(0),
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        };
        let mut a = inputs::table_inputs(join, &[&left, &right], bindings, symbols);
        let mut span = operation.clone();
        ryo_values::domain::plans::service::each_input(&mut span, &mut |input| {
            *input = Input::Step(a.plan.emit.step)
        });
        a.plan
            .steps
            .push(ryo_values::domain::plans::types::PlanStep {
                scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
                operation: span,
            });
        a.plan.emit.step =
            StepId(u32::try_from(a.plan.steps.len() - 1).expect("a step number fits in u32"));
        a
    } else {
        inputs::table_inputs(operation.clone(), &refs, bindings, symbols)
    }
}

/// Each registered kind's preparation of the next kind's fixture operation: the refusal, or `None` when it prepared.
fn all_registrations_refuse_a_foreign_variant() -> Result<Value, Failure> {
    Ok(OperationKind::ALL
        .iter()
        .enumerate()
        .map(|(at, kind)| {
            let foreign = fixture(OperationKind::ALL[(at + 1) % OperationKind::ALL.len()]);
            let refused = prepare_operator(
                *kind,
                &foreign.operation,
                &[],
                &[],
                ryo_values::domain::plan_parts::types::StepScope::PerFile,
            )
            .err();
            (*kind, refused)
        })
        .collect())
}
pub(crate) fn repeated(table: &Table, times: usize) -> Table {
    Table {
        kinds: table.kinds.clone(),
        cells: table.cells.repeat(times),
        count: ryo_values::domain::rows::types::RowCount(table.count.0 * times),
    }
}
type Input = fn() -> Result<Value, Failure>;
type Value = Cow<'static, [(OperationKind, Option<OperatingError>)]>;
type Failure = OperatingError;
const FOREIGN: &[(OperationKind, Option<OperatingError>)] = {
    use OperationKind::*;
    const W: Option<OperatingError> = Some(OperatingError::WrongOperation);
    &[
        (Replace, W),
        (OrderedMatch, W),
        (Select, W),
        (Project, W),
        (Extend, W),
        (Join, W),
        (Union, W),
        (Intersection, W),
        (Difference, W),
        (Antijoin, W),
        (GroupCount, W),
        (GroupDigest, W),
        (Capture, W),
        (Rank, W),
        (Words, W),
        (Axis, W),
        (Closure, W),
        (Cycles, W),
        (Rows, W),
        (Hosted, W),
        (SpanBetween, W),
        (SourceMatches, W),
        (RenderSyntax, W),
        (FitsRendered, W),
        (SpanBounds, W),
        (OrderedDfs, W),
    ]
};
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(stringify!(all_registrations_refuse_a_foreign_variant)),
    word: Word::Succeeds,
    input: || all_registrations_refuse_a_foreign_variant,
    expect: Expect::Value(Cow::Borrowed(FOREIGN)),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
