//! The `replay` service: free functions named by verb that pass the rows of a step on, one by one.

use ryo_language::domain::relations::types::Value;

use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, InputPlace, Memo, OperatorContext, Room};
use crate::domain::table_model::service::{count_of, positions, read_row, row_in};
use crate::domain::table_model::types::{RowCount, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::domain::plans::service::inputs_of;
use ryo_values::domain::plans::types::Operation;

/// What a step that reads nothing reads.
static NOTHING: Table = Table {
    kinds: Vec::new(),
    cells: Vec::new(),
    count: RowCount(0),
};

/// The context of a step: what is shared (the bindings, the names, the bytes) from the context of the step that passed on the
/// row, and the room of its own.
pub(crate) fn derive<'x>(
    context: &'x mut OperatorContext<'_>,
    memo: &'x mut Vec<Memo>,
    scratch: &'x mut OperatorScratch,
) -> OperatorContext<'x> {
    OperatorContext {
        stored_indexes: context.stored_indexes.as_deref_mut(),
        bindings: context.bindings,
        symbols: &mut *context.symbols,
        source: context.source,
        paths: context.paths,
        arenas: context.arenas,
        memo,
        scratch,
    }
}

/// Holds the inputs of the step whole, computes its rows with `compute`, and passes them on.
///
/// # Errors
///
/// Whatever the inputs, `compute` or `emit` refuse.
pub(crate) fn replay_computed(
    operation: &Operation,
    ins: &mut dyn Feed<'_>,
    context: &mut OperatorContext<'_>,
    room: &mut Room,
    emit: &mut dyn Emit,
    compute: fn(
        &Operation,
        &[Rows<'_>],
        &mut OperatorContext<'_>,
        &mut Table,
    ) -> Result<(), OperatingError>,
) -> Result<(), OperatingError> {
    let used = inputs_of(operation).used.0;
    let Room {
        memo,
        scratch,
        held,
        out,
        ..
    } = room;
    let [first, second, third] = held;
    {
        let mut rows: [Rows<'_>; 3] = std::array::from_fn(|_| Rows::Computed(&NOTHING));
        rows[0] = ins.hold(InputPlace(0), context, first)?;
        if used > 1 {
            rows[1] = ins.hold(InputPlace(1), context, second)?;
        }
        if used > 2 {
            rows[2] = ins.hold(InputPlace(2), context, third)?;
        }
        let mut own = OperatorContext {
            stored_indexes: context.stored_indexes.as_deref_mut(),
            bindings: context.bindings,
            symbols: &mut *context.symbols,
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
            memo,
            scratch,
        };
        compute(
            operation,
            rows.get(..used).unwrap_or_default(),
            &mut own,
            out,
        )?;
    }
    let mut own = OperatorContext {
        stored_indexes: context.stored_indexes.as_deref_mut(),
        bindings: context.bindings,
        symbols: &mut *context.symbols,
        source: context.source,
        paths: context.paths,
        arenas: context.arenas,
        memo,
        scratch,
    };
    for index in positions(out.count) {
        if let Some(row) = row_in(out, index) {
            emit.row(&mut own, row)?;
        }
    }
    Ok(())
}

/// Passes on the rows of `rows`, one by one, as they are in the store or in a table.
///
/// # Errors
///
/// Whatever `emit` refuses.
pub(crate) fn scan(
    rows: &Rows<'_>,
    buffer: &mut Vec<Value>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    for position in positions(count_of(rows)) {
        if let Some(row) = read_row(rows, position, buffer) {
            if matches!(rows, Rows::Selected(_))
                && let Some(cache) = context.stored_indexes.as_mut()
            {
                cache.selection_counts.output_cells = cache
                    .selection_counts
                    .output_cells
                    .saturating_add(row.len() as u64);
            }
            emit.row(context, row)?;
        }
    }
    Ok(())
}
