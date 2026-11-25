//! The `scan_bound` item of the `kernels` collection: rows the engine holds for the run after the files (a frontier entry, a
//! bound observation, the host's rows) and the rows a `list<table>` parameter's bound value holds; a withheld input is the
//! `Withheld` fault with the engine's token, never empty rows.

use crate::domain::contract::errors::ContractError;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::KernelRun;
use crate::domain::physical_plan::types::held_rows::value::HeldRows;
use crate::domain::physical_plan::types::lowering::value::BoundScan;
use crate::domain::table_rows::types::{RowTexts, TableRowOutput};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::written_rows::types::RowsSource;



/// The rows held, or the fault naming the engine's token.
///
/// # Errors
///
/// `Withheld` with the engine's token when the engine withheld the input.
pub(crate) fn held(rows: HeldRows<'_>) -> Result<&Table, PortableOperatorFault> {
    rows.table().map_err(PortableOperatorFault::Withheld)
}

/// The rows of a bound scan: the host's rows of a relation as the engine holds them, or one row for each table of a
/// `list<table>` parameter, the text under each key in key order, or one row for each row the query wrote; each text made a
/// name in the run's symbols.
///
/// # Errors
///
/// `Withheld` for host rows the engine withheld; `Unbound`, `WrongBinding` or `UnboundKey` for a parameter bound to no table
/// list or a table lacking a key.
pub(crate) fn scanned(
    scan: &BoundScan,
    run: &mut KernelRun<'_>,
) -> Result<Table, PortableOperatorFault> {
    match scan {
        BoundScan::Hosted(relation) => held(run.inputs.hosted(*relation)).cloned(),
        BoundScan::Rows {
            source,
            keys,
            bindings,
        } => {
            let mut output = Table {
                kinds: vec![CellKind::Symbol; keys.len()],
                ..Table::default()
            };
            let mut into = RowsInto {
                run,
                output: &mut output,
                row: Vec::with_capacity(keys.len()),
            };
            match source {
                RowsSource::Parameter(parameter) => {
                    let tables = bound_tables(bindings, *parameter)?;
                    crate::domain::table_rows::service::rows(tables, keys, &mut into)?;
                }
                RowsSource::Written(rows) => {
                    crate::domain::table_rows::service::written(rows, &mut into)?;
                }
            }
            Ok(output)
        }
    }
}

/// The tables a `list<table>` parameter is bound to; an empty list of another shape is no table.
fn bound_tables(
    bindings: &[ParameterBinding],
    parameter: ryo_values::domain::plan_parts::types::ParameterId,
) -> Result<&[ryo_values::domain::query_bindings::types::BindingTable], PortableOperatorFault> {
    match ryo_values::domain::plan_parts::service::parameter_in(bindings, parameter)
        .ok_or(PortableOperatorFault::Unbound)?
    {
        ParameterBinding::Tables(tables) => Ok(tables.as_slice()),
        ParameterBinding::Texts(items) if items.is_empty() => Ok(&[]),
        ParameterBinding::Numbers(items) if items.is_empty() => Ok(&[]),
        ParameterBinding::TextSets(items) if items.is_empty() => Ok(&[]),
        ParameterBinding::Number(_)
        | ParameterBinding::Flag(_)
        | ParameterBinding::Text(_)
        | ParameterBinding::Numbers(_)
        | ParameterBinding::Texts(_)
        | ParameterBinding::TextSets(_) => Err(PortableOperatorFault::WrongBinding),
    }
}

/// Writes each table's texts as one row of made names.
struct RowsInto<'r, 'k> {
    /// The run whose resources make the names.
    run: &'r mut KernelRun<'k>,
    /// The rows written.
    output: &'r mut Table,
    /// The row being written.
    row: Vec<Value>,
}

impl TableRowOutput for RowsInto<'_, '_> {
    fn row(&mut self, texts: RowTexts<'_>) -> Result<(), ContractError> {
        self.row.clear();
        for text in texts.0 {
            let made = self.run.resources.mint(*text);
            self.row.push(made);
        }
        push_row(self.output, &self.row);
        Ok(())
    }
}
