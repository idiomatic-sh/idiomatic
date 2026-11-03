//! Compile and run only a complete selected-left/stored-right join, total equality
//! filter, and terminal distinct projection. Every other shape uses the executor.

use super::types::{Chain, KernelPlan};
use crate::domain::lowering::types::Counting;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{
    types::{PrimitiveSemantics, join_result},
    views::{BorrowedRows, IndexedRows},
};
use crate::domain::operator::types::PlanLayout;

use crate::domain::table_model::service::{cell_at, column_in, reset};
use ryo_operators::domain::contract::types::{JoinInput, JoinRow, OperatorCall};
mod driver;
use crate::domain::table_model::types::Rows;
use ryo_derived::domain::contract::types::KeyIndex;
use ryo_language::domain::symbols::service::{hash_in, hash_of, text_of};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, Value};
use ryo_values::domain::plan_parts::types::{
    Comparison, Input, JoinColumn, Operand, QueryLiteral, Side, Test,
};
use ryo_values::domain::plans::types::{Operation, Plan, StepScope};

/// A typed equality against authored text has no row-dependent error. Nonempty
/// text also cannot match a malformed Symbol that the ordinary evaluator reads
/// as empty. Other comparisons retain their original evaluation and error order.
fn total_symbol_filter(
    comparisons: &[Comparison],
) -> Option<(
    ryo_values::domain::plan_parts::types::ColumnIndex,
    ryo_values::domain::plan_parts::types::LiteralText,
)> {
    let [comparison] = comparisons else {
        return None;
    };
    if comparison.test != Test::Equal {
        return None;
    }
    let (column, text) = match (&comparison.left, &comparison.right) {
        (Operand::Column(column), Operand::Literal(QueryLiteral::Text(text)))
        | (Operand::Literal(QueryLiteral::Text(text)), Operand::Column(column)) => (column, text),
        _ => return None,
    };
    (!text.0.is_empty()).then(|| (*column, text.clone()))
}

/// Prepare immutable chains under the same plan/layout identity as the executor.
#[must_use]
pub(crate) fn compile(plan: &Plan, layout: &PlanLayout) -> KernelPlan {
    let mut chains = vec![None; plan.steps.len()];
    for (target, step) in plan.steps.iter().enumerate() {
        if step.scope != StepScope::PerFile || layout.kept.0.get(target) != Some(&true) {
            continue;
        }
        let Operation::Project {
            from: Input::Step(filter_step),
            columns,
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
        } = &step.operation
        else {
            continue;
        };
        let filter_id = filter_step.0 as usize;
        let Some(filter_step) = plan.steps.get(filter_id) else {
            continue;
        };
        let Operation::Select {
            from: Input::Step(join_step),
            comparisons,
            kept,
        } = &filter_step.operation
        else {
            continue;
        };
        if !kept.is_empty() {
            continue;
        }
        let Some((filter_column, literal)) = total_symbol_filter(comparisons) else {
            continue;
        };
        let join_id = join_step.0 as usize;
        let Some(join_step) = plan.steps.get(join_id) else {
            continue;
        };
        let Operation::Join {
            left: Input::Step(left_step),
            right: Input::Relation(right),
            left_on,
            right_on,
            output,
            filter,
            view,
            reach: None,
        } = &join_step.operation
        else {
            continue;
        };
        let left_id = left_step.0 as usize;
        // A shared left cut is read as it was made; neither child of this chain
        // may be independently kept or handed to a frontier.
        if layout.kept.0.get(left_id) != Some(&true)
            || layout.kept.0.get(filter_id) != Some(&false)
            || layout.kept.0.get(join_id) != Some(&false)
            || !filter.is_empty()
            || !view.is_empty()
            || left_on.is_empty()
            || left_on.len() != right_on.len()
            || [filter_id, join_id].iter().any(|&id| {
                plan.steps
                    .get(id)
                    .is_none_or(|step| step.scope != StepScope::PerFile)
            })
            || [target, filter_id, join_id].iter().any(|&id| {
                layout.lowering.shapes.get(id).is_some_and(Option::is_some)
                    || layout.lowering.counting.get(id) == Some(&Counting::Counts)
            })
        {
            continue;
        }
        let (Some(left_kinds), Some(join_kinds)) =
            (layout.kinds.get(left_id), layout.kinds.get(join_id))
        else {
            continue;
        };
        let right_kinds = kinds(*right);
        if left_on.iter().zip(right_on).any(|(left, right)| {
            column_in(left_kinds, *left)
                .zip(column_in(right_kinds, *right))
                .is_none_or(|(left, right)| left != right)
        }) || output.iter().any(|column| {
            let source: &[CellKind] = if column.side == Side::Left {
                left_kinds.as_slice()
            } else {
                right_kinds
            };
            column_in(source, column.column).is_none()
        }) || column_in(join_kinds, filter_column) != Some(&CellKind::Symbol)
        {
            continue;
        }
        let Some(&filter_source) = column_in(output, filter_column) else {
            continue;
        };
        let Some(projected) = columns
            .iter()
            .map(|column| column_in(output, *column).copied())
            .collect::<Option<Vec<JoinColumn>>>()
        else {
            continue;
        };
        chains[target] = Some(Chain {
            left_step: left_id,
            right: *right,
            left_on: left_on.clone(),
            right_on: right_on.clone(),
            filter: filter_source,
            literal,
            projected,
            index: None,
        });
    }
    KernelPlan(chains)
}

