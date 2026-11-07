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

/// Interns each delivered text as derived text and emits the row.
struct RowMinting<'a> {
    driver: &'a mut dyn crate::domain::evaluation::types::InvocationDriver,
    row: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
}

impl TableRowOutput for RowMinting<'_> {
    fn row(
        &mut self,
        texts: RowTexts<'_>,
    ) -> Result<(), crate::domain::contract::errors::ContractError> {
        self.row.clear();
        for text in texts.0 {
            let made = self.driver.mint(*text);
            self.row.push(made);
        }
        self.driver
            .emit(self.row)
            .map_err(|_| crate::domain::contract::errors::ContractError::Interrupted)
    }
}

/// Emits one row of interned texts for each table of a table-list parameter; an empty list of any kind is
/// no tables.
pub(crate) fn evaluate(
    parameter: ryo_values::domain::plan_parts::types::ParameterId,
    keys: &[TableKey],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    driver: &mut dyn crate::domain::evaluation::types::InvocationDriver,
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
    rows(tables, keys, &mut RowMinting { driver, row })
}
