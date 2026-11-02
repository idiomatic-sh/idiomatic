//! The `rows` operator: the rows a `list<table>` parameter holds, one for each table, a text column for each key.

use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::terms::types::NameText;

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::minted;
use crate::domain::operating::types::AfterTheFiles;
use crate::domain::operating::types::{Emit, Feed, OperatorContext, Room};
use crate::domain::replay::types::Collect;
use crate::domain::table_model::service::reset;
use crate::domain::table_model::types::{ParameterBinding, Rows, Table};
use ryo_values::domain::plan_parts::service::parameter_in;
use ryo_values::domain::plans::types::Operation;

/// `rows`, one item of the `operators` collection.
pub(crate) struct RowsItem;

/// Passes on a row for each table of the parameter, its cells the texts under the keys, in order, as names.
///
/// # Errors
///
/// `Unbound` for a parameter nothing gave, `WrongBinding` for one that is not a list of tables or a table without a key.
fn tables_rows(
    operation: &Operation,
    context: &mut OperatorContext<'_>,
    row: &mut Vec<Value>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let Operation::Rows { parameter, keys } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    let bindings = context.bindings;
    let tables = match parameter_in(bindings, *parameter).ok_or(OperatingError::Unbound)? {
        ParameterBinding::Tables(tables) => tables.as_slice(),
        // An empty list says no kind of its items: it is no tables.
        ParameterBinding::Texts(items) if items.is_empty() => &[],
        ParameterBinding::Numbers(items) if items.is_empty() => &[],
        ParameterBinding::TextSets(items) if items.is_empty() => &[],
        ParameterBinding::Number(_)
        | ParameterBinding::Flag(_)
        | ParameterBinding::Text(_)
        | ParameterBinding::Numbers(_)
        | ParameterBinding::Texts(_)
        | ParameterBinding::TextSets(_) => return Err(OperatingError::WrongBinding),
    };
    let after = if context.arenas.is_empty() {
        AfterTheFiles::Before
    } else {
        AfterTheFiles::After
    };
    for table in tables {
        row.clear();
        for key in keys {
            let cell = table
                .0
                .iter()
                .find(|cell| *cell.key.0 == *key.0)
                .ok_or(OperatingError::WrongBinding)?;
            let made = query_intern_derived(
                &mut context.symbols,
                ByteSlice(context.source.0),
                NameText(&cell.value.0),
            );
            row.push(minted(context.symbols.as_ref(), made, after));
        }
        emit.row(context, row)?;
    }
    Ok(())
}

impl OperatorsItem for RowsItem {
    fn compute(
        &self,
        operation: &Operation,
        _inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Rows { keys, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        reset(out, &vec![CellKind::Symbol; keys.len()]);
        let mut row = std::mem::take(&mut context.scratch.row);
        let outcome = tables_rows(operation, context, &mut row, &mut Collect(out));
        context.scratch.row = row;
        outcome
    }

    fn stream(
        &self,
        operation: &Operation,
        _ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let mut row = std::mem::take(&mut room.scratch.row);
        let outcome = tables_rows(operation, context, &mut row, emit);
        room.scratch.row = row;
        outcome
    }
}
