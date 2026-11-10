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

/// Emits one row of interned texts for each table of a table-list parameter; an empty list of any kind is
/// no tables.
pub(crate) fn evaluate(
    parameter: ryo_values::domain::plan_parts::types::ParameterId,
    keys: &[TableKey],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    driver: &mut dyn crate::domain::contract::types::RowDriver,
    row: &mut Vec<ryo_relations::domain::relations::types::Value>,
) -> Result<(), PortableOperatorFault> {
    use ryo_values::domain::query_bindings::types::ParameterBinding;
    let tables = match ryo_values::domain::plan_parts::service::parameter_in(bindings, parameter)
        .ok_or(PortableOperatorFault::Unbound)?
    {
        ParameterBinding::Tables(tables) => tables.as_slice(),
        ParameterBinding::Texts(items) if items.is_empty() => &[],
        ParameterBinding::Numbers(items) if items.is_empty() => &[],
        ParameterBinding::TextSets(items) if items.is_empty() => &[],
        ParameterBinding::Number(_)
        | ParameterBinding::Flag(_)
        | ParameterBinding::Text(_)
        | ParameterBinding::Numbers(_)
        | ParameterBinding::Texts(_)
        | ParameterBinding::TextSets(_) => return Err(PortableOperatorFault::WrongBinding),
    };
    rows(
        tables,
        keys,
        &mut crate::domain::table_rows::types::RowMinting { driver, row },
    )
}
