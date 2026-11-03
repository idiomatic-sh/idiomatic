//! Reachability semantics over ordered, engine-prepared key partitions.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_reach::service::reach_all;
use crate::domain::table_reach::types::Edges;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct ClosureItem;
impl OperatorsItem for ClosureItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Closure,
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
        if !matches!(operation, Operation::Closure { .. })
            || !matches!(inputs, PreparedInputs::Views(_))
        {
            return Err(OperatingError::WrongOperation);
        }
        let partitions = lease.views.closure;
        let mut joined = std::mem::take(lease.workspace.row);
        let result = (|| {
            for partition in partitions {
                let partition = partition?;
                let from = [ColumnIndex(0)];
                let edges = Edges {
                    table: partition.table(),
                    order: partition.index(),
                    from: &from,
                    to: ColumnIndex(1),
                };
                for start in partition.sources() {
                    let walk = lease
                        .workspace
                        .closure_walk
                        .as_deref_mut()
                        .ok_or(OperatingError::WrongBinding)?;
                    reach_all(&edges, *start, walk);
                    let count = walk.reached.len();
                    for index in 0..count {
                        let reached = lease
                            .workspace
                            .closure_walk
                            .as_deref()
                            .and_then(|walk| walk.reached.get(index))
                            .copied()
                            .ok_or(OperatingError::ShapeMismatch)?;
                        joined.clear();
                        joined.extend_from_slice(partition.key());
                        joined.extend_from_slice(&[*start, reached]);
                        emit.row(lease, &joined)?;
                    }
                }
            }
            Ok(())
        })();
        *lease.workspace.row = joined;
        result
    }
}