/// Resolve one physical cell, counting only actual explicit decodes.
fn cell(
    left: &Rows<'_>,
    right: &Rows<'_>,
    left_row: RowIndex,
    right_row: RowIndex,
    source: JoinColumn,
    decoded: &std::cell::Cell<u64>,
) -> Option<Value> {
    let value = match source.side {
        Side::Left => cell_at(left, left_row, source.column),
        Side::Right => cell_at(right, right_row, source.column),
    }?;
    decoded.set(decoded.get().saturating_add(1));
    Some(value)
}

/// Match the ordinary literal-Text fast path, including exact hash/text checks.
fn symbol_holds(value: Value, literal: &str, symbols: &Symbols, source: ByteSlice<'_>) -> bool {
    let Value::Symbol(symbol) = value else {
        return false;
    };
    let found = text_of(symbols, source, symbol);
    match hash_in(symbols, symbol) {
        Some(hash) => {
            hash == hash_of(ByteSlice(literal.as_bytes()))
                && found.is_some_and(|text| text.0 == literal)
        }
        // The ordinary evaluator falls through to its Datum text path when
        // a Symbol has no hash. Keep that behavior for malformed registrations.
        None => found.is_some_and(|text| text.0 == literal),
    }
}

/// A row-ID cursor is the whole intermediate representation: no joined pair
/// list or materialized intermediate table survives a candidate iteration.
pub(crate) fn execute_with_index(
    semantics: &dyn PrimitiveSemantics,
    chain: &Chain,
    left: &Rows<'_>,
    right: &Rows<'_>,
    right_source: ColumnsView<'_>,
    index: &KeyIndex,
    symbols: &Symbols,
    source: ByteSlice<'_>,
    target: &mut Table,
    kinds: &[CellKind],
) -> Result<Option<(u64, u64, u64)>, OperatingError> {
    let left_count = match left {
        Rows::Selected(view) => view.selection.rows.len(),
        _ => return Ok(None),
    };
    reset(target, kinds);
    let mut key = Vec::with_capacity(chain.left_on.len());
    let mut projected = Vec::with_capacity(chain.projected.len());
    let decoded = std::cell::Cell::new(0_u64);
    let hits = std::cell::Cell::new(0_u64);
    let emitted = std::cell::Cell::new(0_u64);
    let indexed = IndexedRows {
        #[cfg(feature = "diagnostic-row-progress")]
        progress: semantics.progress(),
        rows: BorrowedRows::Stored(right_source),
        columns: &chain.right_on,
        index,
    };
    for position in 0..left_count {
        #[cfg(feature = "diagnostic-row-progress")]
        (semantics.progress())(
            crate::domain::operator::contract::views::ProgressCounter::InputRows,
            1,
        );
        let left_row = RowIndex(position);
        let reader = driver::Left {
            rows: left,
            row: left_row,
            decoded: &decoded,
        };
        let mut driver = driver::Driver {
            chain,
            left,
            left_row,
            right,
            index: indexed,
            probe: None,
            current: None,
            symbols,
            source,
            target,
            decoded: &decoded,
            hits: &hits,
            emitted: &emitted,
            error: None,
        };
        let answer = semantics.operators(OperatorCall::EqualityJoin {
            input: JoinInput::Values {
                left: JoinRow::Columns(&reader),
                left_on: &chain.left_on,
                output: &chain.projected,
                view: &[],
                driver: &mut driver,
                key: &mut key,
                target: &mut projected,
            },
        });
        join_result(answer, driver.error.take())?;
    }
    Ok(Some((emitted.get(), decoded.get(), hits.get())))
}

use ryo_values::domain::rows::types::{RowIndex, Table};
