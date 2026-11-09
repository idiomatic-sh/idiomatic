//! Every operation executes through the real prepare/open_file/collect-or-deliver protocol.
use super::super::super::fixtures as owner_fixtures;
use super::fixtures::{ObservedDelivery, RefusalOutcome, deliver};
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::SemanticLookup;
use ryo_execution_domain::domain::operator::service::prepare_operator;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::*;
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
    stop: Option<usize>,
) -> Result<ObservedDelivery, OperatingError> {
    compare_delivery_with_semantic(
        operation,
        inputs,
        bindings,
        source,
        symbols,
        stop,
        None,
        RefusalOutcome::Failed,
    )
}
pub(crate) fn compare_delivery_with_semantic(
    operation: &Operation,
    inputs: &[Table],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    stop: Option<usize>,
    semantic: Option<&dyn SemanticLookup>,
    refusal: RefusalOutcome,
) -> Result<ObservedDelivery, OperatingError> {
    compare_delivery_with_port(
        operation,
        inputs,
        bindings,
        source,
        symbols,
        stop,
        semantic,
        refusal,
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
    stop: Option<usize>,
    semantic: Option<&dyn SemanticLookup>,
    refusal: RefusalOutcome,
    port: Arc<dyn OperatorsPort>,
) -> Result<ObservedDelivery, OperatingError> {
    let schema = schema(operation, inputs, bindings);
    let authored = author(operation, inputs, bindings, symbols);
    deliver(
        authored,
        schema,
        inputs.len(),
        bindings,
        source,
        symbols,
        stop,
        semantic,
        refusal,
        port,
    )
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
        a.plan.emit.step = StepId((a.plan.steps.len() - 1) as u32);
        a
    } else {
        inputs::table_inputs(operation.clone(), &refs, bindings, symbols)
    }
}

fn all_registrations_refuse_a_foreign_variant() -> Result<(), Failure> {
    for (at, kind) in OperationKind::ALL.iter().enumerate() {
        let foreign = fixture(OperationKind::ALL[(at + 1) % OperationKind::ALL.len()]);
        assert!(matches!(
            prepare_operator(
                *kind,
                &foreign.operation,
                &[],
                &[],
                ryo_values::domain::plan_parts::types::StepScope::PerFile
            ),
            Err(OperatingError::WrongOperation)
        ));
    }
    Ok(())
}
pub(crate) fn repeated(table: &Table, times: usize) -> Table {
    Table {
        kinds: table.kinds.clone(),
        cells: table.cells.repeat(times),
        count: ryo_values::domain::rows::types::RowCount(table.count.0 * times),
    }
}
type Input = fn() -> Result<Value, Failure>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(stringify!(all_registrations_refuse_a_foreign_variant)),
    word: Word::Succeeds,
    input: || all_registrations_refuse_a_foreign_variant,
    expect: Expect::Value(()),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
