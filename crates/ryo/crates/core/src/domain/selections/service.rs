//! The `selections` service: the lowered steps that keep rows, with no IO and no ports.

use ryo_language::domain::symbols::service::name_count;
use ryo_relations::domain::relations::types::Value;

use super::evaluation::tree_each;
use super::types::ProjectedColumns;
use crate::domain::comparing::service::holds_all;
use crate::domain::lowering::service::{kind_of, shape_at};
use crate::domain::lowering::types::Lowering;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Memo, NameCount, OperatorContext};
use crate::domain::positions::service::{count, key_into, made_of, row, values};
use crate::domain::positions::types::{LoweredRoom, Slots, Stride, Work};
use crate::domain::table_cells::types::TreeRows;
use crate::domain::table_model::service::positions;
use crate::domain::table_model::types::Verdict;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::{Comparison, StepId};

/// The elements of the tree the comparisons admit.
///
/// # Errors
///
/// What a comparison refuses.
pub(crate) fn select_tree(
    tree: &TreeRows<'_>,
    comparisons: &[Comparison],
    context: &mut OperatorContext<'_>,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let out = &mut out.0;
    let mut buffer = std::mem::take(&mut context.scratch.row);
    let outcome = tree_each(
        comparisons,
        tree,
        &mut buffer,
        context,
        &mut |index: RowIndex, _buffer: &mut Vec<Value>, _context: &mut OperatorContext<'_>| {
            out.push(u32::try_from(index.0).unwrap_or(u32::MAX));
            Ok(())
        },
    );
    context.scratch.row = buffer;
    outcome
}

/// The rows of the step `input` the comparisons admit.
///
/// # Errors
///
/// What a comparison refuses.
pub(crate) fn select_rows(
    tree: &TreeRows<'_>,
    lowering: &Lowering,
    input: StepId,
    comparisons: &[Comparison],
    work: Work<'_, '_>,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let out = &mut out.0;
    let Work { room, context, .. } = work;
    let shape = shape_at(lowering, input)?;
    let kinds: Vec<CellKind> = shape
        .columns
        .iter()
        .map(|each| kind_of(each.field))
        .collect();
    let names = NameCount(name_count(context.symbols.as_ref()).0);
    context.memo.clear();
    context
        .memo
        .resize(comparisons.len() * names.0, Memo::Unknown);
    let LoweredRoom { made, cells, .. } = room;
    let rows = made_of(made, input).ok_or(OperatingError::NotRun)?;
    for at in positions(count(rows)) {
        let each = row(rows, at);
        values(tree, &shape.columns, each, cells);
        if holds_all(comparisons, cells, &kinds, names, context)? == Verdict::Holds {
            out.extend_from_slice(each.0);
        }
    }
    Ok(())
}

/// The rows of the step `input`, once each by the columns a projection keeps when it drops repeated rows.
pub(crate) fn project(
    derived: &mut dyn crate::domain::operator::contract::derived::ScopedDerived,
    tree: &TreeRows<'_>,
    lowering: &Lowering,
    input: StepId,
    projection: ProjectedColumns<'_>,
    room: &mut LoweredRoom,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let out = &mut out.0;
    let LoweredRoom {
        made, keys, keep, ..
    } = room;
    let Some(rows) = made_of(made, input) else {
        return Ok(());
    };
    if projection.repeats == RepeatedRows::Kept {
        out.extend_from_slice(&rows.slots.0);
        return Ok(());
    }
    let Ok(shape) = shape_at(lowering, input) else {
        return Ok(());
    };
    keys.0.clear();
    for at in positions(count(rows)) {
        key_into(
            tree,
            &shape.columns,
            row(rows, at),
            projection.columns,
            keys,
        );
    }
    derived.with_raw_order(&keys.0, projection.columns.len().max(1), &mut |order| {
        order.firsts(&mut keep.0);
        for (at, kept) in keep.0.iter().enumerate() {
            if *kept {
                out.extend_from_slice(row(rows, RowIndex(at)).0);
            }
        }
        Ok(())
    })
}

use ryo_values::domain::rows::types::RowIndex;
