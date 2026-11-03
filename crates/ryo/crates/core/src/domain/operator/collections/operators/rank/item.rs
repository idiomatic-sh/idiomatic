//! Append the first matching resolved text-set rank, preserving input order.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::service::cell;
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_relations::domain::relations::types::{Integer, Value};
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct RankItem;
impl OperatorsItem for RankItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Rank,
            operation,
            inputs,
            bindings,
        )
    }
    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        emit: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let PreparedInputs::Row { row, .. } = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let Operation::Rank { column, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let layers = lease
            .views
            .resources
            .rank_layers()
            .ok_or(OperatingError::WrongBinding)?;
        let texts = Texts {
            symbols: lease.derived.as_ref(),
            source: lease.views.source,
            paths: lease.views.paths,
            arenas: lease.views.arenas,
        };
        let Datum::Text(named) = datum(cell(row, *column)?, &texts) else {
            return Ok(());
        };
        let Some(place) = layers
            .iter()
            .position(|set| set.iter().any(|each| *each.0 == *named.0))
        else {
            return Ok(());
        };
        let mut joined = std::mem::take(&mut lease.workspace.row);
        joined.clear();
        joined.extend_from_slice(row);
        joined.push(Value::Number(Integer(
            i64::try_from(place).unwrap_or(i64::MAX),
        )));
        let result = emit.row(lease, &joined);
        lease.workspace.row = joined;
        result
    }
}
