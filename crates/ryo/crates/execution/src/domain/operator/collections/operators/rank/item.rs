//! Append the first matching resolved text-set rank, preserving input order.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use ryo_values::domain::rows::service::cell;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct RankItem;
impl OperatorsItem for RankItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
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
        let answer = lease.semantics().operators(
            ryo_operators::inbound::use_cases::operators::interface::OperatorCall::Rank {
                text: named.0,
                layers,
            },
        );
        let rank = match answer {
            Ok(ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer::Rank(
                Some(rank),
            )) => rank,
            Ok(ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer::Rank(
                None,
            )) => return Ok(()),
            Err(error) => {
                return Err(crate::domain::operator::contract::types::portable_fault(
                    error,
                ));
            }
            _ => return Err(OperatingError::WrongOperation),
        };
        let mut joined = std::mem::take(&mut lease.workspace.row);
        joined.clear();
        joined.extend_from_slice(row);
        joined.push(Value::Number(rank));
        let result = emit.row(lease, &joined);
        lease.workspace.row = joined;
        result
    }
}
