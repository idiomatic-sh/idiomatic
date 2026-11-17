//! One row for each table, a text for each key, in key order.
use super::types::{RowTexts, TableRowOutput};
use crate::domain::contract::types::PortableOperatorFault;
use ryo_values::domain::plan_parts::types::TableKey;
use ryo_values::domain::query_bindings::types::BindingTable;
use ryo_values::domain::row_texts::types::CellText;

/// Delivers, for each table, the texts under `keys`; a table lacking a key is an unbound key, before its row.
pub(crate) fn rows(
    tables: &[BindingTable],
    keys: &[TableKey],
    each: &mut dyn TableRowOutput,
) -> Result<(), PortableOperatorFault> {
    for table in tables {
        let mut texts = Vec::with_capacity(keys.len());
        for key in keys {
            let cell = table
                .0
                .iter()
                .find(|cell| *cell.key.0 == *key.0)
                .ok_or(PortableOperatorFault::UnboundKey)?;
            texts.push(CellText(&cell.value.0));
        }
        each.row(RowTexts(&texts))
            .map_err(|_| PortableOperatorFault::Interrupted)?;
    }
    Ok(())
}
