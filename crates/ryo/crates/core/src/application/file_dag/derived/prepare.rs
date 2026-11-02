//! Standalone application preparation uses the same required declarations as request preparation.
use crate::domain::{
    operating::errors::OperatingError,
    operation_schema::types::InputSchema,
    operator::{
        collections::operators::interface::OperatorsItem, contract::types::OperatorContract,
    },
    table_model::types::ParameterBinding,
};
use ryo_values::domain::plans::types::Operation;

#[cfg(test)]
pub(crate) fn operator<T: OperatorsItem + ?Sized>(
    item: &T,
    operation: &Operation,
    schemas: &[InputSchema<'_>],
    bindings: &[ParameterBinding],
) -> Result<OperatorContract, OperatingError> {
    let mut contract = item.prepare(operation, schemas, bindings)?;
    super::request::RequestViews::default().bind(&mut contract)?;
    Ok(contract)
}
