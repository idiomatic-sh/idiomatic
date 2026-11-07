//! Tarjan cycle semantics over immutable engine-prepared adjacency.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use ryo_derived::domain::contract::types::CycleGraph;
use ryo_operators::domain::cycles::types::{CycleDriver, EdgePlace, NodeCount, NodeNumber};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_bindings::types::ParameterBinding;
/// The prepared adjacency of the derived resource, read through the operators' port.
struct Adjacency<'a>(&'a CycleGraph);
impl CycleDriver for Adjacency<'_> {
    fn nodes(&self) -> NodeCount {
        NodeCount(self.0.values().len())
    }
    fn value(&self, node: NodeNumber) -> Option<Value> {
        self.0.values().get(node.0).copied()
    }
    fn edge(&self, node: NodeNumber, place: EdgePlace) -> Option<NodeNumber> {
        let starts = self.0.starts();
        let at = starts.get(node.0)?.checked_add(place.0)?;
        (at < *starts.get(node.0 + 1)?)
            .then(|| self.0.leaving().get(at).copied().map(NodeNumber))
            .flatten()
    }
}
pub(crate) struct CyclesItem;
impl OperatorsItem for CyclesItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Cycles,
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
        if !matches!(operation, Operation::Cycles { .. })
            || !matches!(inputs, PreparedInputs::Views(_))
        {
            return Err(OperatingError::WrongOperation);
        }
        let graph = lease
            .views
            .resources
            .cycles()?
            .ok_or(OperatingError::WrongBinding)?;
        let mut marks = lease
            .workspace
            .cycles
            .take()
            .ok_or(OperatingError::WrongBinding)?;
        let answer = lease.semantics().operators(
            ryo_operators::inbound::use_cases::operators::types::OperatorCall::Cycles {
                graph: &Adjacency(graph),
                scratch: &mut marks,
            },
        );
        lease.workspace.cycles = Some(marks);
        crate::domain::operator::contract::types::written_result(answer, None)?;
        // The marks remain engine-owned; copy one scalar before the downstream reborrow.
        let count = lease
            .workspace
            .cycles
            .as_ref()
            .map_or(0, |work| work.ordered.len());
        for index in 0..count {
            let value = lease
                .workspace
                .cycles
                .as_ref()
                .and_then(|work| work.ordered.get(index))
                .copied()
                .ok_or(OperatingError::ShapeMismatch)?;
            emit.row(lease, &[value, value])?;
        }
        Ok(())
    }
}
